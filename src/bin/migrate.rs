//! Database migration utility

use felex::db::Database;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "felex.db".to_string());

    tracing::info!("Running migrations on: {}", db_path);

    let db = Database::new(&db_path)?;
    db.run_migrations()?;

    // Seed default animal groups
    db.with_conn(|conn| felex::db::animals::seed_default_groups(conn))?;

    tracing::info!("Migrations completed successfully");
    Ok(())
}
