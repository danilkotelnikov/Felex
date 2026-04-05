//! Animal feeding norms module

pub mod cattle_beef;
pub mod cattle_dairy;
pub mod cattle_minerals;
pub mod dynamic;
pub mod factorial;
pub mod poultry;
pub mod ration_matrix;
pub mod swine;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormMethodologyMetric {
    pub key: String,
    pub unit: String,
    pub reference_value: Option<f64>,
    pub current_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormMethodologyFactor {
    pub key: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormMethodology {
    pub key: String,
    pub reference_group_id: String,
    pub dynamic: bool,
    pub source_refs: Vec<String>,
    pub driver_metrics: Vec<NormMethodologyMetric>,
    pub derived_metrics: Vec<NormMethodologyMetric>,
    pub scaling_factors: Vec<NormMethodologyFactor>,
}

/// Animal feeding norm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalNorm {
    pub id: String,
    pub species: String,
    pub production_type: Option<String>,
    pub breed_group: Option<String>,
    pub sex: Option<String>,
    pub age_from_days: Option<i32>,
    pub age_to_days: Option<i32>,
    pub weight_from_kg: Option<f64>,
    pub weight_to_kg: Option<f64>,
    pub milk_yield_kg: Option<f64>,
    pub milk_fat_pct: Option<f64>,
    pub milk_protein_pct: Option<f64>,
    pub daily_gain_g: Option<i32>,
    pub nutrients_min: HashMap<String, f64>,
    pub nutrients_max: HashMap<String, f64>,
    pub nutrients_target: HashMap<String, f64>,
    pub feed_intake_min: Option<f64>,
    pub feed_intake_max: Option<f64>,
    pub notes: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnimalContext {
    pub species: Option<String>,
    pub production_type: Option<String>,
    pub breed: Option<String>,
    pub sex: Option<String>,
    pub live_weight_kg: Option<f64>,
    pub age_from_days: Option<i32>,
    pub age_to_days: Option<i32>,
    pub milk_yield_kg: Option<f64>,
    pub milk_fat_pct: Option<f64>,
    pub daily_gain_g: Option<i32>,
    pub egg_production_per_year: Option<f64>,
    pub litter_size: Option<f64>,
    pub reproductive_stage: Option<String>,
}

impl Default for AnimalNorm {
    fn default() -> Self {
        Self {
            id: String::new(),
            species: String::new(),
            production_type: None,
            breed_group: None,
            sex: None,
            age_from_days: None,
            age_to_days: None,
            weight_from_kg: None,
            weight_to_kg: None,
            milk_yield_kg: None,
            milk_fat_pct: None,
            milk_protein_pct: None,
            daily_gain_g: None,
            nutrients_min: HashMap::new(),
            nutrients_max: HashMap::new(),
            nutrients_target: HashMap::new(),
            feed_intake_min: None,
            feed_intake_max: None,
            notes: None,
            source: None,
        }
    }
}

/// Required nutrient definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrientRequirement {
    pub key: String,
    pub name_ru: String,
    pub name_en: String,
    pub unit: String,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub target_value: Option<f64>,
}

/// Get norms for an animal group
pub fn get_norms_for_group(group_id: &str) -> Option<AnimalNorm> {
    // Parse group ID to determine species and type
    if group_id.starts_with("cattle_dairy") {
        cattle_dairy::get_norms(group_id)
    } else if group_id.starts_with("cattle_beef") {
        cattle_beef::get_norms(group_id)
    } else if group_id.starts_with("swine") {
        swine::get_norms(group_id)
    } else if group_id.starts_with("poultry") {
        poultry::get_norms(group_id)
    } else {
        None
    }
}

/// Get list of required nutrients for a species
pub fn required_nutrients_for_species(species: &str) -> Vec<NutrientRequirement> {
    match species {
        "cattle" => cattle_dairy::required_nutrients(),
        "swine" => swine::required_nutrients(),
        "poultry" => poultry::required_nutrients(),
        _ => vec![],
    }
}

pub fn derive_norms_for_context(
    group_id: &str,
    context: Option<&AnimalContext>,
) -> Option<AnimalNorm> {
    dynamic::derive_norms_for_context(group_id, context)
}

pub fn describe_norm_methodology(
    group_id: &str,
    context: Option<&AnimalContext>,
) -> Option<NormMethodology> {
    dynamic::describe_norm_methodology(group_id, context)
}
