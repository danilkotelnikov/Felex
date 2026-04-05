//! Legacy cap.ru compatibility importer.
//!
//! The application now seeds feeds from the local normalized authority in
//! `database/output`. The legacy cap.ru import entrypoints remain available as
//! compatibility aliases so older workflows do not break.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::db::feeds;
use crate::db::Database;
use crate::scraper::normalized_feed_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub imported: usize,
    pub errors: usize,
    pub total: usize,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CapRuScraper;

impl CapRuScraper {
    pub fn new() -> Self {
        Self
    }

    pub fn import_all(
        &self,
        db: &Database,
        on_progress: impl Fn(usize, usize),
    ) -> Result<ImportReport> {
        let seed_feeds = normalized_feed_db::load_workspace_seed_feeds()?.ok_or_else(|| {
            anyhow!("No generated feed authority or database/output source-family files found")
        })?;

        let total = seed_feeds.len();
        let mut imported = 0usize;
        let mut errors = 0usize;

        for (index, feed) in seed_feeds.iter().enumerate() {
            on_progress(index + 1, total);

            match db.with_conn(|conn| feeds::upsert_feed(conn, feed)) {
                Ok(_) => imported += 1,
                Err(error) => {
                    tracing::warn!(
                        "Failed to merge local feed authority row '{}': {}",
                        feed.name_ru,
                        error
                    );
                    errors += 1;
                }
            }
        }

        Ok(ImportReport {
            imported,
            errors,
            total,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::CapRuScraper;
    use crate::db::feeds;
    use crate::db::Database;

    #[test]
    fn import_all_uses_local_seed_authority() {
        let db = Database::new(":memory:").unwrap();
        db.run_migrations().unwrap();

        let report = CapRuScraper::new().import_all(&db, |_current, _total| {}).unwrap();

        assert!(report.total > 100);
        assert_eq!(report.imported, report.total);
        assert_eq!(report.errors, 0);

        let total = db.with_conn(|conn| feeds::count_feeds(conn, None)).unwrap();
        assert!(total > 100);
    }
}
