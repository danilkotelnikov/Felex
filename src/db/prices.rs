use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedPrice {
    pub id: Option<i64>,
    pub feed_id: i64,
    pub region: Option<String>,
    pub price_rubles_per_ton: f64,
    pub price_date: Option<String>,
    pub source: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PriceAnchorSource {
    pub kind: String,
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PriceProvenance {
    pub kind: String,
    pub is_precise_source: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchor_sources: Vec<PriceAnchorSource>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScrapedFeedPrice {
    pub feed_id: i64,
    pub price: f64,
    pub currency: String,
    pub source_url: Option<String>,
    pub confidence_score: Option<i32>,
    pub locale: String,
}

pub fn extract_source_url(notes: Option<&str>) -> Option<String> {
    let notes = notes?.trim();
    let start = notes.find("http://").or_else(|| notes.find("https://"))?;
    let url = notes[start..]
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches([',', ')', ';', '.']);
    if url.is_empty() {
        None
    } else {
        Some(url.to_string())
    }
}

pub fn extract_domain_from_url(raw_url: &str) -> Option<String> {
    let parsed = Url::parse(raw_url).ok()?;
    let hostname = parsed.host_str()?.trim().trim_start_matches("www.");
    if hostname.is_empty() {
        None
    } else {
        Some(hostname.to_string())
    }
}

pub fn extract_source_domain(price: &FeedPrice) -> Option<String> {
    if let Some(domain) = extract_source_url(price.notes.as_deref())
        .as_deref()
        .and_then(extract_domain_from_url)
    {
        return Some(domain);
    }

    let source = price.source.as_deref()?.trim().to_lowercase();
    if source.is_empty() || source == "manual" || source == "seed" || source.starts_with("inferred:")
    {
        return None;
    }

    if let Some(domain) = source.strip_prefix("aggregated:") {
        let trimmed = domain.trim().trim_start_matches("www.");
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }

    if source.contains('.') && !source.contains(' ') {
        return Some(source.trim_start_matches("www.").to_string());
    }

    None
}

pub fn basic_provenance(price: &FeedPrice, is_custom_feed: bool) -> PriceProvenance {
    let source_url = extract_source_url(price.notes.as_deref());
    let source_domain = source_url
        .as_deref()
        .and_then(extract_domain_from_url)
        .or_else(|| extract_source_domain(price));

    match price.source.as_deref() {
        Some("manual") => PriceProvenance {
            kind: "manual".to_string(),
            is_precise_source: false,
            source_url: None,
            source_domain: None,
            benchmark_level: None,
            anchor_count: None,
            anchor_sources: Vec::new(),
        },
        Some("seed") => PriceProvenance {
            kind: "seed".to_string(),
            is_precise_source: false,
            source_url,
            source_domain,
            benchmark_level: None,
            anchor_count: None,
            anchor_sources: Vec::new(),
        },
        Some(source) if source.starts_with("inferred:") => PriceProvenance {
            kind: "benchmark".to_string(),
            is_precise_source: false,
            source_url: None,
            source_domain: None,
            benchmark_level: None,
            anchor_count: None,
            anchor_sources: Vec::new(),
        },
        Some(_) if source_domain.is_some() || source_url.is_some() => PriceProvenance {
            kind: "direct".to_string(),
            is_precise_source: source_url.is_some(),
            source_url,
            source_domain,
            benchmark_level: None,
            anchor_count: None,
            anchor_sources: Vec::new(),
        },
        Some(_) => PriceProvenance {
            kind: "unknown".to_string(),
            is_precise_source: false,
            source_url: None,
            source_domain: None,
            benchmark_level: None,
            anchor_count: None,
            anchor_sources: Vec::new(),
        },
        None if is_custom_feed => PriceProvenance {
            kind: "manual".to_string(),
            is_precise_source: false,
            source_url: None,
            source_domain: None,
            benchmark_level: None,
            anchor_count: None,
            anchor_sources: Vec::new(),
        },
        None => PriceProvenance {
            kind: "seed".to_string(),
            is_precise_source: false,
            source_url,
            source_domain,
            benchmark_level: None,
            anchor_count: None,
            anchor_sources: Vec::new(),
        },
    }
}

fn current_price_date() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn current_history_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn normalize_region(region: Option<&str>) -> Option<String> {
    region
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn feed_price_from_row(row: &Row) -> rusqlite::Result<FeedPrice> {
    Ok(FeedPrice {
        id: row.get(0)?,
        feed_id: row.get(1)?,
        region: row.get(2)?,
        price_rubles_per_ton: row.get(3)?,
        price_date: row.get(4)?,
        source: row.get(5)?,
        notes: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn latest_cached_prices(conn: &Connection, region: Option<&str>) -> Result<Vec<FeedPrice>> {
    let region = normalize_region(region);
    let mut rows = if region.is_some() {
        let mut stmt = conn.prepare(
            "SELECT id, feed_id, region, price_rubles_per_ton, price_date, source, notes, created_at
             FROM feed_prices
             WHERE region = ?1 OR region IS NULL
             ORDER BY feed_id ASC,
                      CASE WHEN region = ?1 THEN 0 ELSE 1 END,
                      COALESCE(price_date, '') DESC,
                      COALESCE(created_at, '') DESC,
                      id DESC",
        )?;
        let rows = stmt
            .query_map(params![region.clone()], feed_price_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, feed_id, region, price_rubles_per_ton, price_date, source, notes, created_at
             FROM feed_prices
             ORDER BY feed_id ASC,
                      COALESCE(price_date, '') DESC,
                      COALESCE(created_at, '') DESC,
                      id DESC",
        )?;
        let rows = stmt
            .query_map([], feed_price_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let mut seen = HashSet::new();
    rows.retain(|row| seen.insert(row.feed_id));
    Ok(rows)
}

fn sync_feed_row(conn: &Connection, price: &FeedPrice) -> Result<()> {
    let price_date = price
        .price_date
        .clone()
        .unwrap_or_else(current_price_date);
    conn.execute(
        "UPDATE feeds
         SET price_per_ton = ?1,
             price_updated_at = ?2,
             region = COALESCE(?3, region),
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ?4",
        params![
            price.price_rubles_per_ton,
            price_date,
            price.region,
            price.feed_id,
        ],
    )?;
    Ok(())
}

fn append_price_history(conn: &Connection, price: &FeedPrice) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO feed_price_history (
            feed_id, region, price, recorded_at
        ) VALUES (?1, ?2, ?3, ?4)",
        params![
            price.feed_id,
            price.region,
            price.price_rubles_per_ton,
            current_history_timestamp(),
        ],
    )?;
    Ok(())
}

fn fallback_feed_price(conn: &Connection, feed_id: i64) -> Result<Option<FeedPrice>> {
    conn.query_row(
        "SELECT price_per_ton, price_updated_at, region
         FROM feeds
         WHERE id = ?1 AND price_per_ton IS NOT NULL",
        params![feed_id],
        |row| {
            Ok(FeedPrice {
                id: None,
                feed_id,
                region: row.get(2)?,
                price_rubles_per_ton: row.get(0)?,
                price_date: row.get(1)?,
                source: None,
                notes: None,
                created_at: None,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn upsert_price(conn: &Connection, price: &FeedPrice) -> Result<()> {
    let region = normalize_region(price.region.as_deref());
    let price_date = price.price_date.clone().unwrap_or_else(current_price_date);

    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id
             FROM feed_prices
             WHERE feed_id = ?1 AND (?2 IS NULL OR region = ?2 OR region IS NULL)
             ORDER BY CASE WHEN region = ?2 THEN 0 ELSE 1 END,
                      COALESCE(price_date, '') DESC,
                      COALESCE(created_at, '') DESC,
                      id DESC
             LIMIT 1",
            params![price.feed_id, region.clone()],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing_id {
        conn.execute(
            "UPDATE feed_prices
             SET region = ?1,
                 price_rubles_per_ton = ?2,
                 price_date = ?3,
                 source = ?4,
                 notes = ?5
             WHERE id = ?6",
            params![
                region.clone(),
                price.price_rubles_per_ton,
                price_date.clone(),
                price.source.clone(),
                price.notes.clone(),
                id,
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO feed_prices (
                feed_id, region, price_rubles_per_ton, price_date, source, notes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                price.feed_id,
                region.clone(),
                price.price_rubles_per_ton,
                price_date.clone(),
                price.source.clone(),
                price.notes.clone(),
            ],
        )?;
    }

    let persisted = FeedPrice {
        id: existing_id,
        feed_id: price.feed_id,
        region,
        price_rubles_per_ton: price.price_rubles_per_ton,
        price_date: Some(price_date),
        source: price.source.clone(),
        notes: price.notes.clone(),
        created_at: price.created_at.clone(),
    };

    append_price_history(conn, &persisted)?;
    sync_feed_row(conn, &persisted)?;
    Ok(())
}

pub fn clear_price(conn: &Connection, feed_id: i64, region: Option<&str>) -> Result<()> {
    let region = normalize_region(region);
    conn.execute(
        "DELETE FROM feed_prices
         WHERE feed_id = ?1 AND (?2 IS NULL OR region = ?2 OR region IS NULL)",
        params![
            feed_id,
            region.clone(),
        ],
    )?;

    conn.execute(
        "UPDATE feeds
         SET price_per_ton = NULL,
             price_updated_at = NULL,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        params![feed_id],
    )?;
    Ok(())
}

pub fn list_prices(conn: &Connection, region: Option<&str>) -> Result<Vec<FeedPrice>> {
    let region = normalize_region(region);
    let mut prices = latest_cached_prices(conn, region.as_deref())?;
    let mut seen_feed_ids = prices.iter().map(|price| price.feed_id).collect::<HashSet<_>>();

    let fallback_rows = match region.as_deref() {
        Some(region) => {
            let mut stmt = conn.prepare(
                "SELECT id, price_per_ton, price_updated_at, region
                 FROM feeds
                 WHERE price_per_ton IS NOT NULL AND (region = ?1 OR region IS NULL)
                 ORDER BY id ASC",
            )?;
            let rows = stmt
                .query_map(params![region], |row| {
                    Ok(FeedPrice {
                        id: None,
                        feed_id: row.get(0)?,
                        region: row.get(3)?,
                        price_rubles_per_ton: row.get(1)?,
                        price_date: row.get(2)?,
                        source: None,
                        notes: None,
                        created_at: None,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, price_per_ton, price_updated_at, region
                 FROM feeds
                 WHERE price_per_ton IS NOT NULL
                 ORDER BY id ASC",
            )?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(FeedPrice {
                        id: None,
                        feed_id: row.get(0)?,
                        region: row.get(3)?,
                        price_rubles_per_ton: row.get(1)?,
                        price_date: row.get(2)?,
                        source: None,
                        notes: None,
                        created_at: None,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        }
    };

    for price in fallback_rows {
        if seen_feed_ids.insert(price.feed_id) {
            prices.push(price);
        }
    }

    prices.sort_by(|left, right| left.feed_id.cmp(&right.feed_id));
    Ok(prices)
}

pub fn get_price(conn: &Connection, feed_id: i64, region: Option<&str>) -> Result<Option<FeedPrice>> {
    let region = normalize_region(region);
    let price = conn
        .query_row(
            "SELECT id, feed_id, region, price_rubles_per_ton, price_date, source, notes, created_at
             FROM feed_prices
             WHERE feed_id = ?1 AND (?2 IS NULL OR region = ?2 OR region IS NULL)
             ORDER BY CASE WHEN region = ?2 THEN 0 ELSE 1 END,
                      COALESCE(price_date, '') DESC,
                      COALESCE(created_at, '') DESC,
                      id DESC
             LIMIT 1",
            params![feed_id, region.clone()],
            feed_price_from_row,
        )
        .optional()?;

    match price {
        Some(price) => Ok(Some(price)),
        None => fallback_feed_price(conn, feed_id),
    }
}

pub fn get_price_history(conn: &Connection, feed_id: i64, region: Option<&str>, days: Option<i32>) -> Result<Vec<FeedPrice>> {
    let region = normalize_region(region);
    let cutoff = days.map(|value| (chrono::Utc::now() - chrono::Duration::days(value as i64)).to_rfc3339());

    let history = match (region.as_deref(), cutoff.as_deref()) {
        (Some(region), Some(cutoff)) => {
            let mut stmt = conn.prepare(
                "SELECT NULL, feed_id, region, price, substr(recorded_at, 1, 10), NULL, NULL, recorded_at
                 FROM feed_price_history
                 WHERE feed_id = ?1 AND (region = ?2 OR region IS NULL) AND recorded_at >= ?3
                 ORDER BY recorded_at DESC",
            )?;
            let rows = stmt
                .query_map(params![feed_id, region, cutoff], feed_price_from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        }
        (Some(region), None) => {
            let mut stmt = conn.prepare(
                "SELECT NULL, feed_id, region, price, substr(recorded_at, 1, 10), NULL, NULL, recorded_at
                 FROM feed_price_history
                 WHERE feed_id = ?1 AND (region = ?2 OR region IS NULL)
                 ORDER BY recorded_at DESC",
            )?;
            let rows = stmt
                .query_map(params![feed_id, region], feed_price_from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        }
        (None, Some(cutoff)) => {
            let mut stmt = conn.prepare(
                "SELECT NULL, feed_id, region, price, substr(recorded_at, 1, 10), NULL, NULL, recorded_at
                 FROM feed_price_history
                 WHERE feed_id = ?1 AND recorded_at >= ?2
                 ORDER BY recorded_at DESC",
            )?;
            let rows = stmt
                .query_map(params![feed_id, cutoff], feed_price_from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        }
        (None, None) => {
            let mut stmt = conn.prepare(
                "SELECT NULL, feed_id, region, price, substr(recorded_at, 1, 10), NULL, NULL, recorded_at
                 FROM feed_price_history
                 WHERE feed_id = ?1
                 ORDER BY recorded_at DESC",
            )?;
            let rows = stmt
                .query_map(params![feed_id], feed_price_from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        }
    };

    Ok(history)
}

pub fn repair_feed_price_cache(conn: &Connection) -> Result<usize> {
    let prices = latest_cached_prices(conn, None)?;
    let mut repaired = 0usize;

    for price in &prices {
        sync_feed_row(conn, price)?;
        repaired += 1;
    }

    Ok(repaired)
}

pub fn insert_scraped_price(conn: &Connection, price: &ScrapedFeedPrice) -> Result<()> {
    conn.execute(
        "INSERT INTO scraped_feed_prices (
            feed_id, price, currency, source_url, confidence_score, locale
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(feed_id, locale) DO UPDATE SET
            price = excluded.price,
            currency = excluded.currency,
            source_url = excluded.source_url,
            confidence_score = excluded.confidence_score,
            timestamp = CURRENT_TIMESTAMP",
        params![
            price.feed_id,
            price.price,
            price.currency,
            price.source_url,
            price.confidence_score,
            price.locale,
        ],
    )?;
    Ok(())
}

pub fn get_scraped_price(conn: &Connection, feed_id: i64, locale: &str) -> Result<Option<ScrapedFeedPrice>> {
    conn.query_row(
        "SELECT feed_id, price, currency, source_url, confidence_score, locale 
         FROM scraped_feed_prices 
         WHERE feed_id = ?1 AND locale = ?2",
        params![feed_id, locale],
        |row| {
            Ok(ScrapedFeedPrice {
                feed_id: row.get(0)?,
                price: row.get(1)?,
                currency: row.get(2)?,
                source_url: row.get(3)?,
                confidence_score: row.get(4)?,
                locale: row.get(5)?,
            })
        },
    ).optional().map_err(Into::into)
}
