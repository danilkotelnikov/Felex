//! Feeds API handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use super::{ApiError, ApiResponse, PaginatedResponse};
use crate::db::feed_quality;
use crate::db::feed_source_profiles;
use crate::db::feeds::{self, Feed};
use crate::db::prices;
use crate::diet_engine::feed_groups::assess_feed_suitability;
use crate::scraper::SyncReport;
use crate::AppState;

fn enrich_feed(feed: Feed, include_source_nutrition: bool) -> Feed {
    let feed = feed_source_profiles::enrich_feed(feed, include_source_nutrition);
    feed_quality::enrich_feed(feed)
}

fn enrich_feed_for_context(
    mut feed: Feed,
    species: Option<&str>,
    stage_context: Option<&str>,
) -> Feed {
    let Some(species) = species.filter(|species| !species.trim().is_empty()) else {
        return feed;
    };

    feed.critical_nutrient_audit =
        feed_quality::audit_feed_critical_coverage(&feed, species, stage_context);

    let assessment = assess_feed_suitability(&feed, species, stage_context);
    feed.suitability_status = Some(assessment.status);
    feed.suitability_notes = Some(assessment.notes);
    feed.suitability_max_inclusion_pct = assessment.max_inclusion_pct;
    feed
}

fn enrich_feeds(feeds: Vec<Feed>) -> Vec<Feed> {
    feeds
        .into_iter()
        .map(|feed| enrich_feed(feed, false))
        .collect()
}

/// Query parameters for listing feeds
#[derive(Debug, Deserialize)]
pub struct ListFeedsQuery {
    pub category: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub species: Option<String>,
    pub stage_context: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FeedContextQuery {
    pub species: Option<String>,
    pub stage_context: Option<String>,
}

/// List all feeds
pub async fn list_feeds(
    State(state): State<AppState>,
    Query(query): Query<ListFeedsQuery>,
) -> Result<Json<PaginatedResponse<Feed>>, ApiError> {
    let existing_total = state.db.with_conn(|conn| feeds::count_feeds(conn, None))?;
    if existing_total == 0 {
        crate::scraper::multi_source::seed_from_json_if_empty(&state.db)?;
    }

    let result = state.db.with_conn(|conn| {
        let feeds = feeds::list_feeds(
            conn,
            query.category.as_deref(),
            query.search.as_deref(),
            query.limit,
            query.offset,
        )?;
        let total = feeds::count_feeds_filtered(
            conn,
            query.category.as_deref(),
            query.search.as_deref(),
        )?;
        let feeds = enrich_feeds(feeds)
            .into_iter()
            .map(|feed| {
                enrich_feed_for_context(
                    feed,
                    query.species.as_deref(),
                    query.stage_context.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        Ok((feeds, total))
    })?;

    Ok(Json(PaginatedResponse {
        data: result.0,
        total: result.1,
        limit: query.limit.unwrap_or(100),
        offset: query.offset.unwrap_or(0),
    }))
}

/// Get a single feed
pub async fn get_feed(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(context): Query<FeedContextQuery>,
) -> Result<Json<ApiResponse<Feed>>, (StatusCode, Json<ApiError>)> {
    let feed = state
        .db
        .with_conn(|conn| feeds::get_feed(conn, id))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::from(e))))?;

    match feed {
        Some(f) => Ok(ApiResponse::new(enrich_feed_for_context(
            enrich_feed(f, true),
            context.species.as_deref(),
            context.stage_context.as_deref(),
        ))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "not_found".to_string(),
                message: format!("Feed {} not found", id),
            }),
        )),
    }
}

/// Create feed request
#[derive(Debug, Deserialize)]
pub struct CreateFeedRequest {
    pub name_ru: String,
    pub name_en: Option<String>,
    pub category: String,
    pub subcategory: Option<String>,
    pub dry_matter: Option<f64>,
    pub energy_oe_cattle: Option<f64>,
    pub energy_oe_pig: Option<f64>,
    pub energy_oe_poultry: Option<f64>,
    pub crude_protein: Option<f64>,
    pub crude_fat: Option<f64>,
    pub crude_fiber: Option<f64>,
    pub calcium: Option<f64>,
    pub phosphorus: Option<f64>,
    pub price_per_ton: Option<f64>,
    pub notes: Option<String>,
}

/// Create a new feed
pub async fn create_feed(
    State(state): State<AppState>,
    Json(req): Json<CreateFeedRequest>,
) -> Result<(StatusCode, Json<ApiResponse<i64>>), ApiError> {
    let feed = Feed {
        name_ru: req.name_ru,
        name_en: req.name_en,
        category: req.category,
        subcategory: req.subcategory,
        dry_matter: req.dry_matter,
        energy_oe_cattle: req.energy_oe_cattle,
        energy_oe_pig: req.energy_oe_pig,
        energy_oe_poultry: req.energy_oe_poultry,
        crude_protein: req.crude_protein,
        crude_fat: req.crude_fat,
        crude_fiber: req.crude_fiber,
        calcium: req.calcium,
        phosphorus: req.phosphorus,
        price_per_ton: req.price_per_ton,
        notes: req.notes,
        is_custom: true,
        ..Default::default()
    };

    let id = state.db.with_conn(|conn| feeds::insert_feed(conn, &feed))?;

    if let Some(price_per_ton) = req.price_per_ton {
        state.db.with_conn(|conn| {
            prices::upsert_price(
                conn,
                &prices::FeedPrice {
                    id: None,
                    feed_id: id,
                    region: None,
                    price_rubles_per_ton: price_per_ton,
                    price_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                    source: Some("manual".to_string()),
                    notes: None,
                    created_at: None,
                },
            )
        })?;
    }

    Ok((StatusCode::CREATED, ApiResponse::new(id)))
}

/// Update a feed
pub async fn update_feed(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<CreateFeedRequest>,
) -> Result<StatusCode, ApiError> {
    let feed = Feed {
        id: Some(id),
        name_ru: req.name_ru,
        name_en: req.name_en,
        category: req.category,
        subcategory: req.subcategory,
        dry_matter: req.dry_matter,
        energy_oe_cattle: req.energy_oe_cattle,
        energy_oe_pig: req.energy_oe_pig,
        energy_oe_poultry: req.energy_oe_poultry,
        crude_protein: req.crude_protein,
        crude_fat: req.crude_fat,
        crude_fiber: req.crude_fiber,
        calcium: req.calcium,
        phosphorus: req.phosphorus,
        price_per_ton: req.price_per_ton,
        notes: req.notes,
        ..Default::default()
    };

    state
        .db
        .with_conn(|conn| feeds::update_feed(conn, id, &feed))?;

    match req.price_per_ton {
        Some(price_per_ton) => {
            state.db.with_conn(|conn| {
                prices::upsert_price(
                    conn,
                    &prices::FeedPrice {
                        id: None,
                        feed_id: id,
                        region: None,
                        price_rubles_per_ton: price_per_ton,
                        price_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                        source: Some("manual".to_string()),
                        notes: None,
                        created_at: None,
                    },
                )
            })?;
        }
        None => {
            state
                .db
                .with_conn(|conn| prices::clear_price(conn, id, None))?;
        }
    }

    Ok(StatusCode::OK)
}

/// Delete a feed
pub async fn delete_feed(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state.db.with_conn(|conn| feeds::delete_feed(conn, id))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Import report
#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub imported: usize,
    pub errors: usize,
    pub total: usize,
}

/// Import feeds through the legacy cap.ru compatibility endpoint.
pub async fn import_capru(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ImportReport>>, ApiError> {
    use crate::scraper::CapRuScraper;

    // Run blocking local catalog import in a separate thread.
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || {
        let scraper = CapRuScraper::new();
        scraper.import_all(&db, |_current, _total| {
            // Progress callback - could be used for SSE in future
        })
    })
    .await
    .map_err(|e| ApiError {
        error: "task_error".to_string(),
        message: format!("Import task failed: {}", e),
    })?
    .map_err(|e| ApiError {
        error: "scraper_error".to_string(),
        message: format!("Import error: {}", e),
    })?;

    Ok(ApiResponse::new(ImportReport {
        imported: result.imported,
        errors: result.errors,
        total: result.total,
    }))
}

/// Sync feeds from local seed data and cached price sources.
pub async fn sync_feeds(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SyncReport>>, ApiError> {
    let db = state.db.clone();
    let report = crate::scraper::multi_source::sync_all(db)
        .await
        .map_err(|e| ApiError {
            error: "sync_error".to_string(),
            message: e.to_string(),
        })?;
    Ok(ApiResponse::new(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::feed_quality::{
        FeedProfileSectionKey, FeedProfileStatus, FeedSourceKind, FeedTranslationStatus,
    };
    use crate::db::Database;
    use crate::AppConfig;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn test_state() -> AppState {
        let db = Database::new(":memory:").unwrap();
        db.run_migrations().unwrap();

        AppState {
            db: Arc::new(db),
            config: Arc::new(RwLock::new(AppConfig::default())),
        }
    }

    #[tokio::test]
    async fn list_feeds_returns_backend_quality_metadata() {
        let state = test_state();
        state
            .db
            .with_conn(|conn| {
                feeds::insert_feed(
                    conn,
                    &Feed {
                        source_id: Some("seed:normalized-db:n1".to_string()),
                        name_ru: "Пшеница фуражная".to_string(),
                        name_en: Some("Feed wheat".to_string()),
                        category: "grain".to_string(),
                        dry_matter: Some(86.0),
                        koe: Some(1.2),
                        energy_oe_cattle: Some(13.1),
                        crude_protein: Some(125.0),
                        dig_protein_cattle: Some(92.0),
                        crude_fiber: Some(115.0),
                        calcium: Some(0.8),
                        phosphorus: Some(3.6),
                        ..Default::default()
                    },
                )?;
                Ok(())
            })
            .unwrap();

        let Json(response) = list_feeds(
            State(state),
            Query(ListFeedsQuery {
                category: None,
                search: None,
                limit: None,
                offset: None,
                species: None,
                stage_context: None,
            }),
        )
        .await
        .unwrap();

        assert_eq!(response.data.len(), 1);
        let feed = &response.data[0];
        assert_eq!(feed.source_kind, Some(FeedSourceKind::Normalized));
        assert_eq!(feed.translation_status, Some(FeedTranslationStatus::Ready));
        assert_eq!(feed.profile_status, Some(FeedProfileStatus::Complete));
        assert!(feed
            .profile_sections
            .as_ref()
            .is_some_and(|sections| sections.iter().any(|section| {
                section.key == FeedProfileSectionKey::Fiber && section.expected == 1
            })));
    }

    #[tokio::test]
    async fn list_feeds_returns_contextual_suitability_metadata() {
        let state = test_state();
        state
            .db
            .with_conn(|conn| {
                feeds::insert_feed(
                    conn,
                    &Feed {
                        source_id: Some("seed:layer_shell_grit".to_string()),
                        name_ru: "Ракушка кормовая для несушек".to_string(),
                        category: "mineral".to_string(),
                        subcategory: Some("layer_shell_grit".to_string()),
                        calcium: Some(360.0),
                        ..Default::default()
                    },
                )?;
                Ok(())
            })
            .unwrap();

        let Json(response) = list_feeds(
            State(state),
            Query(ListFeedsQuery {
                category: None,
                search: None,
                limit: None,
                offset: None,
                species: Some("poultry".to_string()),
                stage_context: Some("poultry_broiler".to_string()),
            }),
        )
        .await
        .unwrap();

        assert_eq!(response.data.len(), 1);
        assert_eq!(
            response.data[0].suitability_status,
            Some(crate::diet_engine::feed_groups::FeedSuitabilityStatus::Restricted)
        );
        assert_eq!(
            response.data[0]
                .critical_nutrient_audit
                .as_ref()
                .map(|audit| audit.coverage_status),
            Some(FeedProfileStatus::Complete)
        );
    }

    #[tokio::test]
    async fn get_feed_returns_imported_source_audit() {
        let state = test_state();
        let feed_id = state
            .db
            .with_conn(|conn| {
                feeds::insert_feed(
                    conn,
                    &Feed {
                        source_id: Some("https://example.test/feed".to_string()),
                        name_ru: "Imported sample".to_string(),
                        category: "other".to_string(),
                        crude_protein: Some(155.0),
                        ..Default::default()
                    },
                )
            })
            .unwrap();

        let Json(response) = get_feed(
            State(state),
            Path(feed_id),
            Query(FeedContextQuery::default()),
        )
        .await
        .unwrap();

        assert_eq!(response.data.source_kind, Some(FeedSourceKind::Imported));
        assert_eq!(
            response.data.translation_status,
            Some(FeedTranslationStatus::SourceOnly)
        );
        assert_eq!(
            response.data.profile_status,
            Some(FeedProfileStatus::Partial)
        );
    }

    #[tokio::test]
    async fn get_feed_returns_contextual_suitability_metadata() {
        let state = test_state();
        let feed_id = state
            .db
            .with_conn(|conn| {
                feeds::insert_feed(
                    conn,
                    &Feed {
                        source_id: Some("seed:layer_shell_grit".to_string()),
                        name_ru: "Ракушка кормовая для несушек".to_string(),
                        category: "mineral".to_string(),
                        subcategory: Some("layer_shell_grit".to_string()),
                        calcium: Some(360.0),
                        ..Default::default()
                    },
                )
            })
            .unwrap();

        let Json(response) = get_feed(
            State(state),
            Path(feed_id),
            Query(FeedContextQuery {
                species: Some("poultry".to_string()),
                stage_context: Some("poultry_broiler".to_string()),
            }),
        )
        .await
        .unwrap();

        assert_eq!(
            response.data.suitability_status,
            Some(crate::diet_engine::feed_groups::FeedSuitabilityStatus::Restricted)
        );
        assert_eq!(response.data.suitability_max_inclusion_pct, None);
        assert!(response
            .data
            .suitability_notes
            .as_ref()
            .is_some_and(|notes| notes
                .iter()
                .any(|note| note.contains("egg-producing poultry"))));
        assert_eq!(
            response
                .data
                .critical_nutrient_audit
                .as_ref()
                .map(|audit| audit.required_total),
            Some(1)
        );
        assert_eq!(
            response
                .data
                .critical_nutrient_audit
                .as_ref()
                .map(|audit| audit.present_required),
            Some(1)
        );
    }

    #[tokio::test]
    async fn get_feed_returns_contextual_critical_audit_metadata() {
        let state = test_state();
        let feed_id = state
            .db
            .with_conn(|conn| {
                feeds::insert_feed(
                    conn,
                    &Feed {
                        source_id: Some("seed:corn_silage".to_string()),
                        name_ru: "Силос кукурузный".to_string(),
                        category: "silage".to_string(),
                        dry_matter: Some(34.0),
                        energy_oe_cattle: Some(10.4),
                        crude_protein: Some(81.0),
                        dig_protein_cattle: Some(52.0),
                        calcium: Some(2.2),
                        phosphorus: Some(2.1),
                        ..Default::default()
                    },
                )
            })
            .unwrap();

        let Json(response) = get_feed(
            State(state),
            Path(feed_id),
            Query(FeedContextQuery {
                species: Some("cattle".to_string()),
                stage_context: Some("cattle_dairy_early_lact".to_string()),
            }),
        )
        .await
        .unwrap();

        let audit = response
            .data
            .critical_nutrient_audit
            .as_ref()
            .expect("critical audit");
        assert_eq!(audit.coverage_status, FeedProfileStatus::Complete);
        assert!(audit.missing_keys.is_empty());
    }

    #[tokio::test]
    async fn import_capru_compatibility_endpoint_uses_local_feed_authority() {
        let state = test_state();

        let Json(response) = import_capru(State(state.clone())).await.unwrap();

        assert!(response.data.total > 100);
        assert_eq!(response.data.imported, response.data.total);
        assert_eq!(response.data.errors, 0);

        let total = state
            .db
            .with_conn(|conn| feeds::count_feeds(conn, None))
            .unwrap();
        assert!(total > 100);
    }
}
