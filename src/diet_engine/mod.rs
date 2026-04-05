//! Diet Engine - Nutrient calculations and LP optimization

pub mod auto_populate;
pub mod benchmarking;
pub mod category_populate;
pub mod economics;
pub mod feed_groups;
pub mod nutrient_calc;
pub mod optimizer;
pub mod screening;
pub mod validator;

use crate::db::{
    feeds::Feed,
    rations::{RationFull, RationItem},
};
use serde::{Deserialize, Serialize};

pub use auto_populate::{AutoPopulateItem, AutoPopulatePlan};
pub use category_populate::{
    CategoryPopulatePlan, CategoryRequirement, CategorySuggestion, FeedCategory, FeedOption,
    FeedWithAmount, RationStructure,
};
pub use economics::EconomicAnalysis;
pub use nutrient_calc::NutrientSummary;
pub use optimizer::{DietSolution, SolutionStatus};
pub use screening::{FeedRecommendation, ScreeningReport};

/// Optimization mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OptimizationMode {
    MinimizeCost,
    BalanceNutrients,
    SinglePassBalance,
    TieredBalance,
    FixedRation,
    RepairWithAdditions,
}

/// High-level solve intent selected by the user or inferred from the ration state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SolveIntent {
    SelectedOnly,
    CompleteFromLibrary,
    BuildFromLibrary,
}

/// Coarse ration state used to select the default workflow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RationState {
    Empty,
    Sparse,
    Structured,
}

/// Calculate nutrients for a ration
pub fn calculate_nutrients(ration: &RationFull) -> NutrientSummary {
    nutrient_calc::calculate_nutrients(&ration.items)
}

/// Calculate economics for a ration
pub fn calculate_economics(ration: &RationFull) -> EconomicAnalysis {
    economics::calculate_economics(&ration.items, ration.ration.animal_count)
}

/// Optimize a ration
pub fn optimize_ration(
    ration: &RationFull,
    mode: OptimizationMode,
    norms_override: Option<&crate::norms::AnimalNorm>,
) -> anyhow::Result<DietSolution> {
    optimizer::optimize(ration, mode, norms_override)
}

/// Optimize a ration with access to the current feed library.
pub fn optimize_ration_with_library(
    ration: &RationFull,
    mode: OptimizationMode,
    norms_override: Option<&crate::norms::AnimalNorm>,
    available_feeds: Option<&[Feed]>,
) -> anyhow::Result<DietSolution> {
    optimizer::optimize_with_library(ration, mode, norms_override, available_feeds)
}

/// Construct a ration directly from the current feed library.
pub fn construct_ration_from_library(
    ration: &RationFull,
    norms: &crate::norms::AnimalNorm,
    available_feeds: &[Feed],
) -> anyhow::Result<Option<DietSolution>> {
    optimizer::construct_ration_from_library(ration, norms, available_feeds)
}

/// Nutrition warning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NutritionWarning {
    EnergyDeficit {
        deficit_pct: f64,
    },
    ProteinDeficit {
        deficit_g: f64,
    },
    LimitingAminoAcid {
        aa_name: String,
        percent_of_norm: f64,
    },
    CaPhosImbalance {
        ratio: f64,
        normal: (f64, f64),
    },
    SeleniumToxicity {
        actual_mg: f64,
        max_mg: f64,
    },
    HighStarchRumen {
        starch_pct: f64,
    },
    HighMoisture {
        moisture_pct: f64,
    },
    DCadOutOfRange {
        dcad: f64,
    },
    PriceDataMissing {
        feed_names: Vec<String>,
    },
    FeedNotPalatable {
        feed_name: String,
    },
}

/// Validate a ration and return warnings
pub fn validate_ration(
    items: &[RationItem],
    norms: &crate::norms::AnimalNorm,
) -> Vec<NutritionWarning> {
    validator::validate(items, norms)
}
