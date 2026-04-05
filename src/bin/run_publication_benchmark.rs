use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use chrono::Utc;
use felex::db::{self, Database};
use felex::diet_engine::benchmarking::{
    default_benchmark_cases, run_publication_benchmark, BenchmarkPriceInfo, BenchmarkRun,
};
use felex::scraper::multi_source;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct BenchmarkArtifact {
    generated_at_utc: String,
    source_feed_database: String,
    benchmark_catalog_database: String,
    source_price_database: Option<String>,
    matched_price_anchors: usize,
    observed_source_nutrients: Vec<String>,
    benchmark: BenchmarkRun,
}

#[derive(Debug)]
struct PreparedCatalog {
    feeds: Vec<db::feeds::Feed>,
    price_info: HashMap<i64, BenchmarkPriceInfo>,
    matched_price_anchors: usize,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let output_dir = args
        .iter()
        .position(|arg| arg == "--output-dir")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .unwrap_or_else(default_output_dir);
    let case_filter = args
        .iter()
        .position(|arg| arg == "--case")
        .and_then(|index| args.get(index + 1))
        .cloned();
    let price_db_path = args
        .iter()
        .position(|arg| arg == "--price-db")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("felex.db"));

    fs::create_dir_all(&output_dir)?;
    let benchmark_catalog_db = output_dir.join("benchmark_catalog.db");
    let prepared = prepare_benchmark_catalog(&benchmark_catalog_db, Some(&price_db_path))?;

    let mut cases = default_benchmark_cases();
    if let Some(filter) = case_filter {
        cases.retain(|case| case.id == filter);
    }

    let benchmark = run_publication_benchmark(&prepared.feeds, Some(&prepared.price_info), &cases)?;

    let output_path = output_dir.join("benchmark_results.json");
    let writer = BufWriter::new(File::create(&output_path)?);
    let artifact = BenchmarkArtifact {
        generated_at_utc: Utc::now().to_rfc3339(),
        source_feed_database: "database/output/feeds_database.json".to_string(),
        benchmark_catalog_database: benchmark_catalog_db.display().to_string(),
        source_price_database: price_db_path.exists().then(|| price_db_path.display().to_string()),
        matched_price_anchors: prepared.matched_price_anchors,
        observed_source_nutrients: observed_source_nutrients()?,
        benchmark,
    };
    serde_json::to_writer_pretty(writer, &artifact)?;

    println!("{}", output_path.display());
    Ok(())
}

fn prepare_benchmark_catalog(
    benchmark_db_path: &Path,
    source_price_db_path: Option<&Path>,
) -> anyhow::Result<PreparedCatalog> {
    if benchmark_db_path.exists() {
        fs::remove_file(benchmark_db_path)?;
    }

    let benchmark_db = Database::new(&benchmark_db_path.display().to_string())?;
    benchmark_db.run_migrations()?;
    multi_source::seed_from_json_if_empty(&benchmark_db)?;

    let matched_price_anchors = match source_price_db_path {
        Some(path) if path.exists() => import_price_anchors(path, &benchmark_db)?,
        _ => 0,
    };

    multi_source::refresh_inferred_prices(&benchmark_db)?;

    let (feeds, price_info) = benchmark_db.with_conn(|conn| {
        let feeds = db::feeds::list_feeds(conn, None, None, Some(10_000), None)?;
        let current_prices = db::prices::list_prices(conn, None)?
            .into_iter()
            .map(|price| (price.feed_id, price))
            .collect::<HashMap<_, _>>();

        let mut price_info = HashMap::new();
        for feed in &feeds {
            let Some(feed_id) = feed.id else {
                continue;
            };
            let Some(price) = current_prices.get(&feed_id) else {
                continue;
            };
            let provenance =
                multi_source::resolve_price_provenance(feed, price, &feeds, &current_prices);
            price_info.insert(
                feed_id,
                BenchmarkPriceInfo {
                    kind: provenance.kind,
                    is_precise_source: provenance.is_precise_source,
                    benchmark_level: provenance.benchmark_level,
                },
            );
        }

        Ok((feeds, price_info))
    })?;

    Ok(PreparedCatalog {
        feeds,
        price_info,
        matched_price_anchors,
    })
}

fn import_price_anchors(source_price_db_path: &Path, benchmark_db: &Database) -> anyhow::Result<usize> {
    let source_db = Database::new(&source_price_db_path.display().to_string())?;

    let source_feeds = source_db.with_conn(|conn| db::feeds::list_feeds(conn, None, None, Some(10_000), None))?;
    let source_prices = source_db.with_conn(|conn| db::prices::list_prices(conn, None))?;
    let latest_non_inferred = source_prices
        .into_iter()
        .filter(|price| {
            !price
                .source
                .as_deref()
                .is_some_and(|source| source.starts_with("inferred:"))
        })
        .map(|price| (price.feed_id, price))
        .collect::<HashMap<_, _>>();

    let target_feeds = benchmark_db.with_conn(|conn| db::feeds::list_feeds(conn, None, None, Some(10_000), None))?;
    let target_by_source_id = target_feeds
        .iter()
        .filter_map(|feed| Some((feed.source_id.as_ref()?.clone(), feed.id?)))
        .collect::<HashMap<_, _>>();
    let target_by_name = target_feeds
        .iter()
        .filter_map(|feed| Some((normalize_name(&feed.name_ru), feed.id?)))
        .collect::<HashMap<_, _>>();

    benchmark_db.with_conn_mut(|conn| {
        let mut matched = 0usize;
        for source_feed in &source_feeds {
            let Some(source_feed_id) = source_feed.id else {
                continue;
            };
            let Some(price) = latest_non_inferred.get(&source_feed_id) else {
                continue;
            };

            let target_feed_id = source_feed
                .source_id
                .as_deref()
                .and_then(|source_id| target_by_source_id.get(source_id).copied())
                .or_else(|| target_by_name.get(&normalize_name(&source_feed.name_ru)).copied());
            let Some(target_feed_id) = target_feed_id else {
                continue;
            };

            let mapped_price = db::prices::FeedPrice {
                id: None,
                feed_id: target_feed_id,
                region: price.region.clone(),
                price_rubles_per_ton: price.price_rubles_per_ton,
                price_date: price.price_date.clone(),
                source: price.source.clone(),
                notes: price.notes.clone(),
                created_at: None,
            };
            db::prices::upsert_price(conn, &mapped_price)?;
            matched += 1;
        }
        Ok(matched)
    })
}

fn normalize_name(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('ё', "е")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("deliverables").join(format!(
        "publication_benchmark_{}",
        Utc::now().format("%Y-%m-%d")
    ))
}

fn observed_source_nutrients() -> anyhow::Result<Vec<String>> {
    let raw = fs::read_to_string("database/output/feeds_database.json")?;
    let json: serde_json::Value = serde_json::from_str(&raw)?;
    let feeds = json
        .get("feeds")
        .and_then(|value| value.as_array())
        .ok_or_else(|| anyhow::anyhow!("feeds_database.json does not contain a feeds array"))?;

    let mut nutrients = BTreeSet::new();
    for feed in feeds {
        if let Some(nutrition) = feed.get("nutrition").and_then(|value| value.as_object()) {
            nutrients.extend(nutrition.keys().cloned());
        }
    }

    Ok(nutrients.into_iter().collect())
}
