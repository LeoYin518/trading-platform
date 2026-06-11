use crate::errors::AppError;
use crate::modules::ledger::repository as ledger_repository;
use crate::modules::orders::model::dto::CreateOrderRequest;
use crate::modules::orders::model::{Order, OrderStatus};
use crate::modules::orders::{policy, repository};
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
        policy::validate_create_order_request(&request)?;
        let amounts = policy::calculate_order_amounts(request.amount)?;

        let mut tx = self.pool.begin().await?;
        let client = users_repository::find_user_by_id_for_update(&mut tx, request.client_id)
            .await?
            .ok_or_else(|| AppError::NotFound("需求方用户不存在".to_string()))?;
        let worker = users_repository::find_user_by_id_for_update(&mut tx, request.worker_id)
            .await?
            .ok_or_else(|| AppError::NotFound("服务方用户不存在".to_string()))?;

        policy::ensure_client_role(client.role)?;
        policy::ensure_worker_role(worker.role)?;
        policy::ensure_sufficient_available_balance(client.balance, request.amount)?;

        let order =
            repository::create_order(&mut tx, &request, amounts.fee_amount, amounts.worker_amount)
                .await?;
        tx.commit().await?;

        Ok(order)
    }

    pub async fn update_order_status(
        &self,
        order_id: i64,
        target_status: String,
        operator_id: i64,
    ) -> Result<Order, AppError> {
        let target_status = policy::parse_target_status(&target_status)?;

        let mut tx = self.pool.begin().await?;
        let order = repository::find_order_by_id_for_update(&mut tx, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("订单不存在".to_string()))?;

        // 状态机校验必须在事务内基于锁定后的订单执行，避免并发请求读到旧状态后重复流转。
        policy::validate_status_transition(order.status, target_status)?;

        match (order.status, target_status) {
            (OrderStatus::Pending, OrderStatus::Accepted) => {
                policy::ensure_worker_operator(operator_id, order.worker_id)?;

                let client = users_repository::find_user_by_id_for_update(&mut tx, order.client_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("需求方用户不存在".to_string()))?;

                policy::ensure_sufficient_available_balance(client.balance, order.amount)?;

                // 资金冻结和订单状态更新处于同一事务，保证不会出现已接单但资金未冻结的中间状态。
                users_repository::transfer_available_to_frozen(
                    &mut tx,
                    order.client_id,
                    order.amount,
                )
                .await?;
                ledger_repository::insert_freeze_entries(
                    &mut tx,
                    order.id,
                    order.client_id,
                    operator_id,
                    order.amount,
                )
                .await?;
            }
            (OrderStatus::Accepted, OrderStatus::Completed) => {
                policy::ensure_client_operator(operator_id, order.client_id)?;

                let client = users_repository::find_user_by_id_for_update(&mut tx, order.client_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("需求方用户不存在".to_string()))?;
                let _worker =
                    users_repository::find_user_by_id_for_update(&mut tx, order.worker_id)
                        .await?
                        .ok_or_else(|| AppError::NotFound("服务方用户不存在".to_string()))?;

                policy::ensure_sufficient_frozen_balance(client.frozen_balance, order.amount)?;

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
                ledger_repository::insert_worker_settlement_entries(
                    &mut tx,
                    order.id,
                    order.client_id,
                    order.worker_id,
                    operator_id,
                    order.worker_amount,
                )
                .await?;
                ledger_repository::insert_platform_fee_entries(
                    &mut tx,
                    order.id,
                    order.client_id,
                    operator_id,
                    order.fee_amount,
                )
                .await?;
            }
            (OrderStatus::Pending, OrderStatus::Cancelled) => {
                policy::ensure_pending_cancel_operator(
                    operator_id,
                    order.client_id,
                    order.worker_id,
                )?;
            }
            _ => unreachable!("状态流转已在 validate_status_transition 中拦截"),
        }

        let updated_order =
            repository::update_order_status(&mut tx, order_id, target_status).await?;
        tx.commit().await?;

        Ok(updated_order)
    }
}
