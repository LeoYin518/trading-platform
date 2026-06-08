use crate::modules::orders::model::OrderStatus;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Order {
    pub id: i64,
    pub client_id: i64,
    pub worker_id: i64,
    pub amount: i64,
    pub fee_amount: i64,
    pub worker_amount: i64,
    pub description: String,
    pub status: OrderStatus,
    pub accepted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
