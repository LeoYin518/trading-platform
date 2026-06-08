pub mod handler;
pub mod model;
pub mod repository;
pub mod service;

use crate::router::AppState;
use axum::{
    Router,
    routing::{patch, post},
};

pub use self::service::OrderService;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handler::create_order))
        .route("/{id}/status", patch(handler::update_order_status))
}
