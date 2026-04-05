use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use felex::{init_app_state, AppConfig};
use felex::api::prices::{get_jit_feed_price, JitPriceParams};
use felex::db::feeds::{self, Feed};
use felex::db::prices::{self, FeedPrice};
use felex::scraper;

#[tokio::test]
async fn test_get_jit_price_returns_not_found_for_invalid_feed() {
    let config = AppConfig {
        database_path: ":memory:".to_string(),
        server_port: 0,
        static_dir: None,
        cors_origins: vec![],
        workspace_root: ".".to_string(),
    };
    
    let state = init_app_state(config).await.unwrap();

    let result = get_jit_feed_price(
        State(state),
        Path(999999), // non-existent feed id
        Query(JitPriceParams { locale: Some("RU".to_string()) }),
    ).await;

    assert!(result.is_err());
    if let Err((status, _msg)) = result {
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn test_get_jit_price_returns_canonical_cached_price() {
    let config = AppConfig {
        database_path: ":memory:".to_string(),
        server_port: 0,
        static_dir: None,
        cors_origins: vec![],
        workspace_root: ".".to_string(),
    };

    let state = init_app_state(config).await.unwrap();

    let feed_id = state
        .db
        .with_conn(|conn| {
            feeds::insert_feed(
                conn,
                &Feed {
                    name_ru: "Canonical price feed".to_string(),
                    category: "grain".to_string(),
                    ..Default::default()
                },
            )
        })
        .unwrap();

    state
        .db
        .with_conn(|conn| {
            prices::upsert_price(
                conn,
                &FeedPrice {
                    id: None,
                    feed_id,
                    region: Some("test-region".to_string()),
                    price_rubles_per_ton: 18_500.0,
                    price_date: Some("2026-03-22".to_string()),
                    source: Some("manual".to_string()),
                    notes: Some("Price source: https://example.test/canonical".to_string()),
                    created_at: None,
                },
            )
        })
        .unwrap();

    let Json(body) = get_jit_feed_price(
        State(state),
        Path(feed_id),
        Query(JitPriceParams { locale: Some("RU".to_string()) }),
    )
    .await
    .unwrap();

    assert_eq!(body["data"]["feed_id"].as_i64(), Some(feed_id));
    assert_eq!(body["data"]["price"].as_f64(), Some(18_500.0));
    assert_eq!(body["data"]["currency"].as_str(), Some("RUB"));
    assert_eq!(
        body["data"]["source_url"].as_str(),
        Some("https://example.test/canonical")
    );
    assert_eq!(body["data"]["region"].as_str(), Some("test-region"));
    assert_eq!(body["data"]["provenance"]["kind"].as_str(), Some("manual"));
}

#[tokio::test]
async fn test_get_jit_price_returns_benchmark_provenance_for_inferred_price() {
    let config = AppConfig {
        database_path: ":memory:".to_string(),
        server_port: 0,
        static_dir: None,
        cors_origins: vec![],
        workspace_root: ".".to_string(),
    };

    let state = init_app_state(config).await.unwrap();

    let anchor_feed_id = state
        .db
        .with_conn(|conn| {
            feeds::insert_feed(
                conn,
                &Feed {
                    name_ru: "Anchor barley".to_string(),
                    category: "grain".to_string(),
                    subcategory: Some("barley".to_string()),
                    ..Default::default()
                },
            )
        })
        .unwrap();

    let inferred_feed_id = state
        .db
        .with_conn(|conn| {
            feeds::insert_feed(
                conn,
                &Feed {
                    name_ru: "Derived barley".to_string(),
                    category: "grain".to_string(),
                    subcategory: Some("barley".to_string()),
                    ..Default::default()
                },
            )
        })
        .unwrap();

    state
        .db
        .with_conn(|conn| {
            prices::upsert_price(
                conn,
                &FeedPrice {
                    id: None,
                    feed_id: anchor_feed_id,
                    region: Some("test-region".to_string()),
                    price_rubles_per_ton: 16_200.0,
                    price_date: Some("2026-03-22".to_string()),
                    source: Some("manual".to_string()),
                    notes: None,
                    created_at: None,
                },
            )
        })
        .unwrap();

    scraper::refresh_inferred_prices(state.db.as_ref()).unwrap();

    let Json(body) = get_jit_feed_price(
        State(state),
        Path(inferred_feed_id),
        Query(JitPriceParams { locale: Some("RU".to_string()) }),
    )
    .await
    .unwrap();

    assert_eq!(body["data"]["price"].as_f64(), Some(16_200.0));
    assert_eq!(body["data"]["provenance"]["kind"].as_str(), Some("benchmark"));
    assert_eq!(
        body["data"]["provenance"]["benchmark_level"].as_str(),
        Some("subcategory")
    );
    assert_eq!(body["data"]["provenance"]["anchor_count"].as_u64(), Some(1));
    assert_eq!(
        body["data"]["provenance"]["anchor_sources"][0]["kind"].as_str(),
        Some("manual")
    );
}
