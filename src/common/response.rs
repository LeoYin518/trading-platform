use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        Self {
            code: StatusCode::OK.as_u16(),
            message: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn error(status: StatusCode, message: String) -> ApiResponse<()> {
        ApiResponse {
            code: status.as_u16(),
            message,
            data: None,
        }
    }
}

pub fn ok<T>(data: T) -> impl IntoResponse
where
    T: Serialize,
{
    (StatusCode::OK, Json(ApiResponse::success(data)))
}
