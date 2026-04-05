//! Felex Server - Standalone entry point
//!
//! Starts the Axum web server for the Felex API.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use felex::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "felex=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut config = AppConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if i + 1 < args.len() {
                    config.server_port = args[i + 1].parse().unwrap_or(7432);
                    i += 1;
                }
            }
            "--static" => {
                if i + 1 < args.len() {
                    config.static_dir = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--db" => {
                if i + 1 < args.len() {
                    config.database_path = args[i + 1].clone();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    felex::start_http_server(config).await
}
