use crate::errors::AppError;
use crate::modules::orders::model::dto::CreateOrderRequest;
use crate::modules::orders::model::{Order, OrderStatus};
use crate::modules::orders::repository;
use crate::modules::users::model::UserRole;
use crate::modules::users::repository as users_repository;
use sqlx::PgPool;

#[derive(Clone)]
pub struct OrderService {
    pool: PgPool,
}

impl OrderService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_order(&self, request: CreateOrderRequest) -> Result<Order, AppError> {
        validate_create_order_request(&request)?;

        let mut tx = self.pool.begin().await?;
        let client = users_repository::find_user_by_id_for_update(&mut tx, request.client_id)
            .await?
            .ok_or_else(|| AppError::NotFound("需求方用户不存在".to_string()))?;
        let worker = users_repository::find_user_by_id_for_update(&mut tx, request.worker_id)
            .await?
            .ok_or_else(|| AppError::NotFound("服务方用户不存在".to_string()))?;

        if client.role != UserRole::Client {
            return Err(AppError::Forbidden(
                "client_id必须对应需求方用户".to_string(),
            ));
        }
        if worker.role != UserRole::Worker {
            return Err(AppError::Forbidden(
                "worker_id必须对应服务方用户".to_string(),
            ));
        }
        ensure_sufficient_available_balance(client.balance, request.amount)?;

        let order = repository::create_order(&mut tx, &request).await?;
        tx.commit().await?;

        Ok(order)
    }

    pub async fn update_order_status(
        &self,
        order_id: i64,
        target_status: OrderStatus,
        operator_id: i64,
    ) -> Result<Order, AppError> {
        let mut tx = self.pool.begin().await?;
        let order = repository::find_order_by_id_for_update(&mut tx, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("订单不存在".to_string()))?;

        // 状态机校验必须在事务内基于锁定后的订单执行，避免并发请求读到旧状态后重复流转。
        validate_status_transition(order.status, target_status)?;

        match (order.status, target_status) {
            (OrderStatus::Pending, OrderStatus::Accepted) => {
                ensure_worker_operator(operator_id, order.worker_id)?;

                let client = users_repository::find_user_by_id_for_update(&mut tx, order.client_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("需求方用户不存在".to_string()))?;
                ensure_sufficient_available_balance(client.balance, order.amount)?;

                // 资金冻结和订单状态更新处于同一事务，保证不会出现已接单但资金未冻结的中间状态。
                users_repository::transfer_available_to_frozen(
                    &mut tx,
                    order.client_id,
                    order.amount,
                )
                .await?;
            }
            (OrderStatus::Accepted, OrderStatus::Completed) => {
                ensure_client_operator(operator_id, order.client_id)?;

                let client = users_repository::find_user_by_id_for_update(&mut tx, order.client_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("需求方用户不存在".to_string()))?;
                let _worker =
                    users_repository::find_user_by_id_for_update(&mut tx, order.worker_id)
                        .await?
                        .ok_or_else(|| AppError::NotFound("服务方用户不存在".to_string()))?;
                ensure_sufficient_frozen_balance(client.frozen_balance, order.amount)?;

                // 结算顺序放在同一事务中：扣减冻结金额、服务方入账、记录平台手续费、更新订单完成状态。
                users_repository::decrease_frozen_balance(&mut tx, order.client_id, order.amount)
                    .await?;
                users_repository::increase_available_balance(
                    &mut tx,
                    order.worker_id,
                    order.worker_amount,
                )
                .await?;
                repository::insert_platform_fee_record(&mut tx, order.id, order.fee_amount).await?;
            }
            (OrderStatus::Pending, OrderStatus::Cancelled) => {
                ensure_pending_cancel_operator(operator_id, order.client_id, order.worker_id)?;
            }
            _ => unreachable!("状态流转已在validate_status_transition中拦截"),
        }

        let updated_order =
            repository::update_order_status(&mut tx, order_id, target_status).await?;
        tx.commit().await?;

        Ok(updated_order)
    }
}

fn validate_create_order_request(request: &CreateOrderRequest) -> Result<(), AppError> {
    calculate_fee(request.amount)?;
    if request.client_id == request.worker_id {
        return Err(AppError::BadRequest(
            "需求方和服务方不能是同一个用户".to_string(),
        ));
    }
    if request.description.trim().is_empty() {
        return Err(AppError::BadRequest("需求描述不能为空".to_string()));
    }

    Ok(())
}

fn ensure_sufficient_available_balance(balance: i64, amount: i64) -> Result<(), AppError> {
    if balance < amount {
        return Err(AppError::Conflict("需求方可用余额不足".to_string()));
    }

    Ok(())
}

fn ensure_sufficient_frozen_balance(frozen_balance: i64, amount: i64) -> Result<(), AppError> {
    if frozen_balance < amount {
        return Err(AppError::Conflict("需求方冻结余额不足".to_string()));
    }

    Ok(())
}

fn ensure_worker_operator(operator_id: i64, worker_id: i64) -> Result<(), AppError> {
    if operator_id != worker_id {
        return Err(AppError::Forbidden(
            "只有订单指定服务方可以接单".to_string(),
        ));
    }

    Ok(())
}

fn ensure_client_operator(operator_id: i64, client_id: i64) -> Result<(), AppError> {
    if operator_id != client_id {
        return Err(AppError::Forbidden(
            "只有订单需求方可以确认完成".to_string(),
        ));
    }

    Ok(())
}

fn ensure_pending_cancel_operator(
    operator_id: i64,
    client_id: i64,
    worker_id: i64,
) -> Result<(), AppError> {
    if operator_id != client_id && operator_id != worker_id {
        return Err(AppError::Forbidden(
            "只有订单需求方或服务方可以取消待接单订单".to_string(),
        ));
    }

    Ok(())
}

pub fn calculate_fee(amount: i64) -> Result<i64, AppError> {
    if amount <= 0 {
        return Err(AppError::BadRequest("订单金额必须大于0".to_string()));
    }

    Ok(amount / 10)
}

pub fn validate_status_transition(
    current_status: OrderStatus,
    target_status: OrderStatus,
) -> Result<(), AppError> {
    match (current_status, target_status) {
        (OrderStatus::Pending, OrderStatus::Accepted)
        | (OrderStatus::Pending, OrderStatus::Cancelled)
        | (OrderStatus::Accepted, OrderStatus::Completed) => Ok(()),
        _ => Err(AppError::Conflict(format!(
            "订单状态不能从{current_status}流转到{target_status}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_fee_rounds_down() {
        assert_eq!(calculate_fee(333).unwrap(), 33);
        assert_eq!(calculate_fee(10_000).unwrap(), 1_000);
    }

    #[test]
    fn calculate_fee_rejects_non_positive_amount() {
        assert!(calculate_fee(0).is_err());
        assert!(calculate_fee(-1).is_err());
    }

    #[test]
    fn allows_only_required_status_transitions() {
        assert!(validate_status_transition(OrderStatus::Pending, OrderStatus::Accepted).is_ok());
        assert!(validate_status_transition(OrderStatus::Pending, OrderStatus::Cancelled).is_ok());
        assert!(validate_status_transition(OrderStatus::Accepted, OrderStatus::Completed).is_ok());
    }

    #[test]
    fn rejects_cancel_after_accepted() {
        assert!(validate_status_transition(OrderStatus::Accepted, OrderStatus::Cancelled).is_err());
    }

    #[test]
    fn rejects_undefined_status_transitions() {
        assert!(
            validate_status_transition(OrderStatus::Completed, OrderStatus::Cancelled).is_err()
        );
        assert!(validate_status_transition(OrderStatus::Cancelled, OrderStatus::Accepted).is_err());
    }

    #[test]
    fn rejects_insufficient_available_balance_for_order_creation_or_acceptance() {
        assert!(ensure_sufficient_available_balance(99, 100).is_err());
        assert!(ensure_sufficient_available_balance(100, 100).is_ok());
    }

    #[test]
    fn rejects_insufficient_frozen_balance() {
        assert!(ensure_sufficient_frozen_balance(99, 100).is_err());
        assert!(ensure_sufficient_frozen_balance(100, 100).is_ok());
    }

    #[test]
    fn only_order_worker_can_accept_order() {
        assert!(ensure_worker_operator(3, 3).is_ok());
        assert!(ensure_worker_operator(1, 3).is_err());
    }

    #[test]
    fn only_order_client_can_complete_order() {
        assert!(ensure_client_operator(1, 1).is_ok());
        assert!(ensure_client_operator(3, 1).is_err());
    }

    #[test]
    fn only_order_participants_can_cancel_pending_order() {
        assert!(ensure_pending_cancel_operator(1, 1, 3).is_ok());
        assert!(ensure_pending_cancel_operator(3, 1, 3).is_ok());
        assert!(ensure_pending_cancel_operator(4, 1, 3).is_err());
    }
}
