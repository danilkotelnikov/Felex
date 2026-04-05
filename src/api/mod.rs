//! API handlers module

pub mod agent;
pub mod animals;
pub mod app;
pub mod feeds;
pub mod norm_resolution;
pub mod norms;
pub mod prices;
pub mod rations;
pub mod workspace;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            error: "internal_error".to_string(),
            message: err.to_string(),
        }
    }
}

/// Health check handler for frontend readiness polling
pub async fn health_check() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(serde_json::json!({ "status": "ok" }))
}

/// API success response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(data: T) -> Json<Self> {
        Json(Self { data })
    }
}

/// Paginated response
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
