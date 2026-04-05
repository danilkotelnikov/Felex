use anyhow::Result;
use felex::db::Database;
use felex::db::feeds::{self, Feed};
use felex::db::prices::{self, FeedPrice, ScrapedFeedPrice, get_scraped_price, insert_scraped_price};
use felex::scraper;
use rusqlite::params;

fn insert_test_feed(db: &Database, name_ru: &str) -> Result<i64> {
    db.with_conn(|conn| {
        feeds::insert_feed(
            conn,
            &Feed {
                name_ru: name_ru.to_string(),
                category: "concentrate".to_string(),
                ..Default::default()
            },
        )
    })
}

#[test]
fn test_insert_and_retrieve_scraped_price() -> Result<()> {
    let db = Database::new(":memory:")?;
    db.run_migrations()?;

    let price = ScrapedFeedPrice {
        feed_id: 1,
        price: 15.5,
        currency: "RUB".to_string(),
        source_url: Some("http://example.com".to_string()),
        confidence_score: Some(8),
        locale: "RU".to_string(),
    };

    // Assuming we can pass a connection to insert/get
    db.with_conn(|conn| {
        insert_scraped_price(conn, &price)?;
        
        let retrieved = get_scraped_price(conn, 1, "RU")?.expect("Should find price");
        assert_eq!(retrieved.price, 15.5);
        assert_eq!(retrieved.currency, "RUB");
        assert_eq!(retrieved.confidence_score, Some(8));
        Ok(())
    })
}

#[test]
fn test_upsert_price_updates_feed_row_and_history() -> Result<()> {
    let db = Database::new(":memory:")?;
    db.run_migrations()?;
    let feed_id = insert_test_feed(&db, "Test price feed")?;

    db.with_conn(|conn| {
        prices::upsert_price(
            conn,
            &FeedPrice {
                id: None,
                feed_id,
                region: Some("test-region".to_string()),
                price_rubles_per_ton: 12_345.0,
                price_date: Some("2026-03-22".to_string()),
                source: Some("manual".to_string()),
                notes: Some("Price source: https://example.test/manual".to_string()),
                created_at: None,
            },
        )
    })?;

    db.with_conn(|conn| {
        let price = prices::get_price(conn, feed_id, None)?.expect("price should exist");
        assert_eq!(price.price_rubles_per_ton, 12_345.0);
        assert_eq!(price.source.as_deref(), Some("manual"));

        let feed = feeds::get_feed(conn, feed_id)?.expect("feed should exist");
        assert_eq!(feed.price_per_ton, Some(12_345.0));
        assert_eq!(feed.price_updated_at.as_deref(), Some("2026-03-22"));
        assert_eq!(feed.region.as_deref(), Some("test-region"));

        let history = prices::get_price_history(conn, feed_id, None, None)?;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].price_rubles_per_ton, 12_345.0);
        Ok(())
    })
}

#[test]
fn test_clear_price_removes_current_feed_price() -> Result<()> {
    let db = Database::new(":memory:")?;
    db.run_migrations()?;
    let feed_id = insert_test_feed(&db, "Clearable price feed")?;

    db.with_conn(|conn| {
        prices::upsert_price(
            conn,
            &FeedPrice {
                id: None,
                feed_id,
                region: None,
                price_rubles_per_ton: 9_000.0,
                price_date: Some("2026-03-22".to_string()),
                source: Some("manual".to_string()),
                notes: None,
                created_at: None,
            },
        )?;
        prices::clear_price(conn, feed_id, None)
    })?;

    db.with_conn(|conn| {
        assert!(prices::get_price(conn, feed_id, None)?.is_none());
        let feed = feeds::get_feed(conn, feed_id)?.expect("feed should exist");
        assert_eq!(feed.price_per_ton, None);
        assert_eq!(feed.price_updated_at, None);
        Ok(())
    })
}

#[test]
fn test_repair_feed_price_cache_backfills_existing_cached_rows() -> Result<()> {
    let db = Database::new(":memory:")?;
    db.run_migrations()?;
    let feed_id = insert_test_feed(&db, "Legacy cached feed")?;

    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO feed_prices (
                feed_id, region, price_rubles_per_ton, price_date, source, notes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                feed_id,
                "legacy-region",
                7_777.0,
                "2026-03-22",
                "legacy",
                "Price source: https://example.test/legacy",
            ],
        )?;
        Ok(())
    })?;

    db.with_conn(|conn| {
        let feed = feeds::get_feed(conn, feed_id)?.expect("feed should exist");
        assert_eq!(feed.price_per_ton, None);
        Ok(())
    })?;

    let repaired = db.with_conn(|conn| prices::repair_feed_price_cache(conn))?;
    assert_eq!(repaired, 1);

    db.with_conn(|conn| {
        let feed = feeds::get_feed(conn, feed_id)?.expect("feed should exist");
        assert_eq!(feed.price_per_ton, Some(7_777.0));
        assert_eq!(feed.price_updated_at.as_deref(), Some("2026-03-22"));
        assert_eq!(feed.region.as_deref(), Some("legacy-region"));
        Ok(())
    })
}

#[test]
fn test_list_prices_includes_feed_row_without_cached_price_record() -> Result<()> {
    let db = Database::new(":memory:")?;
    db.run_migrations()?;
    let feed_id = insert_test_feed(&db, "Feed row fallback price")?;

    db.with_conn(|conn| {
        conn.execute(
            "UPDATE feeds
             SET price_per_ton = ?1,
                 price_updated_at = ?2,
                 region = ?3
             WHERE id = ?4",
            params![8_888.0, "2026-03-22", "feed-row-region", feed_id],
        )?;
        Ok(())
    })?;

    db.with_conn(|conn| {
        let prices_list = prices::list_prices(conn, None)?;
        let price = prices_list
            .into_iter()
            .find(|item| item.feed_id == feed_id)
            .expect("feed-row fallback should be listed");

        assert_eq!(price.price_rubles_per_ton, 8_888.0);
        assert_eq!(price.price_date.as_deref(), Some("2026-03-22"));
        assert_eq!(price.region.as_deref(), Some("feed-row-region"));
        Ok(())
    })
}

#[test]
fn test_refresh_inferred_prices_backfills_missing_catalog_prices() -> Result<()> {
    let db = Database::new(":memory:")?;
    db.run_migrations()?;

    let anchor_id = db.with_conn(|conn| {
        feeds::insert_feed(
            conn,
            &Feed {
                name_ru: "Anchor wheat".to_string(),
                category: "grain".to_string(),
                subcategory: Some("wheat".to_string()),
                ..Default::default()
            },
        )
    })?;

    let inferred_id = db.with_conn(|conn| {
        feeds::insert_feed(
            conn,
            &Feed {
                name_ru: "Benchmark wheat".to_string(),
                category: "grain".to_string(),
                subcategory: Some("wheat".to_string()),
                ..Default::default()
            },
        )
    })?;

    db.with_conn(|conn| {
        prices::upsert_price(
            conn,
            &FeedPrice {
                id: None,
                feed_id: anchor_id,
                region: Some("test-region".to_string()),
                price_rubles_per_ton: 15_500.0,
                price_date: Some("2026-03-22".to_string()),
                source: Some("manual".to_string()),
                notes: Some("Price source: https://example.test/anchor".to_string()),
                created_at: None,
            },
        )
    })?;

    let refreshed = scraper::refresh_inferred_prices(&db)?;
    assert_eq!(refreshed, 1);

    db.with_conn(|conn| {
        let inferred = prices::get_price(conn, inferred_id, None)?
            .expect("benchmark inference should create a missing price");
        assert_eq!(inferred.price_rubles_per_ton, 15_500.0);
        assert_eq!(inferred.source.as_deref(), Some("inferred:grain"));
        assert!(inferred.price_date.is_some());

        let feed = feeds::get_feed(conn, inferred_id)?.expect("feed should exist");
        assert_eq!(feed.price_per_ton, Some(15_500.0));
        assert!(feed.price_updated_at.is_some());
        Ok(())
    })
}
