use crate::modules::orders::model::{Order, OrderStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub client_id: i64,
    pub worker_id: i64,
    pub amount: i64,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrderStatusRequest {
    pub target_status: String,
    pub operator_id: i64,
}

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: i64,
    pub client_id: i64,
    pub worker_id: i64,
    pub amount: i64,
    pub fee_amount: i64,
    pub worker_amount: i64,
    pub description: String,
    pub status: OrderStatus,
}

impl From<Order> for OrderResponse {
    fn from(order: Order) -> Self {
        Self {
            id: order.id,
            client_id: order.client_id,
            worker_id: order.worker_id,
            amount: order.amount,
            fee_amount: order.fee_amount,
            worker_amount: order.worker_amount,
            description: order.description,
            status: order.status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_order_status_request_accepts_raw_target_status_for_service_validation() {
        let request: UpdateOrderStatusRequest =
            serde_json::from_str(r#"{"target_status":"abc","operator_id":1}"#).unwrap();

        assert_eq!(request.target_status, "abc");
    }
}
