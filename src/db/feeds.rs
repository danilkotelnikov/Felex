//! Feed database operations

use anyhow::Result;
use rusqlite::{params, Connection, Row, ToSql};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;

use crate::db::feed_quality::{
    FeedCriticalCoverageAudit, FeedProfileSectionAudit, FeedProfileStatus, FeedSourceKind,
    FeedTranslationStatus,
};
use crate::db::search::trigram_similarity;
use crate::diet_engine::feed_groups::FeedSuitabilityStatus;

/// Feed model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    pub id: Option<i64>,
    pub source_id: Option<String>,
    pub source_url: Option<String>,
    pub name_ru: String,
    pub name_en: Option<String>,
    pub category: String,
    pub subcategory: Option<String>,
    #[serde(default)]
    pub source_category_id: Option<String>,
    #[serde(default)]
    pub source_subcategory_en: Option<String>,
    #[serde(default)]
    pub source_nutrition: Option<serde_json::Value>,

    // Energy
    pub dry_matter: Option<f64>,
    pub energy_oe_cattle: Option<f64>,
    pub energy_oe_pig: Option<f64>,
    pub energy_oe_poultry: Option<f64>,
    pub koe: Option<f64>,

    // Protein
    pub crude_protein: Option<f64>,
    pub dig_protein_cattle: Option<f64>,
    pub dig_protein_pig: Option<f64>,
    pub dig_protein_poultry: Option<f64>,
    pub lysine: Option<f64>,
    pub methionine_cystine: Option<f64>,

    // Fat and fiber
    pub crude_fat: Option<f64>,
    pub crude_fiber: Option<f64>,

    // Carbohydrates
    pub starch: Option<f64>,
    pub sugar: Option<f64>,

    // Minerals
    pub calcium: Option<f64>,
    pub phosphorus: Option<f64>,
    pub magnesium: Option<f64>,
    pub potassium: Option<f64>,
    pub sodium: Option<f64>,
    pub sulfur: Option<f64>,
    pub iron: Option<f64>,
    pub copper: Option<f64>,
    pub zinc: Option<f64>,
    pub manganese: Option<f64>,
    pub cobalt: Option<f64>,
    pub iodine: Option<f64>,
    pub carotene: Option<f64>,

    // Vitamins
    pub vit_d3: Option<f64>,
    pub vit_e: Option<f64>,

    // Other
    pub moisture: Option<f64>,
    pub feed_conversion: Option<f64>,
    pub palatability: Option<i64>,
    pub max_inclusion_cattle: Option<f64>,
    pub max_inclusion_pig: Option<f64>,
    pub max_inclusion_poultry: Option<f64>,

    // Economics
    pub price_per_ton: Option<f64>,
    pub price_updated_at: Option<String>,
    pub region: Option<String>,

    // Metadata
    pub is_custom: bool,
    pub verified: bool,
    pub notes: Option<String>,
    #[serde(default)]
    pub source_kind: Option<FeedSourceKind>,
    #[serde(default)]
    pub translation_status: Option<FeedTranslationStatus>,
    #[serde(default)]
    pub profile_status: Option<FeedProfileStatus>,
    #[serde(default)]
    pub profile_sections: Option<Vec<FeedProfileSectionAudit>>,
    #[serde(default)]
    pub critical_nutrient_audit: Option<FeedCriticalCoverageAudit>,
    #[serde(default)]
    pub suitability_status: Option<FeedSuitabilityStatus>,
    #[serde(default)]
    pub suitability_notes: Option<Vec<String>>,
    #[serde(default)]
    pub suitability_max_inclusion_pct: Option<f64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Default for Feed {
    fn default() -> Self {
        Self {
            id: None,
            source_id: None,
            source_url: None,
            name_ru: String::new(),
            name_en: None,
            category: "other".to_string(),
            subcategory: None,
            source_category_id: None,
            source_subcategory_en: None,
            source_nutrition: None,
            dry_matter: Some(86.0),
            energy_oe_cattle: None,
            energy_oe_pig: None,
            energy_oe_poultry: None,
            koe: None,
            crude_protein: None,
            dig_protein_cattle: None,
            dig_protein_pig: None,
            dig_protein_poultry: None,
            lysine: None,
            methionine_cystine: None,
            crude_fat: None,
            crude_fiber: None,
            starch: None,
            sugar: None,
            calcium: None,
            phosphorus: None,
            magnesium: None,
            potassium: None,
            sodium: None,
            sulfur: None,
            iron: None,
            copper: None,
            zinc: None,
            manganese: None,
            cobalt: None,
            iodine: None,
            carotene: None,
            vit_d3: None,
            vit_e: None,
            moisture: None,
            feed_conversion: None,
            palatability: None,
            max_inclusion_cattle: None,
            max_inclusion_pig: None,
            max_inclusion_poultry: None,
            price_per_ton: None,
            price_updated_at: None,
            region: None,
            is_custom: false,
            verified: false,
            notes: None,
            source_kind: None,
            translation_status: None,
            profile_status: None,
            profile_sections: None,
            critical_nutrient_audit: None,
            suitability_status: None,
            suitability_notes: None,
            suitability_max_inclusion_pct: None,
            created_at: None,
            updated_at: None,
        }
    }
}

impl Feed {
    /// Price per kg
    pub fn price_per_kg(&self) -> f64 {
        self.price_per_ton.unwrap_or(0.0) / 1000.0
    }

    /// Get nutrient value by key
    pub fn nutrient_value(&self, key: &str) -> f64 {
        match key {
            "dry_matter" => self.dry_matter.unwrap_or(0.0),
            "feed_units" | "koe" => self.koe.unwrap_or(0.0),
            "crude_protein" => self.crude_protein.unwrap_or(0.0),
            "dig_protein_cattle" | "digestible_protein_cattle" => {
                self.dig_protein_cattle.unwrap_or(0.0)
            }
            "dig_protein_pig" | "digestible_protein_pig" => self.dig_protein_pig.unwrap_or(0.0),
            "dig_protein_poultry" | "digestible_protein_poultry" => {
                self.dig_protein_poultry.unwrap_or(0.0)
            }
            "lysine" | "lysine_sid" | "lysine_tid" => self.lysine.unwrap_or(0.0),
            "methionine_cystine" | "methionine_cystine_sid" | "methionine_cystine_tid" => {
                self.methionine_cystine.unwrap_or(0.0)
            }
            "crude_fat" => self.crude_fat.unwrap_or(0.0),
            "crude_fiber" => self.crude_fiber.unwrap_or(0.0),
            "starch" => self.starch.unwrap_or(0.0),
            "sugar" | "sugars" => self.sugar.unwrap_or(0.0),
            "calcium" => self.calcium.unwrap_or(0.0),
            "phosphorus" => self.phosphorus.unwrap_or(0.0),
            "magnesium" => self.magnesium.unwrap_or(0.0),
            "potassium" => self.potassium.unwrap_or(0.0),
            "sodium" => self.sodium.unwrap_or(0.0),
            "sulfur" => self.sulfur.unwrap_or(0.0),
            "iron" => self.iron.unwrap_or(0.0),
            "copper" => self.copper.unwrap_or(0.0),
            "zinc" => self.zinc.unwrap_or(0.0),
            "manganese" => self.manganese.unwrap_or(0.0),
            "cobalt" => self.cobalt.unwrap_or(0.0),
            "iodine" => self.iodine.unwrap_or(0.0),
            "carotene" => self.carotene.unwrap_or(0.0),
            "energy_oe_cattle" => self.energy_oe_cattle.unwrap_or(0.0),
            "energy_oe_pig" => self.energy_oe_pig.unwrap_or(0.0),
            "energy_oe_poultry" => self.energy_oe_poultry.unwrap_or(0.0),
            "vit_d3" | "vitamin_d" => self.vit_d3.unwrap_or(0.0),
            "vit_e" | "vitamin_e" => self.vit_e.unwrap_or(0.0),
            _ => 0.0,
        }
    }

    /// Read from database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            source_id: row.get("source_id")?,
            source_url: row.get("source_url")?,
            name_ru: row.get("name_ru")?,
            name_en: row.get("name_en")?,
            category: row.get("category")?,
            subcategory: row.get("subcategory")?,
            source_category_id: None,
            source_subcategory_en: None,
            source_nutrition: None,
            dry_matter: row.get("dry_matter")?,
            energy_oe_cattle: row.get("energy_oe_cattle")?,
            energy_oe_pig: row.get("energy_oe_pig")?,
            energy_oe_poultry: row.get("energy_oe_poultry")?,
            koe: row.get("koe")?,
            crude_protein: row.get("crude_protein")?,
            dig_protein_cattle: row.get("dig_protein_cattle")?,
            dig_protein_pig: row.get("dig_protein_pig")?,
            dig_protein_poultry: row.get("dig_protein_poultry")?,
            lysine: row.get("lysine")?,
            methionine_cystine: row.get("methionine_cystine")?,
            crude_fat: row.get("crude_fat")?,
            crude_fiber: row.get("crude_fiber")?,
            starch: row.get("starch")?,
            sugar: row.get("sugar")?,
            calcium: row.get("calcium")?,
            phosphorus: row.get("phosphorus")?,
            magnesium: row.get("magnesium")?,
            potassium: row.get("potassium")?,
            sodium: row.get("sodium")?,
            sulfur: row.get("sulfur")?,
            iron: row.get("iron")?,
            copper: row.get("copper")?,
            zinc: row.get("zinc")?,
            manganese: row.get("manganese")?,
            cobalt: row.get("cobalt")?,
            iodine: row.get("iodine")?,
            carotene: row.get("carotene")?,
            vit_d3: row.get("vit_d3")?,
            vit_e: row.get("vit_e")?,
            moisture: row.get("moisture")?,
            feed_conversion: row.get("feed_conversion")?,
            palatability: row.get("palatability")?,
            max_inclusion_cattle: row.get("max_inclusion_cattle")?,
            max_inclusion_pig: row.get("max_inclusion_pig")?,
            max_inclusion_poultry: row.get("max_inclusion_poultry")?,
            price_per_ton: row.get("price_per_ton")?,
            price_updated_at: row.get("price_updated_at")?,
            region: row.get("region")?,
            is_custom: row.get::<_, i32>("is_custom")? != 0,
            verified: row.get::<_, i32>("verified")? != 0,
            notes: row.get("notes")?,
            source_kind: None,
            translation_status: None,
            profile_status: None,
            profile_sections: None,
            critical_nutrient_audit: None,
            suitability_status: None,
            suitability_notes: None,
            suitability_max_inclusion_pct: None,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

fn merge_optional<T>(incoming: Option<T>, existing: Option<T>) -> Option<T> {
    incoming.or(existing)
}

fn merge_text(incoming: &str, existing: &str) -> String {
    if incoming.trim().is_empty() {
        existing.to_string()
    } else {
        incoming.to_string()
    }
}

fn merge_category(incoming: &str, existing: &str) -> String {
    if incoming.trim().is_empty() || (incoming == "other" && existing != "other") {
        existing.to_string()
    } else {
        incoming.to_string()
    }
}

fn merge_feed(existing: &Feed, incoming: &Feed) -> Feed {
    Feed {
        id: existing.id,
        source_id: incoming
            .source_id
            .clone()
            .or_else(|| existing.source_id.clone()),
        source_url: incoming
            .source_url
            .clone()
            .or_else(|| existing.source_url.clone()),
        name_ru: merge_text(&incoming.name_ru, &existing.name_ru),
        name_en: incoming
            .name_en
            .clone()
            .or_else(|| existing.name_en.clone()),
        category: merge_category(&incoming.category, &existing.category),
        subcategory: incoming
            .subcategory
            .clone()
            .or_else(|| existing.subcategory.clone()),
        source_category_id: incoming
            .source_category_id
            .clone()
            .or_else(|| existing.source_category_id.clone()),
        source_subcategory_en: incoming
            .source_subcategory_en
            .clone()
            .or_else(|| existing.source_subcategory_en.clone()),
        source_nutrition: incoming
            .source_nutrition
            .clone()
            .or_else(|| existing.source_nutrition.clone()),
        dry_matter: merge_optional(incoming.dry_matter, existing.dry_matter),
        energy_oe_cattle: merge_optional(incoming.energy_oe_cattle, existing.energy_oe_cattle),
        energy_oe_pig: merge_optional(incoming.energy_oe_pig, existing.energy_oe_pig),
        energy_oe_poultry: merge_optional(incoming.energy_oe_poultry, existing.energy_oe_poultry),
        koe: merge_optional(incoming.koe, existing.koe),
        crude_protein: merge_optional(incoming.crude_protein, existing.crude_protein),
        dig_protein_cattle: merge_optional(
            incoming.dig_protein_cattle,
            existing.dig_protein_cattle,
        ),
        dig_protein_pig: merge_optional(incoming.dig_protein_pig, existing.dig_protein_pig),
        dig_protein_poultry: merge_optional(
            incoming.dig_protein_poultry,
            existing.dig_protein_poultry,
        ),
        lysine: merge_optional(incoming.lysine, existing.lysine),
        methionine_cystine: merge_optional(
            incoming.methionine_cystine,
            existing.methionine_cystine,
        ),
        crude_fat: merge_optional(incoming.crude_fat, existing.crude_fat),
        crude_fiber: merge_optional(incoming.crude_fiber, existing.crude_fiber),
        starch: merge_optional(incoming.starch, existing.starch),
        sugar: merge_optional(incoming.sugar, existing.sugar),
        calcium: merge_optional(incoming.calcium, existing.calcium),
        phosphorus: merge_optional(incoming.phosphorus, existing.phosphorus),
        magnesium: merge_optional(incoming.magnesium, existing.magnesium),
        potassium: merge_optional(incoming.potassium, existing.potassium),
        sodium: merge_optional(incoming.sodium, existing.sodium),
        sulfur: merge_optional(incoming.sulfur, existing.sulfur),
        iron: merge_optional(incoming.iron, existing.iron),
        copper: merge_optional(incoming.copper, existing.copper),
        zinc: merge_optional(incoming.zinc, existing.zinc),
        manganese: merge_optional(incoming.manganese, existing.manganese),
        cobalt: merge_optional(incoming.cobalt, existing.cobalt),
        iodine: merge_optional(incoming.iodine, existing.iodine),
        carotene: merge_optional(incoming.carotene, existing.carotene),
        vit_d3: merge_optional(incoming.vit_d3, existing.vit_d3),
        vit_e: merge_optional(incoming.vit_e, existing.vit_e),
        moisture: merge_optional(incoming.moisture, existing.moisture),
        feed_conversion: merge_optional(incoming.feed_conversion, existing.feed_conversion),
        palatability: merge_optional(incoming.palatability, existing.palatability),
        max_inclusion_cattle: merge_optional(
            incoming.max_inclusion_cattle,
            existing.max_inclusion_cattle,
        ),
        max_inclusion_pig: merge_optional(incoming.max_inclusion_pig, existing.max_inclusion_pig),
        max_inclusion_poultry: merge_optional(
            incoming.max_inclusion_poultry,
            existing.max_inclusion_poultry,
        ),
        price_per_ton: merge_optional(incoming.price_per_ton, existing.price_per_ton),
        price_updated_at: incoming
            .price_updated_at
            .clone()
            .or_else(|| existing.price_updated_at.clone()),
        region: incoming.region.clone().or_else(|| existing.region.clone()),
        is_custom: existing.is_custom || incoming.is_custom,
        verified: existing.verified || incoming.verified,
        notes: incoming.notes.clone().or_else(|| existing.notes.clone()),
        source_kind: None,
        translation_status: None,
        profile_status: None,
        profile_sections: None,
        critical_nutrient_audit: None,
        suitability_status: None,
        suitability_notes: None,
        suitability_max_inclusion_pct: None,
        created_at: existing.created_at.clone(),
        updated_at: existing.updated_at.clone(),
    }
}

fn load_feeds_with_query(conn: &Connection, sql: &str, params: &[&dyn ToSql]) -> Result<Vec<Feed>> {
    let mut stmt = conn.prepare(sql)?;
    let feeds = stmt
        .query_map(params, Feed::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(feeds)
}

fn list_feeds_without_search(
    conn: &Connection,
    category: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Feed>> {
    let mut sql = String::from("SELECT * FROM feeds");
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(cat) = category {
        sql.push_str(" WHERE category = ?");
        params_vec.push(Box::new(cat.to_string()));
    }

    sql.push_str(" ORDER BY verified DESC, name_ru ASC");

    if let Some(l) = limit {
        sql.push_str(&format!(" LIMIT {}", l));
    }
    if let Some(o) = offset {
        sql.push_str(&format!(" OFFSET {}", o));
    }

    let params_refs: Vec<&dyn ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    load_feeds_with_query(conn, &sql, params_refs.as_slice())
}

fn tokenize_search_query(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in query.to_lowercase().chars() {
        if ch.is_alphanumeric() {
            current.push(ch);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn field_search_score(field: &str, query: &str) -> f64 {
    let mut best = trigram_similarity(query, field);
    for token in tokenize_search_query(field) {
        best = best.max(trigram_similarity(query, &token));
    }
    best
}

fn search_score(feed: &Feed, query: &str) -> f64 {
    let ru = field_search_score(&feed.name_ru, query);
    let en = feed
        .name_en
        .as_deref()
        .map(|value| field_search_score(value, query))
        .unwrap_or(0.0);
    let subcategory = feed
        .subcategory
        .as_deref()
        .map(|value| field_search_score(value, query))
        .unwrap_or(0.0);

    ru.max(en).max(subcategory)
}

fn all_search_results(
    conn: &Connection,
    category: Option<&str>,
    query: &str,
) -> Result<Vec<Feed>> {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return list_feeds_without_search(conn, category, None, None);
    }

    let mut results = Vec::new();
    let mut seen_ids = HashSet::new();
    let terms = tokenize_search_query(&normalized);

    if !terms.is_empty() {
        let fts_query = terms
            .iter()
            .map(|term| format!("{term}*"))
            .collect::<Vec<_>>()
            .join(" OR ");
        let mut sql = String::from(
            "SELECT f.* FROM feeds f \
             JOIN feeds_fts fts ON f.id = fts.rowid \
             WHERE feeds_fts MATCH ?",
        );
        let mut params_vec: Vec<Box<dyn ToSql>> = vec![Box::new(fts_query)];

        if let Some(cat) = category {
            sql.push_str(" AND f.category = ?");
            params_vec.push(Box::new(cat.to_string()));
        }

        sql.push_str(" ORDER BY bm25(feeds_fts), f.verified DESC, f.name_ru ASC");

        let params_refs: Vec<&dyn ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        for feed in load_feeds_with_query(conn, &sql, params_refs.as_slice())? {
            if let Some(id) = feed.id {
                seen_ids.insert(id);
            }
            results.push(feed);
        }
    }

    let mut fuzzy_matches: Vec<(Feed, f64)> = list_feeds_without_search(conn, category, None, None)?
        .into_iter()
        .filter(|feed| feed.id.map(|id| !seen_ids.contains(&id)).unwrap_or(true))
        .map(|feed| {
            let score = search_score(&feed, &normalized);
            (feed, score)
        })
        .filter(|(_, score)| *score >= 0.3)
        .collect();

    fuzzy_matches.sort_by(|(left_feed, left_score), (right_feed, right_score)| {
        right_score
            .partial_cmp(left_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right_feed.verified.cmp(&left_feed.verified))
            .then_with(|| left_feed.name_ru.cmp(&right_feed.name_ru))
    });

    results.extend(fuzzy_matches.into_iter().map(|(feed, _)| feed));
    Ok(results)
}

pub fn search_feeds(
    conn: &Connection,
    category: Option<&str>,
    query: &str,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Feed>> {
    let start = offset.unwrap_or(0).max(0) as usize;
    let results = all_search_results(conn, category, query)?;
    let iter = results.into_iter().skip(start);

    match limit {
        Some(limit) if limit >= 0 => Ok(iter.take(limit as usize).collect()),
        _ => Ok(iter.collect()),
    }
}

/// List all feeds with optional filters
pub fn list_feeds(
    conn: &Connection,
    category: Option<&str>,
    search: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Feed>> {
    if let Some(query) = search.map(str::trim).filter(|query| !query.is_empty()) {
        return search_feeds(conn, category, query, limit, offset);
    }

    list_feeds_without_search(conn, category, limit, offset)
}

/// Get a single feed by ID
pub fn get_feed(conn: &Connection, id: i64) -> Result<Option<Feed>> {
    let mut stmt = conn.prepare("SELECT * FROM feeds WHERE id = ?")?;
    let feed = stmt.query_row([id], Feed::from_row).ok();
    Ok(feed)
}

/// Insert a new feed
pub fn insert_feed(conn: &Connection, feed: &Feed) -> Result<i64> {
    conn.execute(
        "INSERT INTO feeds (
            source_id, source_url, name_ru, name_en, category, subcategory,
            dry_matter, energy_oe_cattle, energy_oe_pig, energy_oe_poultry, koe,
            crude_protein, dig_protein_cattle, dig_protein_pig, dig_protein_poultry,
            lysine, methionine_cystine,
            crude_fat, crude_fiber, starch, sugar,
            calcium, phosphorus, magnesium, potassium, sodium, sulfur, iron, copper, zinc, manganese, cobalt, iodine, carotene,
            vit_d3, vit_e,
            moisture, feed_conversion, palatability, max_inclusion_cattle, max_inclusion_pig, max_inclusion_poultry,
            price_per_ton, price_updated_at, region, is_custom, verified, notes
        ) VALUES (
            ?, ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
            ?, ?,
            ?, ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?, ?
        )",
        params![
            feed.source_id, feed.source_url, feed.name_ru, feed.name_en,
            feed.category, feed.subcategory,
            feed.dry_matter, feed.energy_oe_cattle, feed.energy_oe_pig, feed.energy_oe_poultry, feed.koe,
            feed.crude_protein, feed.dig_protein_cattle, feed.dig_protein_pig, feed.dig_protein_poultry,
            feed.lysine, feed.methionine_cystine,
            feed.crude_fat, feed.crude_fiber, feed.starch, feed.sugar,
            feed.calcium, feed.phosphorus, feed.magnesium, feed.potassium, feed.sodium, feed.sulfur, feed.iron, feed.copper, feed.zinc, feed.manganese, feed.cobalt, feed.iodine, feed.carotene,
            feed.vit_d3, feed.vit_e,
            feed.moisture, feed.feed_conversion, feed.palatability, feed.max_inclusion_cattle, feed.max_inclusion_pig, feed.max_inclusion_poultry,
            feed.price_per_ton, feed.price_updated_at, feed.region, feed.is_custom as i32, feed.verified as i32, feed.notes
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Update an existing feed
pub fn update_feed(conn: &Connection, id: i64, feed: &Feed) -> Result<()> {
    conn.execute(
        "UPDATE feeds SET
            name_ru = ?, name_en = ?, category = ?, subcategory = ?,
            source_id = ?, source_url = ?,
            dry_matter = ?, energy_oe_cattle = ?, energy_oe_pig = ?, energy_oe_poultry = ?, koe = ?,
            crude_protein = ?, dig_protein_cattle = ?, dig_protein_pig = ?, dig_protein_poultry = ?,
            lysine = ?, methionine_cystine = ?,
            crude_fat = ?, crude_fiber = ?, starch = ?, sugar = ?,
            calcium = ?, phosphorus = ?, magnesium = ?, potassium = ?, sodium = ?, sulfur = ?, iron = ?, copper = ?, zinc = ?, manganese = ?, cobalt = ?, iodine = ?, carotene = ?,
            vit_d3 = ?, vit_e = ?,
            moisture = ?, feed_conversion = ?, palatability = ?, max_inclusion_cattle = ?, max_inclusion_pig = ?, max_inclusion_poultry = ?,
            price_per_ton = ?, price_updated_at = ?, region = ?, verified = ?, notes = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?",
        params![
            feed.name_ru, feed.name_en, feed.category, feed.subcategory,
            feed.source_id, feed.source_url,
            feed.dry_matter, feed.energy_oe_cattle, feed.energy_oe_pig, feed.energy_oe_poultry, feed.koe,
            feed.crude_protein, feed.dig_protein_cattle, feed.dig_protein_pig, feed.dig_protein_poultry,
            feed.lysine, feed.methionine_cystine,
            feed.crude_fat, feed.crude_fiber, feed.starch, feed.sugar,
            feed.calcium, feed.phosphorus, feed.magnesium, feed.potassium, feed.sodium, feed.sulfur, feed.iron, feed.copper, feed.zinc, feed.manganese, feed.cobalt, feed.iodine, feed.carotene,
            feed.vit_d3, feed.vit_e,
            feed.moisture, feed.feed_conversion, feed.palatability, feed.max_inclusion_cattle, feed.max_inclusion_pig, feed.max_inclusion_poultry,
            feed.price_per_ton, feed.price_updated_at, feed.region, feed.verified as i32, feed.notes,
            id
        ],
    )?;

    Ok(())
}

/// Delete a feed by ID
pub fn delete_feed(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM feeds WHERE id = ?", [id])?;
    Ok(())
}

/// Upsert a feed (insert or update based on source_id)
pub fn upsert_feed(conn: &Connection, feed: &Feed) -> Result<i64> {
    if let Some(ref source_id) = feed.source_id {
        let existing: Option<Feed> = conn
            .query_row(
                "SELECT * FROM feeds WHERE source_id = ?",
                [source_id],
                Feed::from_row,
            )
            .ok();

        if let Some(existing_feed) = existing {
            let merged = merge_feed(&existing_feed, feed);
            let id = existing_feed.id.unwrap_or_default();
            update_feed(conn, id, &merged)?;
            return Ok(id);
        }
    }

    if !feed.is_custom && !feed.name_ru.trim().is_empty() {
        let existing: Option<Feed> = conn
            .query_row(
                "SELECT * FROM feeds
                 WHERE is_custom = 0 AND lower(trim(name_ru)) = lower(trim(?))
                 ORDER BY CASE WHEN category = ? THEN 0 ELSE 1 END, id
                 LIMIT 1",
                params![feed.name_ru, feed.category],
                Feed::from_row,
            )
            .ok();

        if let Some(existing_feed) = existing {
            let merged = merge_feed(&existing_feed, feed);
            let id = existing_feed.id.unwrap_or_default();
            update_feed(conn, id, &merged)?;
            return Ok(id);
        }
    }

    insert_feed(conn, feed)
}

/// Count total feeds
pub fn count_feeds(conn: &Connection, category: Option<&str>) -> Result<i64> {
    let sql = match category {
        Some(_) => "SELECT COUNT(*) FROM feeds WHERE category = ?",
        None => "SELECT COUNT(*) FROM feeds",
    };

    let count: i64 = match category {
        Some(cat) => conn.query_row(sql, [cat], |row| row.get(0))?,
        None => conn.query_row(sql, [], |row| row.get(0))?,
    };

    Ok(count)
}

pub fn count_feeds_filtered(
    conn: &Connection,
    category: Option<&str>,
    search: Option<&str>,
) -> Result<i64> {
    if let Some(query) = search.map(str::trim).filter(|query| !query.is_empty()) {
        return Ok(all_search_results(conn, category, query)?.len() as i64);
    }

    count_feeds(conn, category)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;
    use rusqlite::Connection;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn upsert_feed_keeps_existing_values_when_incoming_is_sparse() {
        let conn = setup_conn();

        let base = Feed {
            source_id: Some("seed:test_feed".to_string()),
            name_ru: "Test feed".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(87.0),
            crude_protein: Some(120.0),
            lysine: Some(3.6),
            potassium: Some(4.5),
            notes: Some("base".to_string()),
            verified: true,
            ..Default::default()
        };

        let sparse = Feed {
            source_id: Some("seed:test_feed".to_string()),
            name_ru: "Test feed".to_string(),
            category: "other".to_string(),
            dry_matter: None,
            crude_protein: Some(125.0),
            notes: Some("incoming".to_string()),
            ..Default::default()
        };

        let id = upsert_feed(&conn, &base).unwrap();
        upsert_feed(&conn, &sparse).unwrap();

        let merged = get_feed(&conn, id).unwrap().unwrap();
        assert_eq!(merged.category, "grain");
        assert_eq!(merged.dry_matter, Some(87.0));
        assert_eq!(merged.crude_protein, Some(125.0));
        assert_eq!(merged.lysine, Some(3.6));
        assert_eq!(merged.potassium, Some(4.5));
        assert_eq!(merged.notes.as_deref(), Some("incoming"));
        assert!(merged.verified);
    }

    #[test]
    fn upsert_feed_merges_seed_variants_by_name_when_source_changes() {
        let conn = setup_conn();

        let legacy = Feed {
            source_id: Some("seed:legacy:sunflower_meal".to_string()),
            name_ru: "Подсолнечный шрот".to_string(),
            category: "protein".to_string(),
            crude_protein: Some(320.0),
            lysine: Some(8.2),
            verified: true,
            ..Default::default()
        };

        let normalized = Feed {
            source_id: Some("seed:normalized-db:n42".to_string()),
            source_url: Some("https://example.test/n42".to_string()),
            name_ru: "Подсолнечный шрот".to_string(),
            name_en: Some("Sunflower meal".to_string()),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(89.0),
            crude_protein: Some(345.0),
            methionine_cystine: Some(11.5),
            notes: Some("normalized".to_string()),
            ..Default::default()
        };

        let id = upsert_feed(&conn, &legacy).unwrap();
        let updated_id = upsert_feed(&conn, &normalized).unwrap();

        assert_eq!(id, updated_id);

        let merged = get_feed(&conn, id).unwrap().unwrap();
        assert_eq!(
            conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM feeds", [], |row| row.get(0))
                .unwrap(),
            1
        );
        assert_eq!(merged.source_id.as_deref(), Some("seed:normalized-db:n42"));
        assert_eq!(merged.category, "oilseed_meal");
        assert_eq!(merged.dry_matter, Some(89.0));
        assert_eq!(merged.crude_protein, Some(345.0));
        assert_eq!(merged.lysine, Some(8.2));
        assert_eq!(merged.methionine_cystine, Some(11.5));
        assert_eq!(merged.name_en.as_deref(), Some("Sunflower meal"));
        assert_eq!(merged.notes.as_deref(), Some("normalized"));
    }

    #[test]
    fn search_feeds_matches_prefix_with_fts() {
        let conn = setup_conn();
        insert_feed(
            &conn,
            &Feed {
                name_ru: "Пшеница фуражная".to_string(),
                category: "grain".to_string(),
                verified: true,
                ..Default::default()
            },
        )
        .unwrap();
        insert_feed(
            &conn,
            &Feed {
                name_ru: "Ячмень дробленый".to_string(),
                category: "grain".to_string(),
                verified: true,
                ..Default::default()
            },
        )
        .unwrap();

        let results = search_feeds(&conn, None, "пшен", Some(10), None).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_ru, "Пшеница фуражная");
    }

    #[test]
    fn search_feeds_uses_trigram_fallback_for_typos() {
        let conn = setup_conn();
        insert_feed(
            &conn,
            &Feed {
                name_ru: "Пшеница фуражная".to_string(),
                category: "grain".to_string(),
                verified: true,
                ..Default::default()
            },
        )
        .unwrap();

        let results = search_feeds(&conn, None, "пшенца", Some(10), None).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_ru, "Пшеница фуражная");
    }

    #[test]
    fn count_feeds_filtered_respects_search_and_category() {
        let conn = setup_conn();
        insert_feed(
            &conn,
            &Feed {
                name_ru: "Пшеница фуражная".to_string(),
                category: "grain".to_string(),
                verified: true,
                ..Default::default()
            },
        )
        .unwrap();
        insert_feed(
            &conn,
            &Feed {
                name_ru: "Премикс для свиней".to_string(),
                category: "premix".to_string(),
                verified: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            count_feeds_filtered(&conn, Some("grain"), Some("пшен")).unwrap(),
            1
        );
        assert_eq!(
            count_feeds_filtered(&conn, Some("premix"), Some("пшен")).unwrap(),
            0
        );
    }
}
