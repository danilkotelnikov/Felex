//! Feed import utility

use std::path::Path;

use felex::db::feeds;
use felex::db::Database;
use felex::scraper::normalized_feed_db;

fn import_local_feed_authority(db: &Database, file_path: Option<&str>) -> anyhow::Result<usize> {
    let feed_rows = match file_path {
        Some(path) => normalized_feed_db::load_seed_feeds_from_path(Path::new(path))?,
        None => normalized_feed_db::load_workspace_seed_feeds()?.ok_or_else(|| {
            anyhow::anyhow!("No generated feed authority or database/output source-family files found")
        })?,
    };

    let mut imported = 0usize;
    db.with_conn(|conn| {
        for feed in &feed_rows {
            feeds::upsert_feed(conn, feed)?;
            imported += 1;
        }
        Ok(())
    })?;

    Ok(imported)
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let source = args
        .iter()
        .position(|a| a == "--source")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("normalized_json");

    let db_path = args
        .iter()
        .position(|a| a == "--db")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("felex.db");

    tracing::info!("Importing feeds from {} into {}", source, db_path);

    let db = Database::new(db_path)?;
    db.run_migrations()?;

    match source {
        "capru" | "normalized_json" => {
            let file_path = args
                .iter()
                .position(|a| a == "--file")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.as_str());

            if source == "capru" {
                tracing::warn!(
                    "--source capru is deprecated; importing from the local feed authority instead"
                );
            }

            let imported = import_local_feed_authority(&db, file_path)?;
            tracing::info!(
                "Local catalog import complete: {} rows merged from {}",
                imported,
                file_path.unwrap_or("database/output source family")
            );
        }
        "csv" => {
            let file_path = args
                .iter()
                .position(|a| a == "--file")
                .and_then(|i| args.get(i + 1))
                .ok_or_else(|| anyhow::anyhow!("--file required for CSV import"))?;

            tracing::info!("CSV import from {} not yet implemented", file_path);
        }
        _ => {
            anyhow::bail!("Unknown source: {}", source);
        }
    }

    Ok(())
}
