use crate::common::ok;
use crate::errors::AppError;
use crate::modules::orders::model::dto::{CreateOrderRequest, OrderResponse, UpdateOrderStatusRequest};
use crate::router::AppState;
use axum::{Json, extract::Path, extract::State, response::IntoResponse};

pub async fn create_order(
    State(state): State<AppState>,
    Json(request): Json<CreateOrderRequest>,
) -> Result<impl IntoResponse, AppError> {
    let order = state.order_service.create_order(request).await?;
    Ok(ok(OrderResponse::from(order)))
}

pub async fn update_order_status(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    Json(request): Json<UpdateOrderStatusRequest>,
) -> Result<impl IntoResponse, AppError> {
    let order = state
        .order_service
        .update_order_status(order_id, request.target_status, request.operator_id)
        .await?;
    Ok(ok(OrderResponse::from(order)))
}
