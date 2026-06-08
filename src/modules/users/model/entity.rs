use crate::modules::users::model::UserRole;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub role: UserRole,
    pub balance: i64,
    pub frozen_balance: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
