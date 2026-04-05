//! Felex Tauri Application
//!
//! Desktop application for animal feed ration calculation.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod export;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, State};
use tauri_plugin_shell::ShellExt;
use tokio::sync::{oneshot, RwLock};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Import felex library
use felex::{AppConfig, AppState};
use felex::db::rations::RationItem;

/// Start the embedded HTTP API server reusing an existing AppState
async fn start_embedded_server(
    state: AppState,
    ready_tx: Option<oneshot::Sender<Result<(), String>>>,
) {
    tracing::info!("Starting embedded Felex API server (reusing initialized state)");
    if let Err(e) = felex::start_http_server_with_state_and_signal(state, ready_tx).await {
        tracing::error!("Embedded API server failed: {}", e);
        eprintln!("Embedded API server error: {}", e);
    }
}

/// Application state wrapper for Tauri
struct TauriState {
    app_state: Arc<RwLock<Option<AppState>>>,
}

/// Initialize the application
#[tauri::command]
async fn init_app(state: State<'_, TauriState>) -> Result<String, String> {
    let config = AppConfig::default();

    match felex::init_app_state(config).await {
        Ok(app_state) => {
            let mut state_lock = state.app_state.write().await;
            *state_lock = Some(app_state);
            Ok("Application initialized successfully".to_string())
        }
        Err(e) => Err(format!("Failed to initialize: {}", e)),
    }
}

/// Get application status
#[tauri::command]
async fn get_status(state: State<'_, TauriState>) -> Result<AppStatus, String> {
    let state_lock = state.app_state.read().await;

    if state_lock.is_some() {
        Ok(AppStatus {
            initialized: true,
            database_connected: true,
            agent_ready: false,
        })
    } else {
        Ok(AppStatus {
            initialized: false,
            database_connected: false,
            agent_ready: false,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppStatus {
    initialized: bool,
    database_connected: bool,
    agent_ready: bool,
}

#[tauri::command]
async fn save_ration_export(request: export::ExportCommandRequest) -> Result<export::ExportResponse, String> {
    export::save_ration_export(request).map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(deprecated)]
async fn open_external_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("URL is empty".to_string());
    }

    app.shell()
        .open(trimmed, None)
        .map_err(|error| error.to_string())
}

/// List feeds
#[tauri::command]
async fn list_feeds(
    state: State<'_, TauriState>,
    category: Option<String>,
    search: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let state_lock = state.app_state.read().await;

    let app_state = state_lock
        .as_ref()
        .ok_or_else(|| "App not initialized".to_string())?;

    let feeds = app_state.db.with_conn(|conn| {
        felex::db::feeds::list_feeds(conn, category.as_deref(), search.as_deref(), Some(100), None)
    }).map_err(|e| e.to_string())?;

    Ok(feeds
        .into_iter()
        .map(|f| serde_json::to_value(f).unwrap())
        .collect())
}

/// Calculate nutrients for a ration
#[tauri::command]
async fn calculate_nutrients(
    state: State<'_, TauriState>,
    items: Vec<RationItemInput>,
) -> Result<serde_json::Value, String> {
    let state_lock = state.app_state.read().await;

    let app_state = state_lock
        .as_ref()
        .ok_or_else(|| "App not initialized".to_string())?;

    // Get feeds from database and build RationItems
    let ration_items: Vec<RationItem> = app_state.db.with_conn(|conn| {
        let mut result = Vec::new();
        for (idx, item) in items.iter().enumerate() {
            if let Ok(Some(feed)) = felex::db::feeds::get_feed(conn, item.feed_id) {
                result.push(RationItem {
                    id: None,
                    ration_id: 0,
                    feed_id: item.feed_id,
                    feed: Some(feed),
                    amount_kg: item.amount_kg,
                    is_locked: false,
                    sort_order: idx as i32,
                });
            }
        }
        Ok(result)
    }).map_err(|e: anyhow::Error| e.to_string())?;

    // Calculate nutrients
    let summary = felex::diet_engine::nutrient_calc::calculate_nutrients(&ration_items);

    serde_json::to_value(summary).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Deserialize)]
struct RationItemInput {
    feed_id: i64,
    amount_kg: f64,
}

/// Optimize a ration
#[tauri::command]
async fn optimize_ration(
    _state: State<'_, TauriState>,
    _items: Vec<RationItemInput>,
    _mode: String,
) -> Result<serde_json::Value, String> {
    // Placeholder - full optimization requires norms context
    Ok(serde_json::json!({
        "status": "not_implemented",
        "message": "Optimization requires full context. Use the web API."
    }))
}

fn main() {
    // Initialize tracing so log messages are visible
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "felex=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(TauriState {
            app_state: Arc::new(RwLock::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            init_app,
            get_status,
            list_feeds,
            calculate_nutrients,
            optimize_ration,
            save_ration_export,
            open_external_url,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let config = AppConfig::default();
                let app_state = felex::init_app_state(config).await.map_err(|error| {
                    std::io::Error::other(format!(
                        "Failed to initialize Felex application state: {error}"
                    ))
                })?;

                tracing::info!("App state initialized successfully");

                {
                    let tauri_state: State<TauriState> = handle.state();
                    let mut lock = tauri_state.app_state.write().await;
                    *lock = Some(app_state.clone());
                }

                let (ready_tx, ready_rx) = oneshot::channel();
                tauri::async_runtime::spawn(async move {
                    start_embedded_server(app_state, Some(ready_tx)).await;
                });

                match ready_rx.await {
                    Ok(Ok(())) => {
                        tracing::info!("Embedded API listener is ready");
                    }
                    Ok(Err(message)) => {
                        return Err(std::io::Error::other(format!(
                            "Embedded API startup failed: {message}"
                        ))
                        .into());
                    }
                    Err(_) => {
                        return Err(std::io::Error::other(
                            "Embedded API readiness channel closed unexpectedly",
                        )
                        .into());
                    }
                }

                if let Some(window) = handle.get_webview_window("main") {
                    window
                        .show()
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    window
                        .set_focus()
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                }

                Ok::<(), Box<dyn std::error::Error>>(())
            })?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
