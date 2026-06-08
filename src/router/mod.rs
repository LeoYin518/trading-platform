use crate::modules::orders::{self, OrderService};
use axum::{Router, routing::get};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub order_service: OrderService,
}

pub fn build_router(pool: PgPool) -> Router {
    let state = AppState {
        order_service: OrderService::new(pool),
    };

    Router::new()
        .route("/health", get(health))
        .nest("/orders", orders::routes())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn health() -> &'static str {
    "Your service is up and running successfully!"
}
