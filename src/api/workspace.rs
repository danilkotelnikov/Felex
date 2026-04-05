//! Workspace filesystem API handlers
//!
//! Manages a workspace directory containing `.felex.json` ration project files.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::{ApiError, ApiResponse};
use crate::AppState;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub name: String,
    /// Relative path from workspace root (forward-slash separated)
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
    pub animal_group: Option<String>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RationProject {
    pub version: String,
    pub name: String,
    pub animal_group_id: String,
    pub animal_properties: serde_json::Value,
    pub animal_count: u32,
    pub items: Vec<RationProjectItem>,
    pub norm_preset_id: Option<String>,
    pub custom_norms: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RationProjectItem {
    pub feed_id: i64,
    pub feed_name: String,
    pub amount_kg: f64,
    pub is_locked: bool,
}

// ---------------------------------------------------------------------------
// Request / query structs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct FolderRequest {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct RationWriteRequest {
    pub path: String,
    pub project: RationProject,
}

#[derive(Debug, Deserialize)]
pub struct RationQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigUpdateRequest {
    pub workspace_root: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub workspace_root: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn api_err(status: StatusCode, msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error: status.canonical_reason().unwrap_or("error").to_string(),
            message: msg.into(),
        }),
    )
}

/// Resolve a user-supplied relative path against `workspace_root` and ensure
/// the result stays inside the workspace (no path-traversal via `..`).
fn safe_resolve(
    workspace_root: &str,
    relative: &str,
) -> Result<PathBuf, (StatusCode, Json<ApiError>)> {
    // Reject obvious traversal attempts early
    let normalized = relative.replace('\\', "/");
    if normalized.split('/').any(|seg| seg == "..") {
        return Err(api_err(
            StatusCode::BAD_REQUEST,
            "Path must not contain '..'",
        ));
    }

    let root = PathBuf::from(workspace_root);
    let target = root.join(&normalized);

    // After joining, double-check the path stays inside root.
    // We use starts_with on the logical (non-canonicalized) path first,
    // then also check the canonical form if the file/dir already exists.
    if !target.starts_with(&root) {
        return Err(api_err(
            StatusCode::BAD_REQUEST,
            "Path escapes workspace root",
        ));
    }

    // If the file already exists we can canonicalize for a stronger check
    if target.exists() {
        let canon_target = target.canonicalize().map_err(|e| {
            api_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("canonicalize target: {e}"),
            )
        })?;
        // Root might not exist yet; create it lazily later. If it does exist, check.
        if root.exists() {
            let canon_root = root.canonicalize().map_err(|e| {
                api_err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("canonicalize root: {e}"),
                )
            })?;
            if !canon_target.starts_with(&canon_root) {
                return Err(api_err(
                    StatusCode::BAD_REQUEST,
                    "Resolved path escapes workspace root",
                ));
            }
        }
    }

    Ok(target)
}

/// Convert a path to a relative, forward-slash-separated string from `root`.
fn to_relative(root: &Path, full: &Path) -> String {
    full.strip_prefix(root)
        .unwrap_or(full)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Recursively scan a directory, returning `.felex.json` files and all
/// directories so empty folders remain visible in the workspace tree.
fn scan_tree(root: &Path, dir: &Path) -> std::io::Result<Vec<FileNode>> {
    let mut nodes = Vec::new();

    let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let ft = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().to_string();

        if ft.is_dir() {
            let children = scan_tree(root, &entry.path())?;
            nodes.push(FileNode {
                name,
                path: to_relative(root, &entry.path()),
                is_dir: true,
                children: Some(children),
                animal_group: None,
                modified_at: None,
            });
        } else if ft.is_file() && name.ends_with(".felex.json") {
            let modified_at = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Utc> = t.into();
                    dt.to_rfc3339()
                });

            // Try extracting animal_group_id from file
            let animal_group = std::fs::read_to_string(entry.path())
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.get("animal_group_id")?.as_str().map(String::from));

            nodes.push(FileNode {
                name,
                path: to_relative(root, &entry.path()),
                is_dir: false,
                children: None,
                animal_group,
                modified_at,
            });
        }
    }

    Ok(nodes)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /workspace/tree — recursively scan workspace, return tree of FileNode
pub async fn get_tree(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<FileNode>>>, (StatusCode, Json<ApiError>)> {
    let config = state.config.read().await;
    let root = PathBuf::from(&config.workspace_root);
    drop(config);

    // Ensure workspace root exists
    if !root.exists() {
        std::fs::create_dir_all(&root).map_err(|e| {
            api_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Cannot create workspace root: {e}"),
            )
        })?;
    }

    let tree = scan_tree(&root, &root).map_err(|e| {
        api_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to scan workspace: {e}"),
        )
    })?;

    Ok(ApiResponse::new(tree))
}

/// POST /workspace/folder — create directory relative to workspace root
pub async fn create_folder(
    State(state): State<AppState>,
    Json(body): Json<FolderRequest>,
) -> Result<(StatusCode, Json<ApiResponse<FileNode>>), (StatusCode, Json<ApiError>)> {
    let config = state.config.read().await;
    let workspace_root = config.workspace_root.clone();
    drop(config);

    let target = safe_resolve(&workspace_root, &body.path)?;

    std::fs::create_dir_all(&target).map_err(|e| {
        api_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create directory: {e}"),
        )
    })?;

    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok((
        StatusCode::CREATED,
        ApiResponse::new(FileNode {
            name,
            path: body.path,
            is_dir: true,
            children: Some(Vec::new()),
            animal_group: None,
            modified_at: None,
        }),
    ))
}

/// POST /workspace/ration — create a new .felex.json file
pub async fn create_ration(
    State(state): State<AppState>,
    Json(body): Json<RationWriteRequest>,
) -> Result<(StatusCode, Json<ApiResponse<RationProject>>), (StatusCode, Json<ApiError>)> {
    if !body.path.ends_with(".felex.json") {
        return Err(api_err(
            StatusCode::BAD_REQUEST,
            "Path must end with .felex.json",
        ));
    }

    let config = state.config.read().await;
    let workspace_root = config.workspace_root.clone();
    drop(config);

    let target = safe_resolve(&workspace_root, &body.path)?;

    if target.exists() {
        return Err(api_err(
            StatusCode::CONFLICT,
            "File already exists; use PUT to update",
        ));
    }

    // Ensure parent directory exists
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            api_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create parent dirs: {e}"),
            )
        })?;
    }

    let json = serde_json::to_string_pretty(&body.project).map_err(|e| {
        api_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Serialize error: {e}"),
        )
    })?;

    std::fs::write(&target, json).map_err(|e| {
        api_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write file: {e}"),
        )
    })?;

    Ok((StatusCode::CREATED, ApiResponse::new(body.project)))
}

/// GET /workspace/ration?path=... — read and return RationProject from file
pub async fn get_ration(
    State(state): State<AppState>,
    Query(query): Query<RationQuery>,
) -> Result<Json<ApiResponse<RationProject>>, (StatusCode, Json<ApiError>)> {
    let config = state.config.read().await;
    let workspace_root = config.workspace_root.clone();
    drop(config);

    let target = safe_resolve(&workspace_root, &query.path)?;

    if !target.exists() {
        return Err(api_err(StatusCode::NOT_FOUND, "File not found"));
    }

    let contents = std::fs::read_to_string(&target).map_err(|e| {
        api_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Read error: {e}"),
        )
    })?;

    let project: RationProject = serde_json::from_str(&contents).map_err(|e| {
        api_err(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("Invalid .felex.json: {e}"),
        )
    })?;

    Ok(ApiResponse::new(project))
}

/// PUT /workspace/ration — overwrite existing .felex.json, update `updated_at`
pub async fn update_ration(
    State(state): State<AppState>,
    Json(mut body): Json<RationWriteRequest>,
) -> Result<Json<ApiResponse<RationProject>>, (StatusCode, Json<ApiError>)> {
    if !body.path.ends_with(".felex.json") {
        return Err(api_err(
            StatusCode::BAD_REQUEST,
            "Path must end with .felex.json",
        ));
    }

    let config = state.config.read().await;
    let workspace_root = config.workspace_root.clone();
    drop(config);

    let target = safe_resolve(&workspace_root, &body.path)?;

    if !target.exists() {
        return Err(api_err(
            StatusCode::NOT_FOUND,
            "File not found; use POST to create",
        ));
    }

    // Update the timestamp
    body.project.updated_at = chrono::Utc::now().to_rfc3339();

    let json = serde_json::to_string_pretty(&body.project).map_err(|e| {
        api_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Serialize error: {e}"),
        )
    })?;

    std::fs::write(&target, json).map_err(|e| {
        api_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write file: {e}"),
        )
    })?;

    Ok(ApiResponse::new(body.project))
}

/// DELETE /workspace/ration?path=... — delete file or empty directory
pub async fn delete_ration(
    State(state): State<AppState>,
    Query(query): Query<RationQuery>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let config = state.config.read().await;
    let workspace_root = config.workspace_root.clone();
    drop(config);

    let target = safe_resolve(&workspace_root, &query.path)?;

    if !target.exists() {
        return Err(api_err(StatusCode::NOT_FOUND, "Path not found"));
    }

    if target.is_dir() {
        std::fs::remove_dir(&target).map_err(|e| {
            api_err(
                StatusCode::CONFLICT,
                format!("Cannot remove directory (not empty?): {e}"),
            )
        })?;
    } else {
        std::fs::remove_file(&target).map_err(|e| {
            api_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete file: {e}"),
            )
        })?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /workspace/rename — rename/move file or directory
pub async fn rename_item(
    State(state): State<AppState>,
    Json(body): Json<RenameRequest>,
) -> Result<Json<ApiResponse<FileNode>>, (StatusCode, Json<ApiError>)> {
    let config = state.config.read().await;
    let workspace_root = config.workspace_root.clone();
    drop(config);

    let old = safe_resolve(&workspace_root, &body.old_path)?;
    let new = safe_resolve(&workspace_root, &body.new_path)?;

    if !old.exists() {
        return Err(api_err(StatusCode::NOT_FOUND, "Source path not found"));
    }
    if new.exists() {
        return Err(api_err(StatusCode::CONFLICT, "Destination already exists"));
    }

    // Ensure parent of destination exists
    if let Some(parent) = new.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            api_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create parent dirs: {e}"),
            )
        })?;
    }

    std::fs::rename(&old, &new).map_err(|e| {
        api_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Rename failed: {e}"),
        )
    })?;

    let is_dir = new.is_dir();
    let name = new
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(ApiResponse::new(FileNode {
        name,
        path: body.new_path,
        is_dir,
        children: None,
        animal_group: None,
        modified_at: None,
    }))
}

/// GET /workspace/config — return current workspace root
pub async fn get_config(State(state): State<AppState>) -> Json<ApiResponse<ConfigResponse>> {
    let config = state.config.read().await;
    ApiResponse::new(ConfigResponse {
        workspace_root: config.workspace_root.clone(),
    })
}

/// PUT /workspace/config — update workspace root
pub async fn update_config(
    State(state): State<AppState>,
    Json(body): Json<ConfigUpdateRequest>,
) -> Result<Json<ApiResponse<ConfigResponse>>, (StatusCode, Json<ApiError>)> {
    let new_root = PathBuf::from(&body.workspace_root);

    // Create the directory if it doesn't exist
    if !new_root.exists() {
        std::fs::create_dir_all(&new_root).map_err(|e| {
            api_err(
                StatusCode::BAD_REQUEST,
                format!("Cannot create workspace directory: {e}"),
            )
        })?;
    }

    if !new_root.is_dir() {
        return Err(api_err(
            StatusCode::BAD_REQUEST,
            "workspace_root must be a directory",
        ));
    }

    let mut config = state.config.write().await;
    config.workspace_root = body.workspace_root.clone();
    drop(config);

    Ok(ApiResponse::new(ConfigResponse {
        workspace_root: body.workspace_root,
    }))
}
