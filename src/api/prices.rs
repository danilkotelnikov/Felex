//! Price API endpoints

use crate::db::{feeds, prices};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct FeedPriceResponse {
    #[serde(flatten)]
    pub price: prices::FeedPrice,
    pub provenance: prices::PriceProvenance,
}

#[derive(Debug, Serialize)]
pub struct JitFeedPriceResponse {
    pub feed_id: i64,
    pub price: f64,
    pub currency: &'static str,
    pub source_url: Option<String>,
    pub confidence_score: Option<i32>,
    pub locale: String,
    pub price_date: Option<String>,
    pub source: Option<String>,
    pub region: Option<String>,
    pub provenance: prices::PriceProvenance,
}

fn load_price_context(
    conn: &rusqlite::Connection,
    region: Option<&str>,
) -> anyhow::Result<(
    Vec<feeds::Feed>,
    HashMap<i64, feeds::Feed>,
    HashMap<i64, prices::FeedPrice>,
)> {
    let all_feeds = feeds::list_feeds(conn, None, None, None, None)?;
    let feed_by_id = all_feeds
        .iter()
        .filter_map(|feed| feed.id.map(|id| (id, feed.clone())))
        .collect::<HashMap<_, _>>();
    let current_prices = prices::list_prices(conn, region)?
        .into_iter()
        .map(|price| (price.feed_id, price))
        .collect::<HashMap<_, _>>();
    Ok((all_feeds, feed_by_id, current_prices))
}

fn build_price_response(
    feed: &feeds::Feed,
    price: prices::FeedPrice,
    all_feeds: &[feeds::Feed],
    current_prices: &HashMap<i64, prices::FeedPrice>,
) -> FeedPriceResponse {
    let provenance =
        crate::scraper::multi_source::resolve_price_provenance(feed, &price, all_feeds, current_prices);
    FeedPriceResponse { price, provenance }
}

fn build_jit_price_payload(
    feed: &feeds::Feed,
    price: prices::FeedPrice,
    locale: &str,
    all_feeds: &[feeds::Feed],
    current_prices: &HashMap<i64, prices::FeedPrice>,
) -> JitFeedPriceResponse {
    let provenance =
        crate::scraper::multi_source::resolve_price_provenance(feed, &price, all_feeds, current_prices);
    JitFeedPriceResponse {
        feed_id: price.feed_id,
        price: price.price_rubles_per_ton,
        currency: "RUB",
        source_url: prices::extract_source_url(price.notes.as_deref()),
        confidence_score: None,
        locale: locale.to_string(),
        price_date: price.price_date,
        source: price.source,
        region: price.region,
        provenance,
    }
}

/// Query params for listing prices
#[derive(Debug, Deserialize)]
pub struct ListPricesParams {
    pub region: Option<String>,
}

/// List all feed prices
pub async fn list_prices(
    State(state): State<AppState>,
    Query(params): Query<ListPricesParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let price_list = state
        .db
        .with_conn(|conn| {
            let (all_feeds, feed_by_id, current_prices) =
                load_price_context(conn, params.region.as_deref())?;
            let mut rows = current_prices
                .values()
                .cloned()
                .filter_map(|price| {
                    let feed_id = price.feed_id;
                    let feed = feed_by_id.get(&feed_id)?;
                    Some(build_price_response(feed, price, &all_feeds, &current_prices))
                })
                .collect::<Vec<_>>();
            rows.sort_by(|left, right| left.price.feed_id.cmp(&right.price.feed_id));
            Ok(rows)
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "data": price_list })))
}

/// Update price for a feed
#[derive(Debug, Deserialize)]
pub struct UpdatePriceBody {
    pub price_rubles_per_ton: f64,
    pub region: Option<String>,
    pub source: Option<String>,
}

pub async fn update_price(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
    Json(body): Json<UpdatePriceBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let price = prices::FeedPrice {
        id: None,
        feed_id,
        region: body.region,
        price_rubles_per_ton: body.price_rubles_per_ton,
        price_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
        source: body.source.or(Some("manual".to_string())),
        notes: None,
        created_at: None,
    };

    state
        .db
        .with_conn(|conn| prices::upsert_price(conn, &price))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Fetch prices from web sources
#[derive(Debug, Serialize)]
pub struct FetchPricesResult {
    pub updated: usize,
    pub errors: usize,
}

pub async fn fetch_prices(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let report = crate::scraper::multi_source::sync_all(state.db.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "data": {
            "updated": report.prices_updated,
            "errors": report.prices_total.saturating_sub(report.prices_updated)
        }
    })))
}

/// Get price history for a feed
pub async fn get_price_history(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let history = state
        .db
        .with_conn(|conn| prices::get_price_history(conn, feed_id, None, Some(90)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "data": history })))
}

#[derive(Debug, Deserialize)]
pub struct JitPriceParams {
    pub locale: Option<String>,
}

pub async fn get_jit_feed_price(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
    Query(params): Query<JitPriceParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let locale = params.locale.unwrap_or_else(|| "RU".to_string());

    if let Some((feed, price, all_feeds, current_prices)) = state
        .db
        .with_conn(|conn| {
            let price = prices::get_price(conn, feed_id, None)?;
            let Some(price) = price else {
                return Ok(None);
            };
            let (all_feeds, feed_by_id, current_prices) = load_price_context(conn, None)?;
            let feed = feed_by_id.get(&feed_id).cloned();
            Ok(feed.map(|feed| (feed, price, all_feeds, current_prices)))
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        return Ok(Json(serde_json::json!({
            "data": build_jit_price_payload(&feed, price, &locale, &all_feeds, &current_prices)
        })));
    }

    let feed_exists = state
        .db
        .with_conn(|conn| Ok(feeds::get_feed(conn, feed_id)?.is_some()))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        ;
    if !feed_exists {
        return Err((StatusCode::NOT_FOUND, "Feed not found".to_string()));
    }

    crate::scraper::multi_source::sync_all(state.db.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let refreshed_price = state
        .db
        .with_conn(|conn| {
            let price = prices::get_price(conn, feed_id, None)?;
            let Some(price) = price else {
                return Ok(None);
            };
            let (all_feeds, feed_by_id, current_prices) = load_price_context(conn, None)?;
            let feed = feed_by_id.get(&feed_id).cloned();
            Ok(feed.map(|feed| (feed, price, all_feeds, current_prices)))
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match refreshed_price {
        Some((feed, price, all_feeds, current_prices)) => Ok(Json(serde_json::json!({
            "data": build_jit_price_payload(&feed, price, &locale, &all_feeds, &current_prices)
        }))),
        None => Err((StatusCode::NOT_FOUND, "No price available".to_string())),
    }
}
