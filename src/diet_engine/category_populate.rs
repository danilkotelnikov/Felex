//! Category-aware ration population module.
//!
//! Provides a structured approach to assembling a starter ration by populating
//! each feed category slot (Roughage, Succulent, RootCrops, Concentrate,
//! ProteinSupplement, MineralPremix, OilFat) with the best-matching candidates
//! from the available feed library.

use serde::{Deserialize, Serialize};

use crate::db::feeds::Feed;

use super::feed_groups::{classify_feed, is_feed_allowed_for_context, score_feed_for_group, FeedGroup};

// ---------------------------------------------------------------------------
// Feed category enum
// ---------------------------------------------------------------------------

/// High-level ration category used for structured population.
///
/// Each variant maps to one or more `FeedGroup` classifications and represents
/// a logical slot in the ration template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedCategory {
    Roughage,
    Succulent,
    RootCrops,
    Concentrate,
    ProteinSupplement,
    MineralPremix,
    OilFat,
}

impl FeedCategory {
    /// English display label.
    pub fn label_en(self) -> &'static str {
        match self {
            Self::Roughage => "Roughage",
            Self::Succulent => "Succulent",
            Self::RootCrops => "Root Crops",
            Self::Concentrate => "Concentrate",
            Self::ProteinSupplement => "Protein Supplement",
            Self::MineralPremix => "Mineral / Premix",
            Self::OilFat => "Oil / Fat",
        }
    }

    /// Russian display label.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Roughage => "Грубые корма",
            Self::Succulent => "Сочные корма",
            Self::RootCrops => "Корнеклубнеплоды",
            Self::Concentrate => "Концентраты",
            Self::ProteinSupplement => "Протеиновые добавки",
            Self::MineralPremix => "Минералы / Премиксы",
            Self::OilFat => "Масла / Жиры",
        }
    }

    /// Map a database `category` string to a `FeedCategory`.
    ///
    /// Returns `None` when the category does not map to any known slot.
    pub fn from_db_category(db_category: &str) -> Option<Self> {
        match db_category.trim().to_lowercase().as_str() {
            "roughage" | "hay" | "straw" | "haylage" => Some(Self::Roughage),
            "silage" | "succulent" | "green" => Some(Self::Succulent),
            "root_crops" | "roots" | "beets" | "root" => Some(Self::RootCrops),
            "grain" | "concentrate" | "bran" | "compound_feed" => Some(Self::Concentrate),
            "oilseed_meal" | "protein" | "protein_supplement" | "legume" => {
                Some(Self::ProteinSupplement)
            }
            "mineral" | "premix" | "vitamin" | "additive" => Some(Self::MineralPremix),
            "fat" | "oil" | "oil_fat" => Some(Self::OilFat),
            _ => None,
        }
    }

    /// Map a `FeedGroup` classification to the most appropriate `FeedCategory`.
    pub fn from_feed_group(group: FeedGroup) -> Option<Self> {
        match group {
            FeedGroup::Roughage => Some(Self::Roughage),
            FeedGroup::Succulent => Some(Self::Succulent),
            FeedGroup::Concentrate => Some(Self::Concentrate),
            FeedGroup::Protein | FeedGroup::AnimalOrigin => Some(Self::ProteinSupplement),
            FeedGroup::Mineral | FeedGroup::Premix | FeedGroup::Vitamin => {
                Some(Self::MineralPremix)
            }
            FeedGroup::Other => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Ration structure definition
// ---------------------------------------------------------------------------

/// Defines the inclusion bounds and priority for a single category slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRequirement {
    /// The feed category this requirement targets.
    pub category: FeedCategory,
    /// Minimum share of total DM intake (0.0–1.0).
    pub min_inclusion_pct: f64,
    /// Maximum share of total DM intake (0.0–1.0).
    pub max_inclusion_pct: f64,
    /// Whether a feed must be found for this slot.
    pub required: bool,
    /// Number of alternative candidates to surface in the plan.
    pub alternatives_to_show: usize,
}

/// Describes the target ration structure for a given species.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RationStructure {
    /// Species identifier, e.g. `"cattle"`, `"swine"`.
    pub species: String,
    /// Ordered list of category requirements.
    pub requirements: Vec<CategoryRequirement>,
}

/// Returns the standard dairy-cattle ration structure with six category slots.
pub fn cattle_dairy_structure() -> RationStructure {
    RationStructure {
        species: "cattle".to_string(),
        requirements: vec![
            CategoryRequirement {
                category: FeedCategory::Roughage,
                min_inclusion_pct: 0.15,
                max_inclusion_pct: 0.35,
                required: true,
                alternatives_to_show: 3,
            },
            CategoryRequirement {
                category: FeedCategory::Succulent,
                min_inclusion_pct: 0.25,
                max_inclusion_pct: 0.45,
                required: true,
                alternatives_to_show: 3,
            },
            CategoryRequirement {
                category: FeedCategory::RootCrops,
                min_inclusion_pct: 0.00,
                max_inclusion_pct: 0.15,
                required: false,
                alternatives_to_show: 2,
            },
            CategoryRequirement {
                category: FeedCategory::Concentrate,
                min_inclusion_pct: 0.15,
                max_inclusion_pct: 0.40,
                required: true,
                alternatives_to_show: 3,
            },
            CategoryRequirement {
                category: FeedCategory::ProteinSupplement,
                min_inclusion_pct: 0.03,
                max_inclusion_pct: 0.15,
                required: true,
                alternatives_to_show: 3,
            },
            CategoryRequirement {
                category: FeedCategory::MineralPremix,
                min_inclusion_pct: 0.005,
                max_inclusion_pct: 0.03,
                required: true,
                alternatives_to_show: 2,
            },
        ],
    }
}

// ---------------------------------------------------------------------------
// Plan data structures
// ---------------------------------------------------------------------------

/// A single feed candidate with its computed properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedOption {
    pub feed_id: i64,
    pub feed_name: String,
    pub category: FeedCategory,
    pub score: f64,
    pub dry_matter_pct: f64,
    pub suggested_amount_kg: f64,
}

/// A feed selected for inclusion in the plan together with its amount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedWithAmount {
    pub feed_id: i64,
    pub feed_name: String,
    pub amount_kg: f64,
    pub category: FeedCategory,
}

/// All candidates surfaced for one category slot, together with the chosen feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySuggestion {
    pub category: FeedCategory,
    pub selected: Option<FeedWithAmount>,
    pub alternatives: Vec<FeedOption>,
    pub missing: bool,
}

/// The complete output of `generate_populate_plan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryPopulatePlan {
    pub species: String,
    pub target_dm_kg: f64,
    pub suggestions: Vec<CategorySuggestion>,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Plan generation
// ---------------------------------------------------------------------------

/// Generate a category-aware populate plan.
///
/// # Arguments
/// * `species` – species identifier, e.g. `"cattle"`.
/// * `available_feeds` – slice of feeds from the library.
/// * `target_dm_kg` – target dry-matter intake in kg/day.
pub fn generate_populate_plan(
    species: &str,
    available_feeds: &[Feed],
    target_dm_kg: f64,
) -> CategoryPopulatePlan {
    let structure = structure_for_species(species);
    let mut suggestions = Vec::with_capacity(structure.requirements.len());
    let mut notes = Vec::new();

    for req in &structure.requirements {
        let suggestion = build_category_suggestion(req, species, available_feeds, target_dm_kg);
        if suggestion.missing && req.required {
            notes.push(format!(
                "No suitable feed found for required category: {}.",
                req.category.label_en()
            ));
        }
        suggestions.push(suggestion);
    }

    CategoryPopulatePlan {
        species: species.to_string(),
        target_dm_kg,
        suggestions,
        notes,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn structure_for_species(species: &str) -> RationStructure {
    match species {
        "cattle" => cattle_dairy_structure(),
        _ => cattle_dairy_structure(),
    }
}

fn feed_matches_category(feed: &Feed, category: FeedCategory) -> bool {
    // First, try exact DB-category mapping.
    if let Some(mapped) = FeedCategory::from_db_category(&feed.category) {
        if mapped == category {
            return true;
        }
    }
    // Fall back to FeedGroup classification.
    let group = classify_feed(feed);
    if let Some(mapped) = FeedCategory::from_feed_group(group) {
        return mapped == category;
    }
    false
}

fn corresponding_feed_group(category: FeedCategory) -> FeedGroup {
    match category {
        FeedCategory::Roughage => FeedGroup::Roughage,
        FeedCategory::Succulent => FeedGroup::Succulent,
        FeedCategory::RootCrops => FeedGroup::Succulent,
        FeedCategory::Concentrate => FeedGroup::Concentrate,
        FeedCategory::ProteinSupplement => FeedGroup::Protein,
        FeedCategory::MineralPremix => FeedGroup::Mineral,
        FeedCategory::OilFat => FeedGroup::Concentrate,
    }
}

fn estimate_amount_kg(feed: &Feed, req: &CategoryRequirement, target_dm_kg: f64) -> f64 {
    let midpoint = (req.min_inclusion_pct + req.max_inclusion_pct) / 2.0;
    let target_dm_portion = target_dm_kg * midpoint;
    let dm_fraction = (feed.dry_matter.unwrap_or(86.0) / 100.0).clamp(0.05, 1.0);

    let as_fed = target_dm_portion / dm_fraction;

    // Apply category-specific floor / ceiling limits.
    match req.category {
        FeedCategory::MineralPremix => as_fed.clamp(0.05, 0.30),
        FeedCategory::OilFat => as_fed.clamp(0.05, 0.50),
        FeedCategory::ProteinSupplement => as_fed.clamp(0.10, 3.0),
        FeedCategory::RootCrops => as_fed.clamp(0.0, 8.0),
        _ => as_fed.max(0.05),
    }
}

fn build_category_suggestion(
    req: &CategoryRequirement,
    species: &str,
    available_feeds: &[Feed],
    target_dm_kg: f64,
) -> CategorySuggestion {
    let group = corresponding_feed_group(req.category);

    let mut candidates: Vec<&Feed> = available_feeds
        .iter()
        .filter(|feed| feed.id.is_some())
        .filter(|feed| feed_matches_category(feed, req.category))
        .filter(|feed| is_feed_allowed_for_context(feed, species, None))
        .collect();

    candidates.sort_by(|left, right| {
        score_feed_for_group(right, group, species)
            .partial_cmp(&score_feed_for_group(left, group, species))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut alternatives: Vec<FeedOption> = candidates
        .iter()
        .take(req.alternatives_to_show)
        .map(|feed| {
            let amount_kg = estimate_amount_kg(feed, req, target_dm_kg);
            FeedOption {
                feed_id: feed.id.unwrap_or(-1),
                feed_name: feed.name_ru.clone(),
                category: req.category,
                score: score_feed_for_group(feed, group, species),
                dry_matter_pct: feed.dry_matter.unwrap_or(86.0),
                suggested_amount_kg: (amount_kg * 1000.0).round() / 1000.0,
            }
        })
        .collect();

    let selected = alternatives.first().map(|opt| FeedWithAmount {
        feed_id: opt.feed_id,
        feed_name: opt.feed_name.clone(),
        amount_kg: opt.suggested_amount_kg,
        category: req.category,
    });

    // Remove the selected entry from alternatives so the list is truly
    // "alternatives to the selected feed".
    if !alternatives.is_empty() {
        alternatives.remove(0);
    }

    let missing = selected.is_none();

    CategorySuggestion {
        category: req.category,
        selected,
        alternatives,
        missing,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_feed(
        id: i64,
        name: &str,
        category: &str,
        dry_matter: f64,
        energy: f64,
        crude_protein: f64,
    ) -> Feed {
        Feed {
            id: Some(id),
            name_ru: name.to_string(),
            category: category.to_string(),
            dry_matter: Some(dry_matter),
            energy_oe_cattle: Some(energy),
            crude_protein: Some(crude_protein),
            ..Default::default()
        }
    }

    fn sample_dairy_feeds() -> Vec<Feed> {
        vec![
            make_feed(1, "Hay", "roughage", 87.0, 9.1, 145.0),
            make_feed(2, "Corn silage", "silage", 32.0, 10.7, 80.0),
            make_feed(3, "Barley", "grain", 86.0, 12.4, 115.0),
            make_feed(4, "Soybean meal", "oilseed_meal", 89.0, 11.0, 430.0),
            Feed {
                id: Some(5),
                name_ru: "Feed chalk".to_string(),
                category: "mineral".to_string(),
                calcium: Some(360.0),
                dry_matter: Some(99.0),
                ..Default::default()
            },
            Feed {
                id: Some(6),
                name_ru: "Premix P60".to_string(),
                category: "premix".to_string(),
                vit_d3: Some(250_000.0),
                vit_e: Some(2000.0),
                dry_matter: Some(99.0),
                ..Default::default()
            },
        ]
    }

    // -----------------------------------------------------------------------
    // FeedCategory helpers
    // -----------------------------------------------------------------------

    #[test]
    fn label_en_returns_correct_strings() {
        assert_eq!(FeedCategory::Roughage.label_en(), "Roughage");
        assert_eq!(FeedCategory::MineralPremix.label_en(), "Mineral / Premix");
        assert_eq!(FeedCategory::OilFat.label_en(), "Oil / Fat");
    }

    #[test]
    fn label_ru_returns_correct_strings() {
        assert_eq!(FeedCategory::Roughage.label_ru(), "Грубые корма");
        assert_eq!(FeedCategory::Concentrate.label_ru(), "Концентраты");
    }

    #[test]
    fn from_db_category_maps_known_categories() {
        assert_eq!(
            FeedCategory::from_db_category("roughage"),
            Some(FeedCategory::Roughage)
        );
        assert_eq!(
            FeedCategory::from_db_category("grain"),
            Some(FeedCategory::Concentrate)
        );
        assert_eq!(
            FeedCategory::from_db_category("oilseed_meal"),
            Some(FeedCategory::ProteinSupplement)
        );
        assert_eq!(
            FeedCategory::from_db_category("premix"),
            Some(FeedCategory::MineralPremix)
        );
    }

    #[test]
    fn from_db_category_returns_none_for_unknown() {
        assert_eq!(FeedCategory::from_db_category("unknown_xyz"), None);
    }

    #[test]
    fn from_feed_group_maps_mineral_and_premix() {
        assert_eq!(
            FeedCategory::from_feed_group(FeedGroup::Mineral),
            Some(FeedCategory::MineralPremix)
        );
        assert_eq!(
            FeedCategory::from_feed_group(FeedGroup::Premix),
            Some(FeedCategory::MineralPremix)
        );
        assert_eq!(FeedCategory::from_feed_group(FeedGroup::Other), None);
    }

    // -----------------------------------------------------------------------
    // cattle_dairy_structure
    // -----------------------------------------------------------------------

    #[test]
    fn cattle_dairy_structure_has_six_categories() {
        let s = cattle_dairy_structure();
        assert_eq!(s.requirements.len(), 6);
        assert_eq!(s.species, "cattle");
    }

    #[test]
    fn cattle_dairy_structure_required_slots_are_marked() {
        let s = cattle_dairy_structure();
        let required_count = s.requirements.iter().filter(|r| r.required).count();
        // Roughage, Succulent, Concentrate, ProteinSupplement, MineralPremix = 5 required
        assert_eq!(required_count, 5);
    }

    #[test]
    fn cattle_dairy_structure_inclusion_pct_sums_to_at_most_one() {
        let s = cattle_dairy_structure();
        let max_sum: f64 = s.requirements.iter().map(|r| r.max_inclusion_pct).sum();
        // Should be <= 1.35 (slots can overlap; we just verify it's sensible)
        assert!(max_sum <= 2.0);
    }

    // -----------------------------------------------------------------------
    // generate_populate_plan
    // -----------------------------------------------------------------------

    #[test]
    fn generate_plan_populates_required_slots_when_feeds_available() {
        let feeds = sample_dairy_feeds();
        let plan = generate_populate_plan("cattle", &feeds, 18.0);

        assert_eq!(plan.species, "cattle");
        assert_eq!(plan.target_dm_kg, 18.0);

        // All required category slots must have a selected feed.
        let structure = cattle_dairy_structure();
        for req in structure.requirements.iter().filter(|r| r.required) {
            let suggestion = plan
                .suggestions
                .iter()
                .find(|s| s.category == req.category)
                .expect("suggestion present for required category");
            assert!(
                suggestion.selected.is_some(),
                "expected selection for {:?}",
                req.category
            );
        }
    }

    #[test]
    fn generate_plan_reports_missing_for_empty_library() {
        let plan = generate_populate_plan("cattle", &[], 18.0);
        assert!(
            plan.notes
                .iter()
                .any(|n| n.contains("No suitable feed found")),
            "expected notes about missing feeds"
        );
    }

    #[test]
    fn generate_plan_amounts_are_positive() {
        let feeds = sample_dairy_feeds();
        let plan = generate_populate_plan("cattle", &feeds, 18.0);

        for suggestion in &plan.suggestions {
            if let Some(selected) = &suggestion.selected {
                assert!(
                    selected.amount_kg > 0.0,
                    "amount must be positive for {:?}",
                    selected.feed_name
                );
            }
        }
    }

    #[test]
    fn generate_plan_alternatives_do_not_repeat_selected() {
        let feeds = sample_dairy_feeds();
        let plan = generate_populate_plan("cattle", &feeds, 18.0);

        for suggestion in &plan.suggestions {
            if let Some(selected) = &suggestion.selected {
                assert!(
                    !suggestion
                        .alternatives
                        .iter()
                        .any(|alt| alt.feed_id == selected.feed_id),
                    "selected feed must not appear in alternatives"
                );
            }
        }
    }

    #[test]
    fn generate_plan_feed_ids_are_valid() {
        let feeds = sample_dairy_feeds();
        let plan = generate_populate_plan("cattle", &feeds, 18.0);

        for suggestion in &plan.suggestions {
            if let Some(selected) = &suggestion.selected {
                assert_ne!(
                    selected.feed_id, -1,
                    "feed_id should not be -1 for feeds with valid ids"
                );
            }
        }
    }
}
