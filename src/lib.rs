//! Felex - Animal Feed Ration Calculation System
//!
//! A comprehensive system for creating balanced feed rations for
//! cattle, swine, and poultry with LP optimization and AI assistance.

pub mod agent;
pub mod api;
pub mod db;
pub mod diet_engine;
pub mod norms;
pub mod nutrients;
pub mod presets;
pub mod scraper;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<db::Database>,
    pub config: Arc<RwLock<AppConfig>>,
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_path: String,
    pub server_port: u16,
    pub static_dir: Option<String>,
    pub cors_origins: Vec<String>,
    pub workspace_root: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let workspace_root = dirs::document_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("Felex/projects")
            .to_string_lossy()
            .to_string();

        Self {
            database_path: "felex.db".to_string(),
            server_port: 7432,
            static_dir: None,
            cors_origins: vec!["http://localhost:5173".to_string()],
            workspace_root,
        }
    }
}

/// Initialize application state
pub async fn init_app_state(config: AppConfig) -> anyhow::Result<AppState> {
    let db = db::Database::new(&config.database_path)?;
    db.run_migrations()?;

    if let Err(e) = scraper::seed_from_json_if_empty(&db) {
        tracing::warn!("Failed to seed feeds: {}", e);
    }

    if let Err(e) = db.with_conn(|conn| db::prices::repair_feed_price_cache(conn)) {
        tracing::warn!("Failed to repair cached feed prices: {}", e);
    }

    if let Err(e) = scraper::refresh_inferred_prices(&db) {
        tracing::warn!("Failed to refresh inferred benchmark prices: {}", e);
    }

    Ok(AppState {
        db: Arc::new(db),
        config: Arc::new(RwLock::new(config)),
    })
}

/// Create the Axum router with all API routes
pub fn create_router(state: AppState, agent_state: Arc<RwLock<api::agent::AgentState>>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build agent router with its own state, then convert to stateless Router
    let agent_router: Router = Router::new()
        .route("/status", get(api::agent::get_status))
        .route("/chat", post(api::agent::chat))
        .route("/chat/stream", post(api::agent::chat_stream))
        .route("/reload", post(api::agent::reload))
        .with_state(agent_state);

    // Build API router with AppState, then convert to stateless Router
    let api_router: Router = api_routes().with_state(state);

    Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/api/v1", api_router)
        .nest("/api/v1/agent", agent_router)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        // Feeds
        .route("/feeds", get(api::feeds::list_feeds))
        .route("/feeds", post(api::feeds::create_feed))
        .route("/feeds/:id", get(api::feeds::get_feed))
        .route("/feeds/:id/price", get(api::prices::get_jit_feed_price))
        .route("/feeds/:id", put(api::feeds::update_feed))
        .route("/feeds/:id", delete(api::feeds::delete_feed))
        .route("/feeds/import/capru", post(api::feeds::import_capru))
        .route("/feeds/sync", post(api::feeds::sync_feeds))
        // Rations
        .route("/rations", get(api::rations::list_rations))
        .route("/rations", post(api::rations::create_ration))
        .route("/rations/:id", get(api::rations::get_ration))
        .route("/rations/:id", put(api::rations::update_ration))
        .route("/rations/:id/optimize", post(api::rations::optimize_ration))
        .route(
            "/rations/:id/alternatives",
            post(api::rations::optimize_ration_alternatives),
        )
        .route(
            "/rations/:id/auto-populate",
            post(api::rations::auto_populate_ration),
        )
        .route("/rations/:id/screen", post(api::rations::screen_ration))
        .route("/rations/:id/nutrients", get(api::rations::get_nutrients))
        .route("/rations/:id/economics", get(api::rations::get_economics))
        // Presets
        .route("/presets", get(api::rations::list_presets))
        // Animals
        .route("/animals", get(api::animals::list_animals))
        .route("/animals", post(api::animals::create_animal))
        .route("/animals/:id", get(api::animals::get_animal))
        // Norms
        .route("/norms/:animal_group_id", get(api::norms::get_norms))
        .route(
            "/norms/:animal_group_id/resolve",
            post(api::norms::resolve_norms),
        )
        // Prices
        .route("/prices", get(api::prices::list_prices))
        .route("/prices/fetch", post(api::prices::fetch_prices))
        .route("/prices/:feed_id", put(api::prices::update_price))
        .route(
            "/prices/:feed_id/history",
            get(api::prices::get_price_history),
        )
        // Workspace
        .route("/workspace/tree", get(api::workspace::get_tree))
        .route("/workspace/folder", post(api::workspace::create_folder))
        .route("/workspace/ration", post(api::workspace::create_ration))
        .route("/workspace/ration", get(api::workspace::get_ration))
        .route("/workspace/ration", put(api::workspace::update_ration))
        .route("/workspace/ration", delete(api::workspace::delete_ration))
        .route("/workspace/rename", post(api::workspace::rename_item))
        .route("/workspace/config", get(api::workspace::get_config))
        .route("/workspace/config", put(api::workspace::update_config))
        // App metadata
        .route("/app/meta", get(api::app::get_meta))
}

/// Start the HTTP API server with a fresh AppState. For standalone binary use.
pub async fn start_http_server(config: AppConfig) -> anyhow::Result<()> {
    let state = init_app_state(config).await?;
    start_http_server_with_state(state).await
}

/// Start the HTTP API server reusing an already-initialized AppState.
/// Use this from Tauri to avoid double DB/feed initialization.
pub async fn start_http_server_with_state(state: AppState) -> anyhow::Result<()> {
    start_http_server_with_state_and_signal(state, None).await
}

/// Start the HTTP API server and optionally signal when the listener is ready.
pub async fn start_http_server_with_state_and_signal(
    state: AppState,
    ready_tx: Option<oneshot::Sender<Result<(), String>>>,
) -> anyhow::Result<()> {
    let port = {
        let cfg = state.config.read().await;
        cfg.server_port
    };

    // Create agent state immediately with manager=None so the server can start
    // accepting connections right away. The agent connects in the background.
    let agent_db = state.db.clone();
    let agent_config = agent::AgentConfig::from_env();
    let agent_state = Arc::new(RwLock::new(api::agent::AgentState {
        manager: None,
        config: agent_config.clone(),
        db: agent_db.clone(),
    }));

    let app = create_router(state, agent_state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Felex API server starting on http://{}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            if let Some(tx) = ready_tx {
                let _ = tx.send(Ok(()));
            }
            listener
        }
        Err(error) => {
            if let Some(tx) = ready_tx {
                let _ = tx.send(Err(error.to_string()));
            }
            return Err(error.into());
        }
    };
    tracing::info!("Felex API server listening on http://{}", addr);

    // Spawn agent initialization in the background so it doesn't block serving
    tokio::spawn(async move {
        tracing::info!("Initializing AI agent in background...");
        match agent::AgentManager::new(agent_config, agent_db).await {
            Ok(manager) => {
                let mut lock = agent_state.write().await;
                lock.manager = Some(manager);
                tracing::info!("AI agent initialized successfully");
            }
            Err(e) => {
                tracing::warn!("AI agent initialization failed (non-fatal): {}", e);
            }
        }
    });

    axum::serve(listener, app).await?;

    Ok(())
}
