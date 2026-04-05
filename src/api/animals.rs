//! Animals API handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use super::{ApiError, ApiResponse};
use crate::db::animals::{self, AnimalGroup};
use crate::AppState;

/// Query parameters for listing animals
#[derive(Debug, Deserialize)]
pub struct ListAnimalsQuery {
    pub species: Option<String>,
}

/// List all animal groups
pub async fn list_animals(
    State(state): State<AppState>,
    Query(query): Query<ListAnimalsQuery>,
) -> Result<Json<ApiResponse<Vec<AnimalGroup>>>, ApiError> {
    let groups = state.db.with_conn(|conn| match query.species {
        Some(ref s) => animals::list_by_species(conn, s),
        None => animals::list_animal_groups(conn),
    })?;

    Ok(ApiResponse::new(groups))
}

/// Get a single animal group
pub async fn get_animal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<AnimalGroup>>, (StatusCode, Json<ApiError>)> {
    let group = state
        .db
        .with_conn(|conn| animals::get_animal_group(conn, &id))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::from(e))))?;

    match group {
        Some(g) => Ok(ApiResponse::new(g)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "not_found".to_string(),
                message: format!("Animal group {} not found", id),
            }),
        )),
    }
}

/// Create animal group request
#[derive(Debug, Deserialize)]
pub struct CreateAnimalRequest {
    pub id: String,
    pub species: String,
    pub production_type: Option<String>,
    pub name_ru: String,
    pub name_en: Option<String>,
    pub description: Option<String>,
}

/// Create a new animal group
pub async fn create_animal(
    State(state): State<AppState>,
    Json(req): Json<CreateAnimalRequest>,
) -> Result<(StatusCode, Json<ApiResponse<String>>), ApiError> {
    let group = AnimalGroup {
        id: req.id.clone(),
        species: req.species,
        production_type: req.production_type,
        name_ru: req.name_ru,
        name_en: req.name_en,
        description: req.description,
        created_at: None,
    };

    state
        .db
        .with_conn(|conn| animals::create_animal_group(conn, &group))?;

    Ok((StatusCode::CREATED, ApiResponse::new(req.id)))
}
