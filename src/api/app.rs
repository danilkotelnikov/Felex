//! Application metadata endpoints.

use axum::{extract::State, Json};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

use super::{ApiError, ApiResponse};
use crate::{
    db::{
        feed_quality::{
            self, FeedCriticalNutrientKey, FeedProfileStatus, FeedSourceKind, FeedTranslationStatus,
        },
        feeds,
    },
    diet_engine::feed_groups::{assess_feed_suitability, FeedSuitabilityStatus},
    AppState,
};

#[derive(Debug, Default, Serialize)]
pub struct FeedSourceCounts {
    pub normalized: i64,
    pub curated: i64,
    pub custom: i64,
    pub imported: i64,
}

#[derive(Debug, Default, Serialize)]
pub struct FeedTranslationCounts {
    pub ready: i64,
    pub source_only: i64,
}

#[derive(Debug, Default, Serialize)]
pub struct FeedProfileCounts {
    pub complete: i64,
    pub partial: i64,
    pub limited: i64,
}

#[derive(Debug, Default, Serialize)]
pub struct FeedCatalogQualitySummary {
    pub source_counts: FeedSourceCounts,
    pub translation_counts: FeedTranslationCounts,
    pub profile_counts: FeedProfileCounts,
    pub priced_feed_count: i64,
    pub unpriced_feed_count: i64,
    pub benchmark_critical_contexts: Vec<FeedCriticalCoverageContextSummary>,
}

#[derive(Debug, Serialize)]
pub struct FeedCriticalMissingKeyCount {
    pub key: FeedCriticalNutrientKey,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct FeedCriticalCoverageContextSummary {
    pub id: String,
    pub species: String,
    pub stage_context: String,
    pub audited_feed_count: i64,
    pub coverage_counts: FeedProfileCounts,
    pub top_missing_keys: Vec<FeedCriticalMissingKeyCount>,
}

#[derive(Debug, Serialize)]
pub struct AppMetaResponse {
    pub version: String,
    pub database_path: String,
    pub workspace_root: String,
    pub feed_count: i64,
    pub last_sync_at: Option<String>,
    pub catalog_quality: FeedCatalogQualitySummary,
}

struct CriticalAuditContextPreset {
    id: &'static str,
    species: &'static str,
    stage_context: &'static str,
}

const CRITICAL_AUDIT_CONTEXTS: &[CriticalAuditContextPreset] = &[
    CriticalAuditContextPreset {
        id: "cattle_dairy",
        species: "cattle",
        stage_context: "cattle_dairy_early_lact",
    },
    CriticalAuditContextPreset {
        id: "cattle_beef",
        species: "cattle",
        stage_context: "cattle_beef_finisher",
    },
    CriticalAuditContextPreset {
        id: "swine_finisher",
        species: "swine",
        stage_context: "swine_finisher",
    },
    CriticalAuditContextPreset {
        id: "swine_sow",
        species: "swine",
        stage_context: "swine_sow_lactating",
    },
    CriticalAuditContextPreset {
        id: "poultry_broiler",
        species: "poultry",
        stage_context: "poultry_broiler_starter",
    },
    CriticalAuditContextPreset {
        id: "poultry_layer",
        species: "poultry",
        stage_context: "poultry_layer_peak",
    },
];

fn benchmark_critical_context_summaries(
    feeds: &[crate::db::feeds::Feed],
) -> Vec<FeedCriticalCoverageContextSummary> {
    CRITICAL_AUDIT_CONTEXTS
        .iter()
        .map(|preset| {
            let mut coverage_counts = FeedProfileCounts::default();
            let mut audited_feed_count = 0_i64;
            let mut missing_counts = BTreeMap::<FeedCriticalNutrientKey, i64>::new();

            for feed in feeds {
                let suitability =
                    assess_feed_suitability(feed, preset.species, Some(preset.stage_context));
                if suitability.status == FeedSuitabilityStatus::Restricted {
                    continue;
                }

                let Some(audit) = feed_quality::audit_feed_critical_coverage(
                    feed,
                    preset.species,
                    Some(preset.stage_context),
                ) else {
                    continue;
                };

                audited_feed_count += 1;
                match audit.coverage_status {
                    FeedProfileStatus::Complete => coverage_counts.complete += 1,
                    FeedProfileStatus::Partial => coverage_counts.partial += 1,
                    FeedProfileStatus::Limited => coverage_counts.limited += 1,
                }

                for key in audit.missing_keys {
                    *missing_counts.entry(key).or_insert(0) += 1;
                }
            }

            let mut top_missing_keys = missing_counts.into_iter().collect::<Vec<_>>();
            top_missing_keys
                .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

            FeedCriticalCoverageContextSummary {
                id: preset.id.to_string(),
                species: preset.species.to_string(),
                stage_context: preset.stage_context.to_string(),
                audited_feed_count,
                coverage_counts,
                top_missing_keys: top_missing_keys
                    .into_iter()
                    .take(3)
                    .map(|(key, count)| FeedCriticalMissingKeyCount { key, count })
                    .collect(),
            }
        })
        .collect()
}

pub async fn get_meta(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AppMetaResponse>>, ApiError> {
    let config = state.config.read().await.clone();
    let database_path = PathBuf::from(&config.database_path);
    let absolute_database_path = if database_path.is_absolute() {
        database_path
    } else {
        std::env::current_dir()
            .map_err(anyhow::Error::from)?
            .join(database_path)
    };

    let (feed_count, last_sync_at, catalog_quality) = state.db.with_conn(|conn| {
        let feeds = feeds::list_feeds(conn, None, None, None, None)?;
        let last_sync_at = conn.query_row(
            "SELECT MAX(price_updated_at) FROM feeds WHERE price_updated_at IS NOT NULL",
            [],
            |row| row.get::<_, Option<String>>(0),
        )?;

        let mut catalog_quality = FeedCatalogQualitySummary::default();

        for feed in &feeds {
            let audit = feed_quality::audit_feed(feed);

            match audit.source_kind {
                FeedSourceKind::Normalized => catalog_quality.source_counts.normalized += 1,
                FeedSourceKind::Curated => catalog_quality.source_counts.curated += 1,
                FeedSourceKind::Custom => catalog_quality.source_counts.custom += 1,
                FeedSourceKind::Imported => catalog_quality.source_counts.imported += 1,
            }

            match audit.translation_status {
                FeedTranslationStatus::Ready => catalog_quality.translation_counts.ready += 1,
                FeedTranslationStatus::SourceOnly => {
                    catalog_quality.translation_counts.source_only += 1
                }
            }

            match audit.profile_status {
                FeedProfileStatus::Complete => catalog_quality.profile_counts.complete += 1,
                FeedProfileStatus::Partial => catalog_quality.profile_counts.partial += 1,
                FeedProfileStatus::Limited => catalog_quality.profile_counts.limited += 1,
            }

            if feed
                .price_per_ton
                .is_some_and(|value| value.is_finite() && value > 0.0)
            {
                catalog_quality.priced_feed_count += 1;
            } else {
                catalog_quality.unpriced_feed_count += 1;
            }
        }

        catalog_quality.benchmark_critical_contexts = benchmark_critical_context_summaries(&feeds);

        Ok((feeds.len() as i64, last_sync_at, catalog_quality))
    })?;

    Ok(ApiResponse::new(AppMetaResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        database_path: absolute_database_path.display().to_string(),
        workspace_root: config.workspace_root,
        feed_count,
        last_sync_at,
        catalog_quality,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::feeds::Feed, db::Database, AppConfig, AppState};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn test_state() -> AppState {
        let db = Database::new(":memory:").unwrap();
        db.run_migrations().unwrap();

        let config = AppConfig {
            database_path: "test.db".to_string(),
            ..AppConfig::default()
        };

        AppState {
            db: Arc::new(db),
            config: Arc::new(RwLock::new(config)),
        }
    }

    #[tokio::test]
    async fn get_meta_returns_catalog_quality_summary() {
        let state = test_state();

        state
            .db
            .with_conn(|conn| {
                feeds::insert_feed(
                    conn,
                    &Feed {
                        source_id: Some("seed:normalized-db:n1".to_string()),
                        name_ru: "Кукуруза".to_string(),
                        name_en: Some("Corn".to_string()),
                        category: "grain".to_string(),
                        dry_matter: Some(86.0),
                        koe: Some(1.2),
                        energy_oe_cattle: Some(13.4),
                        crude_protein: Some(85.0),
                        dig_protein_cattle: Some(62.0),
                        crude_fiber: Some(11.0),
                        calcium: Some(0.3),
                        phosphorus: Some(2.7),
                        price_per_ton: Some(16200.0),
                        price_updated_at: Some("2026-03-15T10:00:00Z".to_string()),
                        ..Default::default()
                    },
                )?;

                feeds::insert_feed(
                    conn,
                    &Feed {
                        source_id: Some("seed:soymeal".to_string()),
                        name_ru: "Шрот соевый".to_string(),
                        category: "oilseed_meal".to_string(),
                        crude_protein: Some(435.0),
                        dig_protein_cattle: Some(320.0),
                        lysine: Some(26.0),
                        calcium: Some(3.2),
                        phosphorus: Some(6.1),
                        ..Default::default()
                    },
                )?;

                feeds::insert_feed(
                    conn,
                    &Feed {
                        name_ru: "Премикс 1%".to_string(),
                        category: "premix".to_string(),
                        calcium: Some(120.0),
                        is_custom: true,
                        ..Default::default()
                    },
                )?;

                Ok::<(), anyhow::Error>(())
            })
            .unwrap();

        let Json(response) = get_meta(State(state)).await.unwrap();

        assert_eq!(response.data.feed_count, 3);
        assert_eq!(response.data.catalog_quality.source_counts.normalized, 1);
        assert_eq!(response.data.catalog_quality.source_counts.curated, 1);
        assert_eq!(response.data.catalog_quality.source_counts.custom, 1);
        assert_eq!(response.data.catalog_quality.source_counts.imported, 0);
        assert_eq!(response.data.catalog_quality.translation_counts.ready, 1);
        assert_eq!(
            response.data.catalog_quality.translation_counts.source_only,
            2
        );
        assert_eq!(response.data.catalog_quality.profile_counts.complete, 1);
        assert_eq!(response.data.catalog_quality.profile_counts.partial, 1);
        assert_eq!(response.data.catalog_quality.profile_counts.limited, 1);
        assert_eq!(response.data.catalog_quality.priced_feed_count, 1);
        assert_eq!(response.data.catalog_quality.unpriced_feed_count, 2);
        let dairy_summary = response
            .data
            .catalog_quality
            .benchmark_critical_contexts
            .iter()
            .find(|summary| summary.id == "cattle_dairy")
            .expect("dairy critical summary");
        assert_eq!(dairy_summary.audited_feed_count, 3);
        assert_eq!(dairy_summary.coverage_counts.complete, 1);
        assert_eq!(dairy_summary.coverage_counts.partial, 1);
        assert_eq!(dairy_summary.coverage_counts.limited, 1);
        assert!(dairy_summary
            .top_missing_keys
            .iter()
            .any(|item| { item.key == FeedCriticalNutrientKey::VitD3 && item.count == 1 }));
        assert_eq!(
            response.data.last_sync_at.as_deref(),
            Some("2026-03-15T10:00:00Z")
        );
    }
}
