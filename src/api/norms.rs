//! Norms API handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;

use super::{ApiError, ApiResponse};
use crate::api::norm_resolution::animal_context_from_properties;
use crate::api::norm_resolution::{
    resolved_default_norm, resolved_norm_group_id, NormResolveRequest,
};
use crate::norms::{self, AnimalNorm, NormMethodology};
use crate::AppState;

/// Get norms for an animal group
pub async fn get_norms(
    State(_state): State<AppState>,
    Path(animal_group_id): Path<String>,
) -> Result<Json<ApiResponse<AnimalNorm>>, (StatusCode, Json<ApiError>)> {
    let norm = norms::get_norms_for_group(&animal_group_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "not_found".to_string(),
                message: format!("Norms for group {} not found", animal_group_id),
            }),
        )
    })?;

    Ok(ApiResponse::new(norm))
}

#[derive(Debug, Serialize)]
pub struct ResolvedNormResponse {
    pub resolved_group_id: String,
    pub norm: AnimalNorm,
    pub methodology: Option<NormMethodology>,
}

pub async fn resolve_norms(
    State(_state): State<AppState>,
    Path(animal_group_id): Path<String>,
    Json(req): Json<NormResolveRequest>,
) -> Result<Json<ApiResponse<ResolvedNormResponse>>, (StatusCode, Json<ApiError>)> {
    let resolved_group_id = resolved_norm_group_id(Some(animal_group_id.as_str()), &req);
    let norm = resolved_default_norm(
        Some(animal_group_id.as_str()),
        &req,
        Some(resolved_group_id.as_str()),
    )
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "not_found".to_string(),
                message: format!("Norms for group {} not found", animal_group_id),
            }),
        )
    })?;
    let context = animal_context_from_properties(req.animal_properties.as_ref());
    let methodology =
        norms::describe_norm_methodology(resolved_group_id.as_str(), context.as_ref());

    Ok(ApiResponse::new(ResolvedNormResponse {
        resolved_group_id,
        norm,
        methodology,
    }))
}
