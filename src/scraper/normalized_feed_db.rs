use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::db::feeds::Feed;

const DEFAULT_RELATIVE_PATH: &str = "database/output/feeds_database.json";
const DEFAULT_AUTHORITY_PATH: &str = "database/output/app/feed_authority.jsonl";
const NORMALIZED_SOURCE_NOTE: &str = "Seeded from normalized feed database source family";
const CATEGORY_FILES: &[&str] = &[
    "animal_feeds.json",
    "green_feeds.json",
    "rough_feeds.json",
    "succulent_feeds.json",
    "concentrated_feeds.json",
    "industrial_byproducts.json",
    "mineral_supplements.json",
    "mixed_feeds.json",
    "nitrogen_compounds.json",
];
const MIN_PLAUSIBLE_ENERGY_MJ_PER_KG_DM: f64 = 1.0;
const MAX_PLAUSIBLE_MONOGASTRIC_ENERGY_MJ_PER_KG_DM: f64 = 25.0;
const ENERGY_DECIMAL_SHIFT_DIVISORS: &[f64] = &[1.0, 10.0, 100.0, 1000.0];

#[derive(Debug, Deserialize)]
struct NormalizedFeedDatabase {
    feeds: Vec<RawFeed>,
}

#[derive(Debug, Deserialize)]
struct CategoryFeedPayload {
    feeds: Vec<RawFeed>,
}

#[derive(Debug, Deserialize)]
struct LocalizedText {
    ru: String,
    #[serde(default)]
    en: String,
}

#[derive(Debug, Deserialize)]
struct RawFeed {
    id: String,
    name: LocalizedText,
    category_id: String,
    subcategory: LocalizedText,
    #[serde(default)]
    region_id: Option<String>,
    #[serde(default)]
    nutrition: HashMap<String, RawNutrient>,
    source_url: String,
    #[serde(default)]
    parse_errors: Vec<ParseError>,
}

#[derive(Debug, Deserialize)]
struct ParseError {
    field: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawNutrient {
    Single(UnitValue),
    AnimalSpecific(HashMap<String, UnitValue>),
    Unsupported(serde_json::Value),
}

#[derive(Debug, Clone, Deserialize)]
struct UnitValue {
    value: f64,
    unit: String,
}

impl RawFeed {
    fn uses_monogastric_proxy(category: &str) -> bool {
        matches!(
            category,
            "grain" | "concentrate" | "oilseed_meal" | "protein" | "animal_origin"
        )
    }

    fn single_value(&self, key: &str) -> Option<&UnitValue> {
        match self.nutrition.get(key)? {
            RawNutrient::Single(value) => Some(value),
            RawNutrient::Unsupported(raw) => {
                let _ = raw;
                None
            }
            RawNutrient::AnimalSpecific(_) => None,
        }
    }

    fn animal_value(&self, key: &str, aliases: &[&str]) -> Option<&UnitValue> {
        match self.nutrition.get(key)? {
            RawNutrient::Single(value) => Some(value),
            RawNutrient::AnimalSpecific(values) => aliases
                .iter()
                .find_map(|alias| values.get(*alias))
                .or_else(|| values.get("universal")),
            RawNutrient::Unsupported(raw) => {
                let _ = raw;
                None
            }
        }
    }

    fn normalized_name_en(&self) -> Option<String> {
        let ru = self.name.ru.trim();
        let en = self.name.en.trim();
        if en.is_empty() || en.eq_ignore_ascii_case(ru) || en == ru {
            None
        } else {
            Some(en.to_string())
        }
    }

    fn search_text(&self) -> String {
        format!(
            "{} {} {} {}",
            self.category_id, self.subcategory.ru, self.name.ru, self.name.en
        )
        .to_lowercase()
    }

    fn mapped_category(&self) -> String {
        let search = self.search_text();
        let looks_like_premix = has_any(
            &search,
            &[
                "premix",
                "премикс",
                "бвмд",
                "бмвд",
                "бвд",
                "комбикорм-концентрат",
            ],
        );
        let looks_like_mineral = has_any(
            &search,
            &[
                "chalk",
                "limestone",
                "salt",
                "phosphate",
                "shell",
                "мел",
                "соль",
                "фосфат",
                "известняк",
                "ракуш",
            ],
        );
        let looks_like_oilseed = has_any(
            &search,
            &[
                "meal",
                "cake",
                "шрот",
                "жмых",
                "соев",
                "подсолнеч",
                "рапс",
                "хлопк",
            ],
        );
        let looks_like_protein = has_any(
            &search,
            &[
                "protein",
                "белк",
                "горох",
                "люпин",
                "бобы",
                "дрожж",
                "yeast",
            ],
        );
        let looks_like_grain = has_any(
            &search,
            &[
                "grain",
                "зерно",
                "пшениц",
                "ячмен",
                "кукуруз",
                "овес",
                "овёс",
                "рож",
                "тритикале",
                "bran",
                "отруб",
            ],
        );
        let looks_like_silage = has_any(&search, &["silage", "haylage", "силос", "сенаж"]);

        match self.category_id.as_str() {
            "rough_feeds" => "roughage".to_string(),
            "green_feeds" => {
                if looks_like_silage {
                    "silage".to_string()
                } else {
                    "succulent".to_string()
                }
            }
            "succulent_feeds" => {
                if looks_like_silage {
                    "silage".to_string()
                } else {
                    "succulent".to_string()
                }
            }
            "animal_feeds" => "animal_origin".to_string(),
            "nitrogen_compounds" => "additive".to_string(),
            "mineral_supplements" => {
                if looks_like_premix {
                    "premix".to_string()
                } else {
                    "mineral".to_string()
                }
            }
            "mixed_feeds" => {
                if looks_like_premix {
                    "premix".to_string()
                } else {
                    "concentrate".to_string()
                }
            }
            "industrial_byproducts" => {
                if looks_like_oilseed {
                    "oilseed_meal".to_string()
                } else if looks_like_protein {
                    "protein".to_string()
                } else {
                    "concentrate".to_string()
                }
            }
            "concentrated_feeds" => {
                if looks_like_premix {
                    "premix".to_string()
                } else if looks_like_mineral {
                    "mineral".to_string()
                } else if looks_like_oilseed {
                    "oilseed_meal".to_string()
                } else if looks_like_protein {
                    "protein".to_string()
                } else if looks_like_grain {
                    "grain".to_string()
                } else {
                    "concentrate".to_string()
                }
            }
            _ => "other".to_string(),
        }
    }

    fn dry_matter_pct(&self) -> Option<f64> {
        self.single_value("dry_matter")
            .and_then(convert_dry_matter_pct)
    }

    fn energy_oe(&self, species: &[&str]) -> Option<f64> {
        let as_fed = self
            .animal_value("metabolizable_energy", species)
            .and_then(convert_energy_mj)?;
        let dm_share = self.dry_matter_pct().map(|value| value / 100.0);
        match dm_share {
            Some(share) if share > 0.0 => Some(as_fed / share),
            _ => Some(as_fed),
        }
    }

    fn digestible_protein(&self, species: &[&str]) -> Option<f64> {
        self.animal_value("digestible_protein", species)
            .and_then(convert_grams_per_kg)
    }

    fn notes(&self) -> Option<String> {
        let mut lines = vec![
            NORMALIZED_SOURCE_NOTE.to_string(),
            format!("Original category: {}", self.category_id),
        ];

        if !self.subcategory.ru.trim().is_empty() {
            lines.push(format!(
                "Original subcategory: {}",
                self.subcategory.ru.trim()
            ));
        }
        if !self.parse_errors.is_empty() {
            let fields = self
                .parse_errors
                .iter()
                .take(3)
                .map(|item| item.field.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "Source parse warnings: {}{}",
                self.parse_errors.len(),
                if fields.is_empty() {
                    String::new()
                } else {
                    format!(" ({fields})")
                }
            ));
        }

        Some(lines.join("\n"))
    }

    fn into_feed(self) -> Feed {
        let category = self.mapped_category();
        let name_en = self.normalized_name_en();
        let dry_matter = self.dry_matter_pct();
        let energy_oe_cattle = self.energy_oe(&["cattle"]);
        let energy_oe_pig = self.energy_oe(&["swine"]);
        let energy_oe_poultry = self
            .energy_oe(&["poultry", "chickens", "ducks", "geese", "turkeys"])
            .or_else(|| {
                if Self::uses_monogastric_proxy(&category) {
                    self.energy_oe(&["swine"])
                } else {
                    None
                }
            });
        let koe = self.single_value("feed_units").map(|value| value.value);
        let crude_protein = self
            .single_value("crude_protein")
            .and_then(convert_grams_per_kg);
        let dig_protein_cattle = self.digestible_protein(&["cattle"]);
        let dig_protein_pig = self.digestible_protein(&["swine"]);
        let dig_protein_poultry = self
            .digestible_protein(&["poultry", "chickens"])
            .or_else(|| {
                if Self::uses_monogastric_proxy(&category) {
                    self.digestible_protein(&["swine"])
                } else {
                    None
                }
            });
        let lysine = self.single_value("lysine").and_then(convert_grams_per_kg);
        let methionine_cystine = self
            .single_value("methionine_cystine")
            .and_then(convert_grams_per_kg);
        let crude_fat = self
            .single_value("crude_fat")
            .and_then(convert_grams_per_kg);
        let crude_fiber = self
            .single_value("crude_fiber")
            .and_then(convert_grams_per_kg);
        let starch = self.single_value("starch").and_then(convert_grams_per_kg);
        let sugar = self.single_value("sugars").and_then(convert_grams_per_kg);
        let calcium = self.single_value("calcium").and_then(convert_grams_per_kg);
        let phosphorus = self
            .single_value("phosphorus")
            .and_then(convert_grams_per_kg);
        let magnesium = self
            .single_value("magnesium")
            .and_then(convert_grams_per_kg);
        let potassium = self
            .single_value("potassium")
            .and_then(convert_grams_per_kg);
        let sodium = self.single_value("sodium").and_then(convert_grams_per_kg);
        let sulfur = self.single_value("sulfur").and_then(convert_grams_per_kg);
        let iron = self
            .single_value("iron")
            .and_then(|value| convert_trace_mineral_mg(value, "iron"));
        let copper = self
            .single_value("copper")
            .and_then(|value| convert_trace_mineral_mg(value, "copper"));
        let zinc = self
            .single_value("zinc")
            .and_then(|value| convert_trace_mineral_mg(value, "zinc"));
        let manganese = self
            .single_value("manganese")
            .and_then(|value| convert_trace_mineral_mg(value, "manganese"));
        let cobalt = self
            .single_value("cobalt")
            .and_then(|value| convert_trace_mineral_mg(value, "cobalt"));
        let iodine = self
            .single_value("iodine")
            .and_then(|value| convert_trace_mineral_mg(value, "iodine"));
        let carotene = self.single_value("carotene").and_then(convert_mg_per_kg);
        let vit_d3 = self.single_value("vitamin_d").and_then(convert_vitamin_iu);
        let vit_e = self.single_value("vitamin_e").and_then(convert_mg_per_kg);
        let moisture = dry_matter.map(|value| 100.0 - value);
        let notes = self.notes();
        let parse_warnings = !self.parse_errors.is_empty();
        let source_id = format!("seed:normalized-db:{}", self.id);

        Feed {
            id: None,
            source_id: Some(source_id),
            source_url: Some(self.source_url),
            name_ru: self.name.ru,
            name_en,
            category,
            subcategory: Some(self.subcategory.ru),
            source_category_id: Some(self.category_id),
            source_subcategory_en: if self.subcategory.en.trim().is_empty() {
                None
            } else {
                Some(self.subcategory.en)
            },
            source_nutrition: None,
            dry_matter,
            energy_oe_cattle,
            energy_oe_pig,
            energy_oe_poultry,
            koe,
            crude_protein,
            dig_protein_cattle,
            dig_protein_pig,
            dig_protein_poultry,
            lysine,
            methionine_cystine,
            crude_fat,
            crude_fiber,
            starch,
            sugar,
            calcium,
            phosphorus,
            magnesium,
            potassium,
            sodium,
            sulfur,
            iron,
            copper,
            zinc,
            manganese,
            cobalt,
            iodine,
            carotene,
            vit_d3,
            vit_e,
            moisture,
            feed_conversion: None,
            palatability: None,
            max_inclusion_cattle: None,
            max_inclusion_pig: None,
            max_inclusion_poultry: None,
            price_per_ton: None,
            price_updated_at: None,
            region: self.region_id,
            is_custom: false,
            verified: !parse_warnings,
            notes,
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

fn has_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn normalized_db_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_RELATIVE_PATH)
}

fn normalized_authority_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_AUTHORITY_PATH)
}

fn normalized_shard_paths() -> Vec<PathBuf> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("database/output");
    CATEGORY_FILES.iter().map(|name| base.join(name)).collect()
}

fn convert_dry_matter_pct(value: &UnitValue) -> Option<f64> {
    match normalized_unit(&value.unit).as_str() {
        "g" => Some(value.value / 10.0),
        "%" => Some(value.value),
        "kg" => Some(value.value * 100.0),
        _ => None,
    }
}

fn convert_energy_mj(value: &UnitValue) -> Option<f64> {
    match normalized_unit(&value.unit).as_str() {
        "mj" => Some(value.value),
        _ => None,
    }
}

fn convert_grams_per_kg(value: &UnitValue) -> Option<f64> {
    match normalized_unit(&value.unit).as_str() {
        "g" => Some(value.value),
        "mg" => Some(value.value / 1000.0),
        "kg" => Some(value.value * 1000.0),
        _ => None,
    }
}

fn convert_mg_per_kg(value: &UnitValue) -> Option<f64> {
    match normalized_unit(&value.unit).as_str() {
        "mg" => Some(value.value),
        "g" => Some(value.value * 1000.0),
        "mcg" | "ug" => Some(value.value / 1000.0),
        _ => None,
    }
}

fn convert_trace_mineral_mg(value: &UnitValue, nutrient_key: &str) -> Option<f64> {
    match normalized_unit(&value.unit).as_str() {
        "mg" => Some(value.value),
        // The normalized gov.cap export uses "g" for most iron rows, while the
        // nutrient definition itself is mg and the numeric range matches mg/kg.
        "g" if nutrient_key == "iron" => Some(value.value),
        "g" => Some(value.value * 1000.0),
        "mcg" | "ug" => Some(value.value / 1000.0),
        _ => None,
    }
}

fn convert_vitamin_iu(value: &UnitValue) -> Option<f64> {
    match normalized_unit(&value.unit).as_str() {
        "iu" => Some(value.value),
        "thousand iu" => Some(value.value * 1000.0),
        "million iu" => Some(value.value * 1_000_000.0),
        _ => None,
    }
}

fn normalized_unit(unit: &str) -> String {
    unit.trim().to_lowercase()
}

fn is_plausible_energy(value: f64, max: f64) -> bool {
    value.is_finite() && (MIN_PLAUSIBLE_ENERGY_MJ_PER_KG_DM..=max).contains(&value)
}

fn append_note_once(notes: &mut Option<String>, line: &str) {
    match notes {
        Some(existing) => {
            if !existing.lines().any(|existing_line| existing_line == line) {
                existing.push('\n');
                existing.push_str(line);
            }
        }
        None => *notes = Some(line.to_string()),
    }
}

fn source_animal_energy_as_fed(feed: &Feed, species: &str) -> Option<f64> {
    feed.source_nutrition
        .as_ref()?
        .get("metabolizable_energy")?
        .get(species)?
        .get("value")?
        .as_f64()
}

fn corrected_monogastric_energy_from_source(feed: &Feed, species: &str) -> Option<f64> {
    let raw_as_fed = source_animal_energy_as_fed(feed, species)?;
    let dm_share = feed
        .dry_matter
        .map(|value| value / 100.0)
        .filter(|value| *value > 0.0)?;

    ENERGY_DECIMAL_SHIFT_DIVISORS
        .iter()
        .filter_map(|divisor| {
            let normalized = (raw_as_fed / divisor) / dm_share;
            is_plausible_energy(normalized, MAX_PLAUSIBLE_MONOGASTRIC_ENERGY_MJ_PER_KG_DM)
                .then_some(normalized)
        })
        .max_by(|left, right| left.total_cmp(right))
}

fn reconcile_monogastric_energy(
    feed: &Feed,
    current: Option<f64>,
    species: &str,
    fallback: Option<f64>,
) -> Option<f64> {
    match current {
        Some(value)
            if is_plausible_energy(value, MAX_PLAUSIBLE_MONOGASTRIC_ENERGY_MJ_PER_KG_DM) =>
        {
            Some(value)
        }
        _ => corrected_monogastric_energy_from_source(feed, species).or_else(|| {
            fallback.filter(|value| {
                is_plausible_energy(*value, MAX_PLAUSIBLE_MONOGASTRIC_ENERGY_MJ_PER_KG_DM)
            })
        }),
    }
}

fn sanitize_seed_feed(mut feed: Feed) -> Feed {
    let is_normalized_seed = feed
        .source_id
        .as_deref()
        .is_some_and(|source| source.starts_with("seed:normalized-db:"));
    if !is_normalized_seed {
        return feed;
    }

    let original_pig = feed.energy_oe_pig;
    feed.energy_oe_pig =
        reconcile_monogastric_energy(&feed, original_pig, "swine", feed.energy_oe_cattle);
    if original_pig != feed.energy_oe_pig {
        append_note_once(
            &mut feed.notes,
            "Seed-load plausibility guard corrected implausible swine energy density from the normalized source family.",
        );
    }

    let original_poultry = feed.energy_oe_poultry;
    let poultry_fallback = feed.energy_oe_pig.or(feed.energy_oe_cattle);
    feed.energy_oe_poultry =
        reconcile_monogastric_energy(&feed, original_poultry, "poultry", poultry_fallback);
    if original_poultry != feed.energy_oe_poultry {
        append_note_once(
            &mut feed.notes,
            "Seed-load plausibility guard corrected implausible poultry energy density from the normalized source family.",
        );
    }

    feed
}

pub fn load_workspace_seed_feeds() -> Result<Option<Vec<Feed>>> {
    let authority_path = normalized_authority_path();
    if authority_path.exists() {
        let feeds = load_seed_feeds_from_authority_path(&authority_path)?;
        return Ok(Some(feeds));
    }

    let consolidated_path = normalized_db_path();
    if consolidated_path.exists() {
        let feeds = load_seed_feeds_from_path(&consolidated_path)?;
        return Ok(Some(feeds));
    }

    let existing_shards = normalized_shard_paths()
        .into_iter()
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    if !existing_shards.is_empty() {
        let feeds = load_seed_feeds_from_shard_paths(&existing_shards)?;
        return Ok(Some(feeds));
    }

    Ok(None)
}

pub fn load_seed_feeds_from_authority_path(path: &Path) -> Result<Vec<Feed>> {
    let raw = fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read generated feed authority at {}",
            path.display()
        )
    })?;
    load_seed_feeds_from_authority_str(&raw)
}

pub fn load_seed_feeds_from_authority_str(raw: &str) -> Result<Vec<Feed>> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_str::<Feed>(line)
                .map(sanitize_seed_feed)
                .context("Failed to parse generated feed authority JSONL line")
        })
        .collect()
}

pub fn load_seed_feeds_from_path(path: &Path) -> Result<Vec<Feed>> {
    let raw = fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read normalized feed database at {}",
            path.display()
        )
    })?;
    load_seed_feeds_from_str(&raw)
}

pub fn load_seed_feeds_from_str(raw: &str) -> Result<Vec<Feed>> {
    let database: NormalizedFeedDatabase = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .context("Failed to parse normalized feed database JSON")?;
    Ok(database
        .feeds
        .into_iter()
        .map(RawFeed::into_feed)
        .map(sanitize_seed_feed)
        .collect())
}

pub fn load_seed_feeds_from_shard_paths(paths: &[PathBuf]) -> Result<Vec<Feed>> {
    let mut feeds = Vec::new();
    for path in paths {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read category shard at {}", path.display()))?;
        feeds.extend(load_seed_feeds_from_shard_str(&raw)?);
    }
    Ok(feeds)
}

pub fn load_seed_feeds_from_shard_str(raw: &str) -> Result<Vec<Feed>> {
    let payload: CategoryFeedPayload = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .context("Failed to parse normalized feed category shard JSON")?;
    Ok(payload
        .feeds
        .into_iter()
        .map(RawFeed::into_feed)
        .map(sanitize_seed_feed)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{
        load_seed_feeds_from_authority_str, load_seed_feeds_from_shard_str,
        load_seed_feeds_from_str,
    };

    #[test]
    fn adapter_maps_units_and_species_specific_fields() {
        let json = r#"
        {
          "feeds": [
            {
              "id": "n1",
              "name": { "ru": "Тестовый зеленый корм", "en": "Тестовый зеленый корм" },
              "category_id": "green_feeds",
              "subcategory": { "ru": "Пастбищные травы", "en": "Pasture grasses" },
              "region_id": "ru-central",
              "nutrition": {
                "feed_units": { "value": 0.72, "unit": "" },
                "dry_matter": { "value": 320, "unit": "g" },
                "metabolizable_energy": {
                  "cattle": { "value": 2.88, "unit": "MJ" },
                  "swine": { "value": 3.20, "unit": "MJ" },
                  "poultry": { "value": 3.52, "unit": "MJ" }
                },
                "crude_protein": { "value": 145, "unit": "g" },
                "digestible_protein": {
                  "cattle": { "value": 102, "unit": "g" },
                  "swine": { "value": 90, "unit": "g" },
                  "poultry": { "value": 87, "unit": "g" }
                },
                "lysine": { "value": 7.4, "unit": "g" },
                "methionine_cystine": { "value": 3.8, "unit": "g" },
                "sugars": { "value": 84, "unit": "g" },
                "starch": { "value": 24, "unit": "g" },
                "calcium": { "value": 6.2, "unit": "g" },
                "phosphorus": { "value": 3.1, "unit": "g" },
                "vitamin_d": { "value": 12.5, "unit": "thousand IU" },
                "vitamin_e": { "value": 45, "unit": "mg" },
                "iron": { "value": 126, "unit": "g" }
              },
              "source_url": "https://example.test/n1",
              "parse_errors": [{ "field": "crude_fiber" }]
            }
          ]
        }
        "#;

        let feeds = load_seed_feeds_from_str(json).unwrap();
        let feed = &feeds[0];
        assert_eq!(feed.source_id.as_deref(), Some("seed:normalized-db:n1"));
        assert_eq!(feed.name_en, None);
        assert_eq!(feed.category, "succulent");
        assert_eq!(feed.subcategory.as_deref(), Some("Пастбищные травы"));
        assert_eq!(feed.region.as_deref(), Some("ru-central"));
        assert_eq!(feed.dry_matter, Some(32.0));
        assert_eq!(feed.koe, Some(0.72));
        assert_eq!(feed.energy_oe_cattle, Some(9.0));
        assert_eq!(feed.energy_oe_pig, Some(10.0));
        assert_eq!(feed.energy_oe_poultry, Some(11.0));
        assert_eq!(feed.dig_protein_cattle, Some(102.0));
        assert_eq!(feed.dig_protein_pig, Some(90.0));
        assert_eq!(feed.dig_protein_poultry, Some(87.0));
        assert_eq!(feed.sugar, Some(84.0));
        assert_eq!(feed.vit_d3, Some(12_500.0));
        assert_eq!(feed.vit_e, Some(45.0));
        assert_eq!(feed.iron, Some(126.0));
        assert_eq!(feed.moisture, Some(68.0));
        assert!(!feed.verified);
        assert!(feed
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("Source parse warnings: 1"));
    }

    #[test]
    fn adapter_uses_swine_proxy_for_poultry_monogastric_inputs_when_source_lacks_poultry_rows() {
        let json = r#"
        {
          "feeds": [
            {
              "id": "n2",
              "name": { "ru": "Sunflower grain", "en": "Sunflower grain" },
              "category_id": "concentrated_feeds",
              "subcategory": { "ru": "Grain", "en": "Grain" },
              "nutrition": {
                "dry_matter": { "value": 877, "unit": "g" },
                "metabolizable_energy": {
                  "cattle": { "value": 11.2, "unit": "MJ" },
                  "swine": { "value": 14.6, "unit": "MJ" }
                },
                "digestible_protein": {
                  "cattle": { "value": 137.0, "unit": "g" },
                  "swine": { "value": 150.7, "unit": "g" }
                },
                "crude_protein": { "value": 154.0, "unit": "g" },
                "crude_fat": { "value": 244.0, "unit": "g" }
              },
              "source_url": "https://example.test/n2"
            }
          ]
        }
        "#;

        let feeds = load_seed_feeds_from_str(json).unwrap();
        let feed = &feeds[0];
        assert_eq!(feed.category, "grain");
        assert_eq!(feed.energy_oe_pig, Some(14.6 / 0.877));
        assert_eq!(feed.energy_oe_poultry, Some(14.6 / 0.877));
        assert_eq!(feed.dig_protein_pig, Some(150.7));
        assert_eq!(feed.dig_protein_poultry, Some(150.7));
    }

    #[test]
    fn authority_loader_corrects_implausible_monogastric_energy_from_source_payload() {
        let authority = r#"{"id":1703058380,"source_id":"seed:normalized-db:n349","name_ru":"Топинамбур сушенный","category":"concentrate","source_nutrition":{"metabolizable_energy":{"cattle":{"value":10.49,"unit":"MJ"},"swine":{"value":1140.0,"unit":"MJ"}},"dry_matter":{"value":836.0,"unit":"g"}},"dry_matter":83.6,"energy_oe_cattle":12.547846889952154,"energy_oe_pig":1363.6363636363637,"energy_oe_poultry":1363.6363636363637,"verified":true,"is_custom":false}"#;

        let feeds = load_seed_feeds_from_authority_str(authority).unwrap();
        let feed = &feeds[0];
        assert!((feed.energy_oe_pig.unwrap() - 13.636363636363637).abs() < 1e-9);
        assert!((feed.energy_oe_poultry.unwrap() - 13.636363636363637).abs() < 1e-9);
        assert!(feed
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("corrected implausible swine energy density"));
    }

    #[test]
    fn adapter_normalizes_categories_for_future_ui_and_solver_paths() {
        let json = r#"
        {
          "feeds": [
            {
              "id": "m1",
              "name": { "ru": "Премикс для свиней 1%", "en": "Swine premix 1%" },
              "category_id": "mineral_supplements",
              "subcategory": { "ru": "Премиксы", "en": "Premixes" },
              "nutrition": {},
              "source_url": "https://example.test/m1"
            },
            {
              "id": "c1",
              "name": { "ru": "Подсолнечный шрот", "en": "Sunflower meal" },
              "category_id": "concentrated_feeds",
              "subcategory": { "ru": "Белковые концентраты", "en": "Protein concentrates" },
              "nutrition": {},
              "source_url": "https://example.test/c1"
            }
          ]
        }
        "#;

        let feeds = load_seed_feeds_from_str(json).unwrap();
        assert_eq!(feeds[0].category, "premix");
        assert_eq!(feeds[1].category, "oilseed_meal");
        assert_eq!(feeds[0].name_en.as_deref(), Some("Swine premix 1%"));
    }

    #[test]
    fn authority_jsonl_loader_restores_generated_feed_records() {
        let authority = r#"{"id":101,"source_id":"seed:normalized-db:n1","name_ru":"Поваренная соль","category":"mineral","verified":true,"is_custom":false}
{"id":102,"source_id":"seed:normalized-db:n2","name_ru":"Дерть ячменная","category":"grain","subcategory":"Зерновые культуры","verified":false,"is_custom":false}"#;

        let feeds = load_seed_feeds_from_authority_str(authority).unwrap();
        assert_eq!(feeds.len(), 2);
        assert_eq!(feeds[0].name_ru, "Поваренная соль");
        assert_eq!(feeds[0].source_id.as_deref(), Some("seed:normalized-db:n1"));
        assert_eq!(feeds[1].subcategory.as_deref(), Some("Зерновые культуры"));
        assert!(!feeds[1].verified);
    }

    #[test]
    fn category_shard_loader_maps_source_family_payload() {
        let shard = r#"
        {
          "category_id": "industrial_byproducts",
          "count": 1,
          "feeds": [
            {
              "id": "b1",
              "name": { "ru": "Свекловичный сухой", "en": "Dried sugar beet" },
              "category_id": "industrial_byproducts",
              "subcategory": { "ru": "Жом", "en": "Beet pulp" },
              "nutrition": {
                "dry_matter": { "value": 910, "unit": "g" },
                "crude_fiber": { "value": 185, "unit": "g" }
              },
              "source_url": "https://example.test/b1"
            }
          ]
        }
        "#;

        let feeds = load_seed_feeds_from_shard_str(shard).unwrap();
        assert_eq!(feeds.len(), 1);
        assert_eq!(feeds[0].name_ru, "Свекловичный сухой");
        assert_eq!(feeds[0].subcategory.as_deref(), Some("Жом"));
        assert_eq!(feeds[0].dry_matter, Some(91.0));
        assert_eq!(feeds[0].crude_fiber, Some(185.0));
    }
}
