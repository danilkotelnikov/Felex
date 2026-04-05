//! Nutrient calculation module grounded in the implemented feed data surface.

use crate::{db::rations::RationItem, norms::AnimalNorm};
use serde::{Deserialize, Serialize};

/// Nutrient summary for a ration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NutrientSummary {
    // Total amounts.
    pub total_weight_kg: f64,
    pub total_dm_kg: f64,

    // Energy.
    pub energy_eke: f64,
    pub energy_oe_cattle: f64,
    pub energy_oe_pig: f64,
    pub energy_oe_poultry: f64,

    // Protein.
    pub crude_protein: f64,
    pub dig_protein_cattle: f64,
    pub dig_protein_pig: f64,
    pub dig_protein_poultry: f64,
    pub lysine: f64,
    pub methionine_cystine: f64,

    // Fat, fiber, carbohydrate.
    pub crude_fat: f64,
    pub crude_fiber: f64,
    pub starch: f64,
    pub sugar: f64,

    // Minerals.
    pub calcium: f64,
    pub phosphorus: f64,
    pub ca_p_ratio: f64,
    pub magnesium: f64,
    pub potassium: f64,
    pub sodium: f64,
    pub sulfur: f64,
    pub iron: f64,
    pub copper: f64,
    pub zinc: f64,
    pub manganese: f64,
    pub cobalt: f64,
    pub iodine: f64,

    // Vitamins and provitamins.
    pub vit_d3: f64,
    pub vit_e: f64,
    pub carotene: f64,

    // Derived composition metrics.
    pub dm_pct: f64,
    pub cp_pct_dm: f64,
    pub dig_protein_cattle_pct_cp: f64,
    pub starch_pct_dm: f64,
}

/// Calculate nutrients from ration items.
pub fn calculate_nutrients(items: &[RationItem]) -> NutrientSummary {
    let mut summary = NutrientSummary::default();

    for item in items {
        let Some(feed) = item.feed.as_ref() else {
            continue;
        };

        let kg = item.amount_kg;
        let dm_share = feed.dry_matter.unwrap_or(86.0) / 100.0;
        let dm_kg = kg * dm_share;

        summary.total_weight_kg += kg;
        summary.total_dm_kg += dm_kg;

        summary.energy_oe_cattle += feed.energy_oe_cattle.unwrap_or(0.0) * dm_kg;
        summary.energy_oe_pig += feed.energy_oe_pig.unwrap_or(0.0) * dm_kg;
        summary.energy_oe_poultry += feed.energy_oe_poultry.unwrap_or(0.0) * dm_kg;
        summary.energy_eke += feed.energy_oe_cattle.unwrap_or(0.0) * dm_kg / 10.5;

        summary.crude_protein += feed.crude_protein.unwrap_or(0.0) * kg;
        summary.dig_protein_cattle += feed.dig_protein_cattle.unwrap_or(0.0) * kg;
        summary.dig_protein_pig += feed.dig_protein_pig.unwrap_or(0.0) * kg;
        summary.dig_protein_poultry += feed.dig_protein_poultry.unwrap_or(0.0) * kg;
        summary.lysine += feed.lysine.unwrap_or(0.0) * kg;
        summary.methionine_cystine += feed.methionine_cystine.unwrap_or(0.0) * kg;

        summary.crude_fat += feed.crude_fat.unwrap_or(0.0) * kg;
        summary.crude_fiber += feed.crude_fiber.unwrap_or(0.0) * kg;
        summary.starch += feed.starch.unwrap_or(0.0) * kg;
        summary.sugar += feed.sugar.unwrap_or(0.0) * kg;

        summary.calcium += feed.calcium.unwrap_or(0.0) * kg;
        summary.phosphorus += feed.phosphorus.unwrap_or(0.0) * kg;
        summary.magnesium += feed.magnesium.unwrap_or(0.0) * kg;
        summary.potassium += feed.potassium.unwrap_or(0.0) * kg;
        summary.sodium += feed.sodium.unwrap_or(0.0) * kg;
        summary.sulfur += feed.sulfur.unwrap_or(0.0) * kg;
        summary.iron += feed.iron.unwrap_or(0.0) * kg;
        summary.copper += feed.copper.unwrap_or(0.0) * kg;
        summary.zinc += feed.zinc.unwrap_or(0.0) * kg;
        summary.manganese += feed.manganese.unwrap_or(0.0) * kg;
        summary.cobalt += feed.cobalt.unwrap_or(0.0) * kg;
        summary.iodine += feed.iodine.unwrap_or(0.0) * kg;

        summary.vit_d3 += feed.vit_d3.unwrap_or(0.0) * kg;
        summary.vit_e += feed.vit_e.unwrap_or(0.0) * kg;
        summary.carotene += feed.carotene.unwrap_or(0.0) * kg;
    }

    if summary.total_weight_kg > 0.0 {
        summary.dm_pct = (summary.total_dm_kg / summary.total_weight_kg) * 100.0;
    }
    if summary.total_dm_kg > 0.0 {
        summary.cp_pct_dm = (summary.crude_protein / 1000.0 / summary.total_dm_kg) * 100.0;
        summary.starch_pct_dm = (summary.starch / 1000.0 / summary.total_dm_kg) * 100.0;
    }
    if summary.crude_protein > 0.0 {
        summary.dig_protein_cattle_pct_cp =
            (summary.dig_protein_cattle / summary.crude_protein) * 100.0;
    }
    if summary.phosphorus > 0.0 {
        summary.ca_p_ratio = summary.calcium / summary.phosphorus;
    }

    summary
}

fn percentage_of_feed(total: f64, total_feed_kg: f64) -> Option<f64> {
    if total_feed_kg <= 0.0 {
        None
    } else {
        Some(total / (total_feed_kg * 10.0))
    }
}

fn per_kg_feed(value: f64, total_feed_kg: f64) -> Option<f64> {
    if total_feed_kg <= 0.0 {
        None
    } else {
        Some(value / total_feed_kg)
    }
}

fn uses_absolute_monogastric_basis(norms: &AnimalNorm) -> bool {
    matches!(
        norms.id.as_str(),
        "swine_sow_lactating" | "swine_sow_gestating"
    )
}

fn direct_summary_value(summary: &NutrientSummary, key: &str) -> Option<f64> {
    match key {
        "crude_fat" => Some(summary.crude_fat),
        "crude_fiber" => Some(summary.crude_fiber),
        "starch" => Some(summary.starch),
        "sugar" | "sugars" => Some(summary.sugar),
        "magnesium" => Some(summary.magnesium),
        "potassium" => Some(summary.potassium),
        "sodium" => Some(summary.sodium),
        "sulfur" => Some(summary.sulfur),
        "iron" => Some(summary.iron),
        "copper" => Some(summary.copper),
        "zinc" => Some(summary.zinc),
        "manganese" => Some(summary.manganese),
        "cobalt" => Some(summary.cobalt),
        "iodine" => Some(summary.iodine),
        "carotene" => Some(summary.carotene),
        "vit_d3" | "vitamin_d" => Some(summary.vit_d3),
        "vit_e" | "vitamin_e" => Some(summary.vit_e),
        "dig_protein_pig" => Some(summary.dig_protein_pig),
        "dig_protein_poultry" => Some(summary.dig_protein_poultry),
        _ => None,
    }
}

fn generic_direct_metric_value(
    summary: &NutrientSummary,
    norms: &AnimalNorm,
    key: &str,
) -> Option<f64> {
    let value = direct_summary_value(summary, key)?;
    if norms.species == "poultry"
        || (norms.species == "swine" && !uses_absolute_monogastric_basis(norms))
    {
        per_kg_feed(value, summary.total_weight_kg)
    } else {
        Some(value)
    }
}

fn supported_ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if numerator > 0.0 && denominator > 0.0 {
        Some(numerator / denominator)
    } else {
        None
    }
}

/// Return a nutrient metric in the same unit/basis semantics used by the solver for a given norm.
pub fn metric_value_for_norm(
    summary: &NutrientSummary,
    norms: &AnimalNorm,
    key: &str,
) -> Option<f64> {
    match key {
        "dry_matter_intake" => Some(summary.total_dm_kg),
        "feed_intake" => Some(summary.total_weight_kg),
        "energy_eke" => Some(summary.energy_eke),
        "energy_oe_cattle" => Some(summary.energy_oe_cattle),
        "energy_oe_pig" => Some(summary.energy_oe_pig),
        "energy_oe_poultry" => per_kg_feed(summary.energy_oe_poultry, summary.total_weight_kg),
        "crude_protein" if norms.species != "cattle" => {
            per_kg_feed(summary.crude_protein, summary.total_weight_kg)
        }
        "crude_protein" => Some(summary.crude_protein),
        "crude_protein_pct" => percentage_of_feed(summary.crude_protein, summary.total_weight_kg),
        "crude_fiber_pct" => percentage_of_feed(summary.crude_fiber, summary.total_weight_kg),
        "dig_protein_cattle" => Some(summary.dig_protein_cattle),
        "dig_protein_cattle_pct_cp" => Some(summary.dig_protein_cattle_pct_cp),
        "dig_protein_pig" | "dig_protein_poultry" => {
            generic_direct_metric_value(summary, norms, key)
        }
        "lysine" => Some(summary.lysine),
        "lysine_sid" if norms.species == "swine" && !uses_absolute_monogastric_basis(norms) => {
            per_kg_feed(summary.lysine, summary.total_weight_kg)
        }
        "lysine_sid" => Some(summary.lysine),
        "lysine_sid_pct" | "lysine_tid_pct" => {
            percentage_of_feed(summary.lysine, summary.total_weight_kg)
        }
        "methionine_cystine" => Some(summary.methionine_cystine),
        "methionine_cystine_sid"
            if norms.species == "swine" && !uses_absolute_monogastric_basis(norms) =>
        {
            per_kg_feed(summary.methionine_cystine, summary.total_weight_kg)
        }
        "methionine_cystine_sid" => Some(summary.methionine_cystine),
        "methionine_cystine_tid_pct" => {
            percentage_of_feed(summary.methionine_cystine, summary.total_weight_kg)
        }
        "methionine_cystine_lys_ratio" => {
            supported_ratio(summary.methionine_cystine, summary.lysine)
        }
        "starch_pct_dm" => Some(summary.starch_pct_dm),
        "calcium" if norms.species == "cattle" || uses_absolute_monogastric_basis(norms) => {
            Some(summary.calcium)
        }
        "calcium" => per_kg_feed(summary.calcium, summary.total_weight_kg),
        "calcium_pct" => percentage_of_feed(summary.calcium, summary.total_weight_kg),
        "phosphorus" if norms.species == "cattle" || uses_absolute_monogastric_basis(norms) => {
            Some(summary.phosphorus)
        }
        "phosphorus" => per_kg_feed(summary.phosphorus, summary.total_weight_kg),
        "ca_p_ratio" => supported_ratio(summary.calcium, summary.phosphorus),
        "dm_pct" => Some(summary.dm_pct),
        "cp_pct_dm" => Some(summary.cp_pct_dm),
        _ => generic_direct_metric_value(summary, norms, key),
    }
}
