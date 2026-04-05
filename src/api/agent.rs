//! Agent API endpoints

use crate::agent::{AgentConfig, AgentManager, ChatMessage};
use crate::db::Database;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared agent state
pub struct AgentState {
    pub manager: Option<AgentManager>,
    pub config: AgentConfig,
    pub db: Arc<Database>,
}

impl AgentState {
    pub async fn new(db: Arc<Database>) -> Self {
        let config = AgentConfig::from_env();
        let manager = AgentManager::new(config.clone(), db.clone()).await.ok();

        Self {
            manager,
            config,
            db,
        }
    }
}

/// Get agent status
pub async fn get_status(State(state): State<Arc<RwLock<AgentState>>>) -> Json<AgentStatusResponse> {
    let state = state.read().await;

    if let Some(ref manager) = state.manager {
        manager.refresh_status().await;
        let s = manager.status();
        Json(AgentStatusResponse {
            model_loaded: s.model_loaded,
            model_name: s.model_name,
            backend: s.backend,
            web_enabled: s.web_enabled,
            context_size: s.context_size,
        })
    } else {
        Json(AgentStatusResponse {
            model_loaded: false,
            model_name: state.config.model_name.clone(),
            backend: state.config.backend.clone(),
            web_enabled: state.config.web_enabled,
            context_size: state.config.context_size,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentStatusResponse {
    pub model_loaded: bool,
    pub model_name: String,
    pub backend: String,
    pub web_enabled: bool,
    pub context_size: i32,
}

/// Chat request body
#[derive(Debug, Deserialize)]
pub struct ApiChatRequest {
    pub messages: Vec<ChatMessage>,
    pub context: Option<ChatContextRequest>,
}

#[derive(Debug, Deserialize)]
pub struct ChatContextRequest {
    pub animal_type: Option<String>,
    pub production_level: Option<String>,
    pub current_ration: Option<String>,
    pub nutrient_status: Option<String>,
}

/// Chat response
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: String,
    pub done: bool,
}

/// Non-streaming chat endpoint
pub async fn chat(
    State(state): State<Arc<RwLock<AgentState>>>,
    Json(request): Json<ApiChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let state = state.read().await;

    let manager = state.manager.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "Агент не инициализирован. Проверьте Ollama и модель {}.",
                state.config.model_name
            ),
        )
    })?;

    let context = request
        .context
        .map(|ctx| crate::agent::manager::ChatContext {
            animal_type: ctx.animal_type.unwrap_or_else(|| "Dairy Cow".to_string()),
            production_level: ctx
                .production_level
                .unwrap_or_else(|| "30 kg milk/day".to_string()),
            current_ration: ctx.current_ration,
            nutrient_status: ctx.nutrient_status,
        });

    let response = manager
        .chat(&request.messages, context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ChatResponse {
        message: response,
        done: true,
    }))
}

/// Streaming chat endpoint currently uses non-streaming but returns JSON.
pub async fn chat_stream(
    State(state): State<Arc<RwLock<AgentState>>>,
    Json(request): Json<ApiChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let state_guard = state.read().await;

    let manager = state_guard.manager.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "Агент не инициализирован. Проверьте Ollama и модель {}.",
                state_guard.config.model_name
            ),
        )
    })?;

    let context = request
        .context
        .map(|ctx| crate::agent::manager::ChatContext {
            animal_type: ctx.animal_type.unwrap_or_else(|| "Dairy Cow".to_string()),
            production_level: ctx
                .production_level
                .unwrap_or_else(|| "30 kg milk/day".to_string()),
            current_ration: ctx.current_ration,
            nutrient_status: ctx.nutrient_status,
        });

    let response = manager
        .chat(&request.messages, context)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ChatResponse {
        message: response,
        done: true,
    }))
}

/// Reload request body (optional)
#[derive(Debug, Deserialize, Default)]
pub struct ReloadRequest {
    pub model: Option<String>,
    pub backend: Option<String>,
    pub web_enabled: Option<bool>,
    pub context_size: Option<i32>,
}

/// Reload agent configuration with optional overrides.
pub async fn reload(
    State(state): State<Arc<RwLock<AgentState>>>,
    body: Option<Json<ReloadRequest>>,
) -> Result<Json<AgentStatusResponse>, (StatusCode, String)> {
    let mut state = state.write().await;
    let mut config = state.config.clone();

    if let Some(Json(req)) = body {
        if let Some(model) = req.model {
            tracing::info!("Switching model to: {}", model);
            config.model_name = model;
        }
        if let Some(backend) = req.backend {
            tracing::info!("Switching backend to: {}", backend);
            config.backend = backend;
        }
        if let Some(web) = req.web_enabled {
            config.web_enabled = web;
        }
        if let Some(context_size) = req.context_size {
            config.context_size = normalize_context_size(context_size);
        }
    }

    state.config = config;
    state.manager = AgentManager::new(state.config.clone(), state.db.clone())
        .await
        .ok();

    if let Some(ref manager) = state.manager {
        manager.refresh_status().await;
        let s = manager.status();
        Ok(Json(AgentStatusResponse {
            model_loaded: s.model_loaded,
            model_name: s.model_name,
            backend: s.backend,
            web_enabled: s.web_enabled,
            context_size: s.context_size,
        }))
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Failed to initialize agent".to_string(),
        ))
    }
}

fn normalize_context_size(value: i32) -> i32 {
    value.clamp(1024, 65536)
}
