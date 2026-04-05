use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf, sync::OnceLock};

use serde::Deserialize;
use serde_json::Value;

use crate::db::feeds::Feed;

const DETAIL_PROFILE_PATH: &str = "database/output/app/feed_detail_profiles.json";

#[derive(Debug, Clone, Deserialize)]
struct FeedSourceProfileEntry {
    source_id: Option<String>,
    name_en: Option<String>,
    source_category_id: Option<String>,
    source_subcategory_en: Option<String>,
    source_nutrition: Option<Value>,
}

#[derive(Debug, Clone)]
struct FeedSourceProfile {
    name_en: Option<String>,
    source_category_id: Option<String>,
    source_subcategory_en: Option<String>,
    source_nutrition: Option<Value>,
}

static PROFILE_CACHE: OnceLock<HashMap<String, FeedSourceProfile>> = OnceLock::new();

fn normalized_text(value: &str) -> String {
    value.trim().to_lowercase().replace('ё', "е")
}

fn profile_path() -> PathBuf {
    PathBuf::from(DETAIL_PROFILE_PATH)
}

fn load_profiles() -> HashMap<String, FeedSourceProfile> {
    let path = profile_path();
    let Ok(file) = File::open(path) else {
        return HashMap::new();
    };
    let reader = BufReader::new(file);
    let Ok(entries) = serde_json::from_reader::<_, Vec<FeedSourceProfileEntry>>(reader) else {
        return HashMap::new();
    };

    entries
        .into_iter()
        .filter_map(|entry| {
            let source_id = entry.source_id?;
            Some((
                source_id,
                FeedSourceProfile {
                    name_en: entry.name_en,
                    source_category_id: entry.source_category_id,
                    source_subcategory_en: entry.source_subcategory_en,
                    source_nutrition: entry.source_nutrition,
                },
            ))
        })
        .collect()
}

fn source_profiles() -> &'static HashMap<String, FeedSourceProfile> {
    PROFILE_CACHE.get_or_init(load_profiles)
}

fn should_apply_name_en(feed: &Feed, candidate: &str) -> bool {
    if candidate.trim().is_empty() {
        return false;
    }
    match feed.name_en.as_deref() {
        None => true,
        Some(current) => {
            let current_normalized = normalized_text(current);
            current_normalized.is_empty()
                || current_normalized == normalized_text(&feed.name_ru)
                || current_normalized == normalized_text(candidate)
        }
    }
}

pub fn enrich_feed(feed: Feed, include_source_nutrition: bool) -> Feed {
    let Some(source_id) = feed.source_id.clone() else {
        return feed;
    };
    let Some(profile) = source_profiles().get(&source_id) else {
        return feed;
    };

    let mut enriched = feed;
    if let Some(candidate) = profile.name_en.as_deref() {
        if should_apply_name_en(&enriched, candidate) {
            enriched.name_en = Some(candidate.to_string());
        }
    }
    if enriched.source_category_id.is_none() {
        enriched.source_category_id = profile.source_category_id.clone();
    }
    if enriched.source_subcategory_en.is_none() {
        enriched.source_subcategory_en = profile.source_subcategory_en.clone();
    }
    if include_source_nutrition && enriched.source_nutrition.is_none() {
        enriched.source_nutrition = profile.source_nutrition.clone();
    }
    enriched
}
