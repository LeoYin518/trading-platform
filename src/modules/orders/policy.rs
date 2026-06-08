use crate::errors::AppError;
use crate::modules::orders::model::OrderStatus;
use crate::modules::orders::model::dto::CreateOrderRequest;
use crate::modules::users::model::UserRole;

const PLATFORM_FEE_DIVISOR: i64 = 10;

pub(super) struct OrderAmountBreakdown {
    pub fee_amount: i64,
    pub worker_amount: i64,
}

pub(super) fn validate_create_order_request(request: &CreateOrderRequest) -> Result<(), AppError> {
    calculate_order_amounts(request.amount)?;
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

pub(super) fn ensure_client_role(role: UserRole) -> Result<(), AppError> {
    if role != UserRole::Client {
        return Err(AppError::Forbidden(
            "client_id必须对应需求方用户".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn ensure_worker_role(role: UserRole) -> Result<(), AppError> {
    if role != UserRole::Worker {
        return Err(AppError::Forbidden(
            "worker_id必须对应服务方用户".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn ensure_sufficient_available_balance(
    balance: i64,
    amount: i64,
) -> Result<(), AppError> {
    if balance < amount {
        return Err(AppError::Conflict("需求方可用余额不足".to_string()));
    }

    Ok(())
}

pub(super) fn ensure_sufficient_frozen_balance(
    frozen_balance: i64,
    amount: i64,
) -> Result<(), AppError> {
    if frozen_balance < amount {
        return Err(AppError::Conflict("需求方冻结余额不足".to_string()));
    }

    Ok(())
}

pub(super) fn ensure_worker_operator(operator_id: i64, worker_id: i64) -> Result<(), AppError> {
    if operator_id != worker_id {
        return Err(AppError::Forbidden(
            "只有订单指定服务方可以接单".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn ensure_client_operator(operator_id: i64, client_id: i64) -> Result<(), AppError> {
    if operator_id != client_id {
        return Err(AppError::Forbidden(
            "只有订单需求方可以确认完成".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn ensure_pending_cancel_operator(
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

pub(super) fn calculate_order_amounts(amount: i64) -> Result<OrderAmountBreakdown, AppError> {
    if amount <= 0 {
        return Err(AppError::BadRequest("订单金额必须大于0".to_string()));
    }

    let fee_amount = amount / PLATFORM_FEE_DIVISOR;

    Ok(OrderAmountBreakdown {
        fee_amount,
        worker_amount: amount - fee_amount,
    })
}

pub(super) fn parse_target_status(target_status: &str) -> Result<OrderStatus, AppError> {
    target_status.parse().map_err(|_| {
        AppError::BadRequest(
            "target_status 状态字段无效，只能是 Pending、Accepted、Completed、Cancelled"
                .to_string(),
        )
    })
}

pub(super) fn validate_status_transition(
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
    fn calculate_order_amounts_rounds_fee_down() {
        let small_order = calculate_order_amounts(333).unwrap();
        assert_eq!(small_order.fee_amount, 33);
        assert_eq!(small_order.worker_amount, 300);

        let regular_order = calculate_order_amounts(10_000).unwrap();
        assert_eq!(regular_order.fee_amount, 1_000);
        assert_eq!(regular_order.worker_amount, 9_000);
    }

    #[test]
    fn calculate_order_amounts_rejects_non_positive_amount() {
        assert!(calculate_order_amounts(0).is_err());
        assert!(calculate_order_amounts(-1).is_err());
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
    fn rejects_invalid_target_status_value() {
        let error = parse_target_status("abc").unwrap_err();

        match error {
            AppError::BadRequest(message) => {
                assert!(message.contains("target_status"));
                assert!(message.contains("Pending"));
                assert!(message.contains("Accepted"));
                assert!(message.contains("Completed"));
                assert!(message.contains("Cancelled"));
            }
            _ => panic!("expected bad request for invalid target_status"),
        }
    }

    #[test]
    fn rejects_invalid_user_roles_for_order_creation() {
        assert!(ensure_client_role(UserRole::Worker).is_err());
        assert!(ensure_worker_role(UserRole::Client).is_err());
    }

    #[test]
    fn accepts_required_user_roles_for_order_creation() {
        assert!(ensure_client_role(UserRole::Client).is_ok());
        assert!(ensure_worker_role(UserRole::Worker).is_ok());
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
