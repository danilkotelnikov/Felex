//! Multi-source feed synchronization orchestrator

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::db::feeds;
use crate::db::prices::{self, FeedPrice, PriceAnchorSource, PriceProvenance};
use crate::db::Database;
use crate::scraper::normalized_feed_db;
use crate::scraper::price_fetcher::{map_prices_to_feeds, FetchedPrice, PriceFetcher};

#[derive(Debug, Clone)]
struct PriceBenchmark {
    price_per_ton: f64,
    anchor_count: usize,
    anchor_sources: Vec<PriceAnchorSource>,
}

#[derive(Debug, Default)]
struct PriceBenchmarks {
    subcategory: HashMap<String, PriceBenchmark>,
    category: HashMap<String, PriceBenchmark>,
    family: HashMap<String, PriceBenchmark>,
    global: Option<PriceBenchmark>,
}

#[derive(Debug, Default)]
struct BenchmarkAccumulator {
    values: Vec<f64>,
    anchor_sources: HashMap<(String, String), usize>,
}

#[derive(Debug, Clone)]
struct BenchmarkMatch {
    price_per_ton: f64,
    inferred_source: String,
    level: &'static str,
    anchor_count: usize,
    anchor_sources: Vec<PriceAnchorSource>,
}

fn current_price_date() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn normalize_price_label(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('ё', "е")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_positive_price(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn is_inferred_source(source: Option<&str>) -> bool {
    source.is_some_and(|value| value.starts_with("inferred:"))
}

fn is_manual_source(source: Option<&str>) -> bool {
    matches!(source, Some("manual"))
}

fn benchmark_family(category: &str) -> &'static str {
    match category {
        "roughage" | "silage" | "succulent" | "green_forage" => "forage",
        "grain" | "concentrate" | "compound_feed" | "byproduct" => "energy",
        "oilseed_meal" | "protein" | "animal_origin" => "protein",
        "mineral" | "additive" => "mineral",
        "premix" => "premix",
        "oil_fat" => "fat",
        _ => "other",
    }
}

fn median_price(values: &[f64]) -> Option<f64> {
    let mut sorted = values
        .iter()
        .copied()
        .filter(|value| is_positive_price(*value))
        .collect::<Vec<_>>();
    if sorted.is_empty() {
        return None;
    }

    sorted.sort_by(|left, right| left.total_cmp(right));
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        Some(sorted[middle])
    } else {
        Some((sorted[middle - 1] + sorted[middle]) / 2.0)
    }
}

fn benchmark_anchor_source(feed: &feeds::Feed, price: &FeedPrice) -> Option<(String, String)> {
    if let Some(domain) = prices::extract_source_domain(price) {
        return Some(("domain".to_string(), domain));
    }

    match price.source.as_deref() {
        Some("manual") => Some(("manual".to_string(), "manual".to_string())),
        Some("seed") => Some(("seed".to_string(), "seed".to_string())),
        Some(source) if source.starts_with("inferred:") => None,
        Some(source) if !source.trim().is_empty() => Some(("other".to_string(), source.to_string())),
        Some(_) => None,
        None if feed.is_custom => Some(("manual".to_string(), "manual".to_string())),
        None => Some(("seed".to_string(), "seed".to_string())),
    }
}

fn push_benchmark_value(accumulator: &mut BenchmarkAccumulator, feed: &feeds::Feed, price: &FeedPrice) {
    accumulator.values.push(price.price_rubles_per_ton);
    if let Some((kind, label)) = benchmark_anchor_source(feed, price) {
        *accumulator.anchor_sources.entry((kind, label)).or_insert(0) += 1;
    }
}

fn summarize_anchor_sources(
    counts: HashMap<(String, String), usize>,
) -> Vec<PriceAnchorSource> {
    let mut items = counts
        .into_iter()
        .map(|((kind, label), count)| PriceAnchorSource { kind, label, count })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    items
}

fn summarize_benchmarks(
    groups: HashMap<String, BenchmarkAccumulator>,
) -> HashMap<String, PriceBenchmark> {
    groups
        .into_iter()
        .filter_map(|(key, accumulator)| {
            median_price(&accumulator.values).map(|price_per_ton| {
                (
                    key,
                    PriceBenchmark {
                        price_per_ton,
                        anchor_count: accumulator.values.len(),
                        anchor_sources: summarize_anchor_sources(accumulator.anchor_sources),
                    },
                )
            })
        })
        .collect()
}

fn collect_current_prices(
    conn: &rusqlite::Connection,
    all_feeds: &[feeds::Feed],
) -> Result<HashMap<i64, FeedPrice>> {
    let mut current = prices::list_prices(conn, None)?
        .into_iter()
        .map(|price| (price.feed_id, price))
        .collect::<HashMap<_, _>>();

    for feed in all_feeds {
        let Some(feed_id) = feed.id else {
            continue;
        };

        if current.contains_key(&feed_id) {
            continue;
        }

        let Some(price_per_ton) = feed.price_per_ton.filter(|value| is_positive_price(*value)) else {
            continue;
        };

        current.insert(
            feed_id,
            FeedPrice {
                id: None,
                feed_id,
                region: feed.region.clone(),
                price_rubles_per_ton: price_per_ton,
                price_date: feed.price_updated_at.clone(),
                source: if feed.is_custom {
                    Some("manual".to_string())
                } else {
                    None
                },
                notes: feed.notes.clone(),
                created_at: None,
            },
        );
    }

    Ok(current)
}

fn should_skip_direct_refresh(feed: &feeds::Feed, existing: Option<&FeedPrice>) -> bool {
    if existing.is_some_and(|price| is_manual_source(price.source.as_deref())) {
        return true;
    }

    existing.is_none() && feed.is_custom && feed.price_per_ton.is_some_and(is_positive_price)
}

fn should_skip_inferred_refresh(feed: &feeds::Feed, existing: Option<&FeedPrice>) -> bool {
    if let Some(existing) = existing {
        return !is_inferred_source(existing.source.as_deref());
    }

    feed.is_custom && feed.price_per_ton.is_some_and(is_positive_price)
}

fn is_same_price(existing: Option<&FeedPrice>, candidate: &FeedPrice) -> bool {
    existing.is_some_and(|price| {
        (price.price_rubles_per_ton - candidate.price_rubles_per_ton).abs() < 1e-6
            && price.region == candidate.region
            && price.source == candidate.source
            && price.notes == candidate.notes
    })
}

fn build_price_benchmarks(
    all_feeds: &[feeds::Feed],
    current_prices: &HashMap<i64, FeedPrice>,
) -> PriceBenchmarks {
    let mut subcategory_groups: HashMap<String, BenchmarkAccumulator> = HashMap::new();
    let mut category_groups: HashMap<String, BenchmarkAccumulator> = HashMap::new();
    let mut family_groups: HashMap<String, BenchmarkAccumulator> = HashMap::new();
    let mut global_accumulator = BenchmarkAccumulator::default();

    for feed in all_feeds {
        let Some(feed_id) = feed.id else {
            continue;
        };
        let Some(price) = current_prices.get(&feed_id) else {
            continue;
        };
        if is_inferred_source(price.source.as_deref()) || !is_positive_price(price.price_rubles_per_ton)
        {
            continue;
        }

        push_benchmark_value(&mut global_accumulator, feed, price);

        if let Some(subcategory) = feed
            .subcategory
            .as_deref()
            .map(normalize_price_label)
            .filter(|value| !value.is_empty())
        {
            let entry = subcategory_groups.entry(subcategory).or_default();
            push_benchmark_value(entry, feed, price);
        }

        let category_key = normalize_price_label(&feed.category);
        let category_entry = category_groups.entry(category_key).or_default();
        push_benchmark_value(category_entry, feed, price);

        let family_entry = family_groups
            .entry(benchmark_family(&feed.category).to_string())
            .or_default();
        push_benchmark_value(family_entry, feed, price);
    }

    PriceBenchmarks {
        subcategory: summarize_benchmarks(subcategory_groups),
        category: summarize_benchmarks(category_groups),
        family: summarize_benchmarks(family_groups),
        global: median_price(&global_accumulator.values).map(|price_per_ton| PriceBenchmark {
            price_per_ton,
            anchor_count: global_accumulator.values.len(),
            anchor_sources: summarize_anchor_sources(global_accumulator.anchor_sources),
        }),
    }
}

fn resolve_benchmark_match(feed: &feeds::Feed, benchmarks: &PriceBenchmarks) -> Option<BenchmarkMatch> {
    let inferred_category_source = format!("inferred:{}", feed.category);

    if let Some(subcategory_key) = feed
        .subcategory
        .as_deref()
        .map(normalize_price_label)
        .filter(|value| !value.is_empty())
    {
        if let Some(benchmark) = benchmarks.subcategory.get(&subcategory_key) {
            return Some(BenchmarkMatch {
                price_per_ton: benchmark.price_per_ton,
                inferred_source: inferred_category_source,
                level: "subcategory",
                anchor_count: benchmark.anchor_count,
                anchor_sources: benchmark.anchor_sources.clone(),
            });
        }
    }

    let category_key = normalize_price_label(&feed.category);
    if let Some(benchmark) = benchmarks.category.get(&category_key) {
        return Some(BenchmarkMatch {
            price_per_ton: benchmark.price_per_ton,
            inferred_source: inferred_category_source,
            level: "category",
            anchor_count: benchmark.anchor_count,
            anchor_sources: benchmark.anchor_sources.clone(),
        });
    }

    let family = benchmark_family(&feed.category);
    if let Some(benchmark) = benchmarks.family.get(family) {
        return Some(BenchmarkMatch {
            price_per_ton: benchmark.price_per_ton,
            inferred_source: format!("inferred:{family}"),
            level: "family",
            anchor_count: benchmark.anchor_count,
            anchor_sources: benchmark.anchor_sources.clone(),
        });
    }

    benchmarks.global.as_ref().map(|benchmark| BenchmarkMatch {
        price_per_ton: benchmark.price_per_ton,
        inferred_source: "inferred:global".to_string(),
        level: "global",
        anchor_count: benchmark.anchor_count,
        anchor_sources: benchmark.anchor_sources.clone(),
    })
}

fn infer_benchmark_price(feed: &feeds::Feed, benchmarks: &PriceBenchmarks) -> Option<FeedPrice> {
    let feed_id = feed.id?;
    let price_date = Some(current_price_date());
    let benchmark = resolve_benchmark_match(feed, benchmarks)?;

    Some(FeedPrice {
        id: None,
        feed_id,
        region: feed.region.clone(),
        price_rubles_per_ton: benchmark.price_per_ton,
        price_date,
        source: Some(benchmark.inferred_source),
        notes: None,
        created_at: None,
    })
}

pub fn resolve_price_provenance(
    feed: &feeds::Feed,
    price: &FeedPrice,
    all_feeds: &[feeds::Feed],
    current_prices: &HashMap<i64, FeedPrice>,
) -> PriceProvenance {
    if is_inferred_source(price.source.as_deref()) {
        let benchmarks = build_price_benchmarks(all_feeds, current_prices);
        if let Some(benchmark) = resolve_benchmark_match(feed, &benchmarks) {
            return PriceProvenance {
                kind: "benchmark".to_string(),
                is_precise_source: false,
                source_url: None,
                source_domain: None,
                benchmark_level: Some(benchmark.level.to_string()),
                anchor_count: Some(benchmark.anchor_count),
                anchor_sources: benchmark.anchor_sources,
            };
        }
    }

    prices::basic_provenance(price, feed.is_custom)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub feeds_imported: usize,
    pub feeds_errors: usize,
    pub feeds_total: usize,
    pub prices_updated: usize,
    pub prices_total: usize,
    pub seed_imported: usize,
}

fn refresh_price_cache(
    conn: &rusqlite::Connection,
    fetched_prices: &[FetchedPrice],
) -> Result<usize> {
    let all_feeds = feeds::list_feeds(conn, None, None, None, None)?;
    let benchmark_inputs = all_feeds.clone();
    let feed_by_id = all_feeds
        .iter()
        .filter_map(|feed| feed.id.map(|id| (id, feed)))
        .collect::<HashMap<_, _>>();
    let feed_name_map: HashMap<String, i64> = all_feeds
        .iter()
        .filter_map(|feed| feed.id.map(|id| (feed.name_ru.clone(), id)))
        .collect();
    let mut current_prices = collect_current_prices(conn, &benchmark_inputs)?;

    let mapped = map_prices_to_feeds(fetched_prices, &feed_name_map);
    let mut updated = 0usize;

    for (feed_id, price_per_ton, source, region, source_url) in &mapped {
        let Some(feed) = feed_by_id.get(feed_id) else {
            continue;
        };
        let existing = current_prices.get(feed_id);
        if should_skip_direct_refresh(feed, existing) {
            continue;
        }

        let price = FeedPrice {
            id: None,
            feed_id: *feed_id,
            region: region.clone(),
            price_rubles_per_ton: *price_per_ton,
            price_date: Some(current_price_date()),
            source: Some(source.clone()),
            notes: source_url
                .as_ref()
                .map(|url| format!("Price source: {url}")),
            created_at: None,
        };

        if is_same_price(existing, &price) {
            continue;
        }

        match prices::upsert_price(conn, &price) {
            Ok(_) => {
                current_prices.insert(*feed_id, price);
                updated += 1;
            }
            Err(error) => {
                tracing::warn!("Failed to upsert price for feed {}: {}", feed_id, error)
            }
        }
    }

    let benchmarks = build_price_benchmarks(&benchmark_inputs, &current_prices);

    for feed in &benchmark_inputs {
        let Some(feed_id) = feed.id else {
            continue;
        };

        let existing = current_prices.get(&feed_id);
        if should_skip_inferred_refresh(feed, existing) {
            continue;
        }

        let Some(price) = infer_benchmark_price(feed, &benchmarks) else {
            continue;
        };
        if is_same_price(existing, &price) {
            continue;
        }

        match prices::upsert_price(conn, &price) {
            Ok(_) => {
                current_prices.insert(feed_id, price);
                updated += 1;
            }
            Err(error) => {
                tracing::warn!("Failed to infer price for feed {}: {}", feed_id, error)
            }
        }
    }

    Ok(updated)
}

pub fn refresh_inferred_prices(db: &Database) -> Result<usize> {
    db.with_conn(|conn| refresh_price_cache(conn, &[]))
}

/// Run full sync: ensure source-family seed rows exist and refresh cached web prices.
pub async fn sync_all(db: Arc<Database>) -> Result<SyncReport> {
    let seed_imported = {
        let db_ref = db.clone();
        seed_from_json_if_empty(&db_ref)?
    };

    let feeds_total = db.with_conn(|conn| feeds::count_feeds(conn, None))? as usize;

    let fetcher = PriceFetcher::new();
    let fetched_prices = match fetcher.fetch_all().await {
        Ok(prices) => prices,
        Err(error) => {
            tracing::warn!("Price fetching failed: {}", error);
            Vec::new()
        }
    };

    let prices_total = fetched_prices.len();

    let db_for_prices = db.clone();
    let prices_updated = db_for_prices.with_conn(|conn| refresh_price_cache(conn, &fetched_prices))?;

    Ok(SyncReport {
        feeds_imported: seed_imported,
        feeds_errors: 0,
        feeds_total,
        prices_updated,
        prices_total,
        seed_imported,
    })
}

/// Ensure normalized source-family feeds exist in the database.
/// Existing rows are merged so richer source nutrition can fill missing fields
/// without dropping previously stored values.
pub fn seed_from_json_if_empty(db: &Database) -> Result<usize> {
    let seed_feeds = match normalized_feed_db::load_workspace_seed_feeds()? {
        Some(feeds) => feeds,
        None => {
            return Err(anyhow!(
                "No generated feed authority or database/output source-family files found"
            ));
        }
    };

    let mut imported = 0usize;

    for feed in &seed_feeds {
        match db.with_conn(|conn| {
            let source_id = feed
                .source_id
                .clone()
                .unwrap_or_else(|| format!("seed:{}", feed.name_ru));
            let existing_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM feeds WHERE source_id = ?",
                    [source_id],
                    |row| row.get(0),
                )
                .ok();

            let feed_id = feeds::upsert_feed(conn, feed)?;
            let inserted = existing_id.is_none();

            if let Some(price_per_ton) = feed.price_per_ton {
                let has_price = prices::get_price(conn, feed_id, feed.region.as_deref())?.is_some();
                if !has_price {
                    let price =
                        FeedPrice {
                            id: None,
                            feed_id,
                            region: feed.region.clone(),
                            price_rubles_per_ton: price_per_ton,
                            price_date: Some(feed.price_updated_at.clone().unwrap_or_else(|| {
                                chrono::Utc::now().format("%Y-%m-%d").to_string()
                            })),
                            source: Some("seed".to_string()),
                            notes: feed.notes.clone(),
                            created_at: None,
                        };
                    prices::upsert_price(conn, &price)?;
                }
            }
            Ok(inserted)
        }) {
            Ok(true) => imported += 1,
            Ok(false) => {}
            Err(error) => tracing::warn!("Failed to seed feed '{}': {}", feed.name_ru, error),
        }
    }

    tracing::info!(
        "Inserted {} missing seed feeds from database/output source family",
        imported
    );
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::{
        benchmark_family, build_price_benchmarks, infer_benchmark_price, seed_from_json_if_empty,
    };
    use crate::db::feeds::{self, Feed};
    use crate::db::prices::FeedPrice;
    use crate::db::Database;
    use std::collections::HashMap;

    #[test]
    fn seed_source_family_populates_database() {
        let db = Database::new(":memory:").unwrap();
        db.run_migrations().unwrap();

        let imported = seed_from_json_if_empty(&db).unwrap();
        assert!(imported > 0);

        let total = db.with_conn(|conn| feeds::count_feeds(conn, None)).unwrap();
        assert!(total > 100);

        let salt = db
            .with_conn(|conn| {
                feeds::list_feeds(conn, None, Some("Поваренная соль"), Some(20), None)
            })
            .unwrap();
        assert!(!salt.is_empty());
        assert!(salt.iter().any(|feed| feed.category == "mineral"));
    }

    #[test]
    fn benchmark_inference_prefers_subcategory_then_category() {
        let feeds = vec![
            Feed {
                id: Some(1),
                name_ru: "Feed wheat A".to_string(),
                category: "grain".to_string(),
                subcategory: Some("Wheat".to_string()),
                ..Default::default()
            },
            Feed {
                id: Some(2),
                name_ru: "Feed wheat B".to_string(),
                category: "grain".to_string(),
                subcategory: Some("Wheat".to_string()),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Feed barley".to_string(),
                category: "grain".to_string(),
                subcategory: Some("Barley".to_string()),
                ..Default::default()
            },
        ];

        let current_prices = HashMap::from([(
            1_i64,
            FeedPrice {
                id: None,
                feed_id: 1,
                region: None,
                price_rubles_per_ton: 15_000.0,
                price_date: Some("2026-03-22".to_string()),
                source: Some("fallback".to_string()),
                notes: None,
                created_at: None,
            },
        )]);

        let benchmarks = build_price_benchmarks(&feeds, &current_prices);
        let same_subcategory = infer_benchmark_price(&feeds[1], &benchmarks).unwrap();
        let same_category = infer_benchmark_price(&feeds[2], &benchmarks).unwrap();

        assert_eq!(same_subcategory.price_rubles_per_ton, 15_000.0);
        assert_eq!(same_subcategory.source.as_deref(), Some("inferred:grain"));
        assert!(same_subcategory
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("subcategory median"));

        assert_eq!(same_category.price_rubles_per_ton, 15_000.0);
        assert_eq!(same_category.source.as_deref(), Some("inferred:grain"));
        assert!(same_category
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("category median"));
    }

    #[test]
    fn benchmark_inference_falls_back_to_family_median() {
        let feeds = vec![
            Feed {
                id: Some(10),
                name_ru: "Soy meal".to_string(),
                category: "protein".to_string(),
                subcategory: Some("Soy".to_string()),
                ..Default::default()
            },
            Feed {
                id: Some(11),
                name_ru: "Fish meal".to_string(),
                category: "animal_origin".to_string(),
                subcategory: Some("Fish".to_string()),
                ..Default::default()
            },
        ];

        let current_prices = HashMap::from([(
            10_i64,
            FeedPrice {
                id: None,
                feed_id: 10,
                region: None,
                price_rubles_per_ton: 42_000.0,
                price_date: Some("2026-03-22".to_string()),
                source: Some("fallback".to_string()),
                notes: None,
                created_at: None,
            },
        )]);

        let benchmarks = build_price_benchmarks(&feeds, &current_prices);
        let inferred = infer_benchmark_price(&feeds[1], &benchmarks).unwrap();

        assert_eq!(benchmark_family("animal_origin"), "protein");
        assert_eq!(inferred.price_rubles_per_ton, 42_000.0);
        assert_eq!(inferred.source.as_deref(), Some("inferred:protein"));
        assert!(inferred
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("family median"));
    }
}
