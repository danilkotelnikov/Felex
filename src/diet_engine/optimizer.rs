//! LP diet optimizer.

use good_lp::*;
use serde::{Deserialize, Serialize};

#[path = "alternatives.rs"]
mod alternatives;

use crate::db::{
    feed_labels::display_feed_name,
    feeds::Feed,
    rations::{RationFull, RationItem},
};
use crate::norms::{factorial::ConstraintTier, ration_matrix::RationMatrix, AnimalNorm};

use super::{
    feed_groups, nutrient_calc, screening, validator, FeedRecommendation, NutritionWarning,
    OptimizationMode, RationState, SolveIntent,
};

pub use alternatives::{
    cost_optimize_preserving_groups, AlternativeRationSolution, OptimizationComparison,
    OptimizationResult,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SolutionStatus {
    Optimal,
    Feasible,
    Infeasible,
    Unbounded,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelaxedConstraintType {
    Min,
    Max,
    Target,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaxedTarget {
    pub key: String,
    pub constraint_type: RelaxedConstraintType,
    pub target: f64,
    pub actual: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DietSolution {
    pub items: Vec<OptimizedItem>,
    pub nutrient_summary: nutrient_calc::NutrientSummary,
    pub cost_per_day: f64,
    pub optimization_status: SolutionStatus,
    pub warnings: Vec<NutritionWarning>,
    pub recommendations: Vec<FeedRecommendation>,
    pub applied_strategy: String,
    pub auto_populated: bool,
    pub solve_intent: Option<SolveIntent>,
    pub ration_state: Option<RationState>,
    pub workflow_notes: Vec<String>,
    pub best_achievable: bool,
    pub relaxed_targets: Vec<RelaxedTarget>,
    pub auto_added_feeds: Vec<AutoAddedFeed>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<alternatives::AlternativeRationSolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedItem {
    pub feed_id: i64,
    pub feed_name: String,
    pub amount_kg: f64,
    pub dm_kg: f64,
    pub cost_per_day: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoAddedFeed {
    pub feed_id: i64,
    pub feed_name: String,
    pub amount_kg: f64,
    pub reasons: Vec<String>,
}

#[derive(Clone)]
enum MetricExpr {
    Absolute(Expression),
    PerKgFeed(Expression),
    PercentOfFeed(Expression),
    PercentOfDm(Expression),
    Ratio(Expression, Expression),
}

impl MetricExpr {
    fn lhs(&self) -> Expression {
        match self {
            Self::Absolute(expr)
            | Self::PerKgFeed(expr)
            | Self::PercentOfFeed(expr)
            | Self::PercentOfDm(expr) => expr.clone(),
            Self::Ratio(numerator, _) => numerator.clone(),
        }
    }

    fn rhs(
        &self,
        value: f64,
        total_feed_expr: &Expression,
        total_dm_expr: &Expression,
    ) -> Expression {
        match self {
            Self::Absolute(_) => value.into(),
            Self::PerKgFeed(_) => total_feed_expr.clone() * value,
            Self::PercentOfFeed(_) => total_feed_expr.clone() * (value * 10.0),
            Self::PercentOfDm(_) => total_dm_expr.clone() * (value * 10.0),
            Self::Ratio(_, denominator) => denominator.clone() * value,
        }
    }
}

#[derive(Clone)]
struct TargetDeviation {
    metric: MetricExpr,
    target: f64,
    positive: Variable,
    negative: Variable,
    weight: f64,
}

#[derive(Clone)]
struct MetricBand {
    key: &'static str,
    value: f64,
    tolerance: f64,
}

#[derive(Clone, Copy)]
struct GroupShareBand {
    group: feed_groups::FeedGroup,
    share: f64,
    tolerance: f64,
}

#[derive(Clone, Copy)]
struct GroupShareDeviation {
    group: feed_groups::FeedGroup,
    share: f64,
    positive: Variable,
    negative: Variable,
    weight: f64,
}

#[derive(Clone, Copy)]
struct MovementPolicy {
    change_fraction: f64,
    change_floor_kg: f64,
}

#[derive(Clone, Copy)]
enum BalanceApproach {
    SinglePass,
    Tiered,
}

pub fn optimize(
    ration: &RationFull,
    mode: OptimizationMode,
    norms_override: Option<&AnimalNorm>,
) -> anyhow::Result<DietSolution> {
    optimize_with_library(ration, mode, norms_override, None)
}

pub fn optimize_with_library(
    ration: &RationFull,
    mode: OptimizationMode,
    norms_override: Option<&AnimalNorm>,
    available_feeds: Option<&[Feed]>,
) -> anyhow::Result<DietSolution> {
    let items = &ration.items;
    let default_norms = ration
        .ration
        .animal_group_id
        .as_deref()
        .and_then(crate::norms::get_norms_for_group);
    let norms = norms_override.or(default_norms.as_ref());

    if items.is_empty() {
        return Ok(calculate_current_solution(items, norms));
    }

    match mode {
        OptimizationMode::MinimizeCost => optimize_minimize_cost(items, norms),
        // Hybrid tiered optimization: Energy → Protein → Minerals → Cost
        // Per OPTIMIZER_REDESIGN_PLAN: ±50% of current amount or ±5 kg, whichever is larger
        OptimizationMode::BalanceNutrients | OptimizationMode::TieredBalance => {
            optimize_balance_nutrients(
                items,
                norms,
                MovementPolicy {
                    change_fraction: 0.50, // Allow up to ±50% change from original
                    change_floor_kg: 5.0,  // At least ±5kg movement for small feeds
                },
                BalanceApproach::Tiered,
                "priority_tiered_balance",
            )
        }
        // Single-pass for quick results with more freedom
        OptimizationMode::SinglePassBalance => optimize_balance_nutrients(
            items,
            norms,
            MovementPolicy {
                change_fraction: 0.75, // Allow up to ±75% change
                change_floor_kg: 8.0,  // More floor for single pass
            },
            BalanceApproach::SinglePass,
            "single_pass_balance",
        ),
        // Bounded mode for minimal changes to existing ration
        OptimizationMode::FixedRation => optimize_balance_nutrients(
            items,
            norms,
            MovementPolicy {
                change_fraction: 0.35, // Max 35% change
                change_floor_kg: 3.0,  // At least ±3kg for small feeds
            },
            BalanceApproach::Tiered,
            "bounded_fixed_feed_balance",
        ),
        OptimizationMode::RepairWithAdditions => {
            optimize_repair_with_additions(ration, norms, available_feeds.unwrap_or(&[]))
        }
    }
}

pub fn optimize_with_alternatives(
    ration: &RationFull,
    mode: OptimizationMode,
    norms_override: Option<&AnimalNorm>,
    available_feeds: Option<&[Feed]>,
    max_solutions: Option<usize>,
) -> anyhow::Result<OptimizationResult> {
    alternatives::optimize_with_alternatives(
        ration,
        mode,
        norms_override,
        available_feeds,
        max_solutions,
    )
}

fn calculate_current_solution(items: &[RationItem], norms: Option<&AnimalNorm>) -> DietSolution {
    let amounts: Vec<f64> = items.iter().map(|item| item.amount_kg).collect();
    build_solution(items, &amounts, norms, SolutionStatus::Feasible)
}

fn build_solution(
    items: &[RationItem],
    amounts: &[f64],
    norms: Option<&AnimalNorm>,
    status: SolutionStatus,
) -> DietSolution {
    let optimized_items: Vec<OptimizedItem> = items
        .iter()
        .zip(amounts.iter())
        .filter(|(_, amount)| **amount > 0.001)
        .filter_map(|(item, amount)| {
            let feed = item.feed.as_ref()?;
            Some(OptimizedItem {
                feed_id: item.feed_id,
                feed_name: display_feed_name(feed),
                amount_kg: *amount,
                dm_kg: *amount * feed.dry_matter.unwrap_or(86.0) / 100.0,
                cost_per_day: *amount * feed.price_per_kg(),
            })
        })
        .collect();

    let cost_per_day: f64 = optimized_items.iter().map(|item| item.cost_per_day).sum();
    let updated_items: Vec<RationItem> = items
        .iter()
        .zip(amounts.iter())
        .map(|(item, amount)| RationItem {
            amount_kg: *amount,
            ..item.clone()
        })
        .collect();
    let nutrient_summary = nutrient_calc::calculate_nutrients(&updated_items);
    let warnings = norms
        .map(|norm| validator::validate(&updated_items, norm))
        .unwrap_or_default();

    DietSolution {
        items: optimized_items,
        nutrient_summary,
        cost_per_day,
        optimization_status: status,
        warnings,
        recommendations: Vec::new(),
        applied_strategy: "current_state".to_string(),
        auto_populated: false,
        solve_intent: None,
        ration_state: None,
        workflow_notes: Vec::new(),
        best_achievable: false,
        relaxed_targets: Vec::new(),
        auto_added_feeds: Vec::new(),
        alternatives: Vec::new(),
    }
}

fn total_feed_expr(feed_vars: &[Variable]) -> Expression {
    feed_vars.iter().copied().sum()
}

fn total_dm_expr(feed_vars: &[Variable], items: &[RationItem]) -> Expression {
    feed_vars
        .iter()
        .zip(items.iter())
        .map(|(variable, item)| {
            let dm_share = item
                .feed
                .as_ref()
                .and_then(|feed| feed.dry_matter)
                .unwrap_or(86.0)
                / 100.0;
            *variable * dm_share
        })
        .sum()
}

fn feed_value_expr<F>(feed_vars: &[Variable], items: &[RationItem], mut value_fn: F) -> Expression
where
    F: FnMut(&crate::db::feeds::Feed) -> f64,
{
    feed_vars
        .iter()
        .zip(items.iter())
        .map(|(variable, item)| {
            let value = item.feed.as_ref().map(&mut value_fn).unwrap_or(0.0);
            *variable * value
        })
        .sum()
}

fn cost_expr(feed_vars: &[Variable], items: &[RationItem]) -> Expression {
    feed_vars
        .iter()
        .zip(items.iter())
        .map(|(variable, item)| {
            let price = item
                .feed
                .as_ref()
                .map(|feed| feed.price_per_kg())
                .unwrap_or(0.0);
            *variable * price
        })
        .sum()
}

fn generic_direct_metric_expr(
    norms: &AnimalNorm,
    key: &str,
    feed_vars: &[Variable],
    items: &[RationItem],
) -> Option<MetricExpr> {
    if key.ends_with("_pct")
        || key.ends_with("_ratio")
        || matches!(key, "dry_matter_intake" | "feed_intake")
    {
        return None;
    }

    let has_signal = items
        .iter()
        .filter_map(|item| item.feed.as_ref())
        .any(|feed| feed.nutrient_value(key) > 0.0);
    if !has_signal {
        return None;
    }

    let expr = feed_value_expr(feed_vars, items, |feed| feed.nutrient_value(key));
    if norms.species == "poultry"
        || (norms.species == "swine"
            && !matches!(
                norms.id.as_str(),
                "swine_sow_lactating" | "swine_sow_gestating"
            ))
    {
        Some(MetricExpr::PerKgFeed(expr))
    } else {
        Some(MetricExpr::Absolute(expr))
    }
}

fn resolve_metric_expr(
    norms: &AnimalNorm,
    key: &str,
    feed_vars: &[Variable],
    items: &[RationItem],
) -> Option<MetricExpr> {
    let total_feed = || total_feed_expr(feed_vars);
    let total_dm = || total_dm_expr(feed_vars, items);
    let total_energy_eke = || {
        feed_vars
            .iter()
            .zip(items.iter())
            .map(|(variable, item)| {
                let feed = item.feed.as_ref();
                let dm_share = feed.and_then(|f| f.dry_matter).unwrap_or(86.0) / 100.0;
                let oe = feed.and_then(|f| f.energy_oe_cattle).unwrap_or(0.0);
                *variable * dm_share * oe / 10.5
            })
            .sum()
    };

    let total_energy_cattle = || {
        feed_vars
            .iter()
            .zip(items.iter())
            .map(|(variable, item)| {
                let feed = item.feed.as_ref();
                let dm_share = feed.and_then(|f| f.dry_matter).unwrap_or(86.0) / 100.0;
                let oe = feed.and_then(|f| f.energy_oe_cattle).unwrap_or(0.0);
                *variable * dm_share * oe
            })
            .sum()
    };

    let total_energy_pig = || {
        feed_vars
            .iter()
            .zip(items.iter())
            .map(|(variable, item)| {
                let feed = item.feed.as_ref();
                let dm_share = feed.and_then(|f| f.dry_matter).unwrap_or(86.0) / 100.0;
                let oe = feed.and_then(|f| f.energy_oe_pig).unwrap_or(0.0);
                *variable * dm_share * oe
            })
            .sum()
    };

    let total_energy_poultry = || {
        feed_vars
            .iter()
            .zip(items.iter())
            .map(|(variable, item)| {
                let feed = item.feed.as_ref();
                let dm_share = feed.and_then(|f| f.dry_matter).unwrap_or(86.0) / 100.0;
                let oe = feed.and_then(|f| f.energy_oe_poultry).unwrap_or(0.0);
                *variable * dm_share * oe
            })
            .sum()
    };

    let total_cp = || feed_value_expr(feed_vars, items, |feed| feed.crude_protein.unwrap_or(0.0));
    let total_fiber = || feed_value_expr(feed_vars, items, |feed| feed.crude_fiber.unwrap_or(0.0));
    let total_lysine = || feed_value_expr(feed_vars, items, |feed| feed.lysine.unwrap_or(0.0));
    let total_metcys =
        || feed_value_expr(feed_vars, items, |feed| feed.methionine_cystine.unwrap_or(0.0));
    let total_calcium = || feed_value_expr(feed_vars, items, |feed| feed.calcium.unwrap_or(0.0));
    let total_phosphorus =
        || feed_value_expr(feed_vars, items, |feed| feed.phosphorus.unwrap_or(0.0));
    let total_starch = || feed_value_expr(feed_vars, items, |feed| feed.starch.unwrap_or(0.0));
    let total_vit_d3 = || feed_value_expr(feed_vars, items, |feed| feed.vit_d3.unwrap_or(0.0));
    let total_vit_e = || feed_value_expr(feed_vars, items, |feed| feed.vit_e.unwrap_or(0.0));

    match key {
        "dry_matter_intake" => Some(MetricExpr::Absolute(total_dm())),
        "feed_intake" => Some(MetricExpr::Absolute(total_feed())),
        "energy_eke" => Some(MetricExpr::Absolute(total_energy_eke())),
        "energy_oe_cattle" => Some(MetricExpr::Absolute(total_energy_cattle())),
        "energy_oe_pig" => Some(MetricExpr::Absolute(total_energy_pig())),
        "energy_oe_poultry" => Some(MetricExpr::PerKgFeed(total_energy_poultry())),
        "crude_protein_pct" => Some(MetricExpr::PercentOfFeed(total_cp())),
        "crude_fiber_pct" => Some(MetricExpr::PercentOfFeed(total_fiber())),
        "crude_protein" if norms.species == "cattle" => Some(MetricExpr::Absolute(total_cp())),
        "crude_protein" if norms.id == "swine_finisher" => Some(MetricExpr::PerKgFeed(total_cp())),
        "crude_protein" => Some(MetricExpr::Absolute(total_cp())),
        "dig_protein_cattle" => Some(MetricExpr::Absolute(feed_value_expr(
            feed_vars,
            items,
            |feed| feed.dig_protein_cattle.unwrap_or(0.0),
        ))),
        "dig_protein_cattle_pct_cp" => Some(MetricExpr::Ratio(
            feed_value_expr(feed_vars, items, |feed| {
                feed.dig_protein_cattle.unwrap_or(0.0)
            }),
            total_cp(),
        )),
        "lysine" => Some(MetricExpr::Absolute(total_lysine())),
        "lysine_sid" if norms.id == "swine_finisher" => Some(MetricExpr::PerKgFeed(total_lysine())),
        "lysine_sid" => Some(MetricExpr::Absolute(total_lysine())),
        "lysine_sid_pct" | "lysine_tid_pct" => Some(MetricExpr::PercentOfFeed(total_lysine())),
        "methionine_cystine" => Some(MetricExpr::Absolute(total_metcys())),
        "methionine_cystine_sid" if norms.id == "swine_finisher" => {
            Some(MetricExpr::PerKgFeed(total_metcys()))
        }
        "methionine_cystine_sid" => Some(MetricExpr::Absolute(total_metcys())),
        "methionine_cystine_tid_pct" => Some(MetricExpr::PercentOfFeed(total_metcys())),
        "methionine_cystine_lys_ratio" => Some(MetricExpr::Ratio(total_metcys(), total_lysine())),
        "crude_fiber" => Some(MetricExpr::Absolute(total_fiber())),
        "starch_pct_dm" => Some(MetricExpr::PercentOfDm(total_starch())),
        "calcium" if norms.species == "cattle" || norms.id == "swine_sow_lactating" => {
            Some(MetricExpr::Absolute(total_calcium()))
        }
        "calcium" => Some(MetricExpr::PerKgFeed(total_calcium())),
        "calcium_pct" => Some(MetricExpr::PercentOfFeed(total_calcium())),
        "ca_p_ratio" => Some(MetricExpr::Ratio(total_calcium(), total_phosphorus())),
        "phosphorus" => Some(MetricExpr::Absolute(total_phosphorus())),
        "phosphorus_pct" => Some(MetricExpr::PercentOfFeed(total_phosphorus())),
        "vit_d3" => Some(MetricExpr::Absolute(total_vit_d3())),
        "vit_e" => Some(MetricExpr::Absolute(total_vit_e())),
        _ => generic_direct_metric_expr(norms, key, feed_vars, items),
    }
}

fn apply_hard_constraints<M: SolverModel>(
    mut problem: M,
    norms: &AnimalNorm,
    feed_vars: &[Variable],
    items: &[RationItem],
) -> M {
    let total_feed = total_feed_expr(feed_vars);
    let total_dm = total_dm_expr(feed_vars, items);
    let intake_expr = if norms.species == "cattle" {
        total_dm.clone()
    } else {
        total_feed.clone()
    };

    if let Some(min_intake) = norms.feed_intake_min {
        problem = problem.with(constraint!(intake_expr.clone() >= min_intake));
    }
    if let Some(max_intake) = norms.feed_intake_max {
        problem = problem.with(constraint!(intake_expr <= max_intake));
    }

    for (key, min_value) in &norms.nutrients_min {
        if !metric_has_support_in_items(items, key) {
            continue;
        }
        if let Some(metric) = resolve_metric_expr(norms, key, feed_vars, items) {
            problem = problem.with(constraint!(
                metric.lhs() >= metric.rhs(*min_value, &total_feed, &total_dm)
            ));
        }
    }

    for (key, target_value) in &norms.nutrients_target {
        if norms.nutrients_min.contains_key(key) || !target_acts_as_minimum(key) {
            continue;
        }
        if !metric_has_support_in_items(items, key) {
            continue;
        }
        if let Some(metric) = resolve_metric_expr(norms, key, feed_vars, items) {
            problem = problem.with(constraint!(
                metric.lhs() >= metric.rhs(*target_value, &total_feed, &total_dm)
            ));
        }
    }

    for (key, max_value) in &norms.nutrients_max {
        if !metric_has_support_in_items(items, key) {
            continue;
        }
        if let Some(metric) = resolve_metric_expr(norms, key, feed_vars, items) {
            problem = problem.with(constraint!(
                metric.lhs() <= metric.rhs(*max_value, &total_feed, &total_dm)
            ));
        }
    }

    apply_practical_constraints(problem, norms, feed_vars, items)
}

fn apply_practical_constraints<M: SolverModel>(
    problem: M,
    norms: &AnimalNorm,
    feed_vars: &[Variable],
    items: &[RationItem],
) -> M {
    apply_practical_constraints_with_options(problem, norms, feed_vars, items, true)
}

fn apply_practical_constraints_with_options<M: SolverModel>(
    mut problem: M,
    norms: &AnimalNorm,
    feed_vars: &[Variable],
    items: &[RationItem],
    enforce_ration_matrix: bool,
) -> M {
    let total_feed = total_feed_expr(feed_vars);
    let total_dm = total_dm_expr(feed_vars, items);
    let intake_expr = if norms.species == "cattle" {
        total_dm.clone()
    } else {
        total_feed.clone()
    };
    if let Some(floor) = preserved_intake_floor(norms, items) {
        problem = problem.with(constraint!(intake_expr >= floor));
    }
    let problem =
        apply_feed_inclusion_constraints(problem, norms, feed_vars, items, &total_feed, &total_dm);
    if enforce_ration_matrix {
        apply_category_constraints(problem, norms, feed_vars, items)
    } else {
        problem
    }
}

fn preserved_intake_floor(norms: &AnimalNorm, items: &[RationItem]) -> Option<f64> {
    if items.is_empty() {
        return None;
    }

    let current = if norms.species == "cattle" {
        items.iter().map(RationItem::dm_kg).sum::<f64>()
    } else {
        items.iter().map(|item| item.amount_kg).sum::<f64>()
    };
    if current <= 0.0 {
        return None;
    }

    let mut floor = current * 0.80;
    if let Some(maximum) = norms.feed_intake_max {
        floor = floor.min(maximum);
    }

    Some((floor * 1000.0).round() / 1000.0)
}

fn target_acts_as_minimum(key: &str) -> bool {
    matches!(
        key,
        "crude_protein"
            | "crude_protein_pct"
            | "dig_protein_cattle"
            | "lysine"
            | "lysine_sid"
            | "lysine_sid_pct"
            | "lysine_tid_pct"
            | "crude_fiber"
            | "crude_fiber_pct"
            | "methionine_cystine"
            | "methionine_cystine_sid"
            | "methionine_cystine_tid_pct"
            | "calcium"
            | "calcium_pct"
            | "phosphorus"
            | "phosphorus_pct"
            | "vit_d3"
            | "vit_e"
    ) || key.starts_with("vit_")
        || matches!(
            key,
            "carotene"
                | "magnesium"
                | "potassium"
                | "sodium"
                | "sulfur"
                | "iron"
                | "copper"
                | "zinc"
                | "manganese"
                | "cobalt"
                | "iodine"
        )
}

fn all_objective_keys(norms: &AnimalNorm) -> Vec<&'static str> {
    let mut keys = Vec::new();
    for tier in tier_keys(norms) {
        for key in tier {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    keys
}

fn key_present_in_norms(norms: &AnimalNorm, key: &str) -> bool {
    norms.nutrients_min.contains_key(key)
        || norms.nutrients_max.contains_key(key)
        || norms.nutrients_target.contains_key(key)
}

pub(super) fn feed_has_metric_signal(feed: &Feed, key: &str) -> bool {
    match key {
        "energy_oe_poultry"
        | "energy_oe_pig"
        | "energy_oe_cattle"
        | "crude_protein"
        | "crude_fiber"
        | "dig_protein_cattle"
        | "dig_protein_pig"
        | "dig_protein_poultry"
        | "lysine"
        | "starch"
        | "sugar"
        | "calcium"
        | "phosphorus"
        | "magnesium"
        | "potassium"
        | "sodium"
        | "sulfur"
        | "iron"
        | "copper"
        | "zinc"
        | "manganese"
        | "cobalt"
        | "iodine"
        | "carotene"
        | "vit_d3"
        | "vit_e" => feed.nutrient_value(key) > 0.0,
        "energy_eke" => feed.energy_oe_cattle.unwrap_or(0.0) > 0.0,
        "crude_protein_pct" => feed.crude_protein.unwrap_or(0.0) > 0.0,
        "crude_fiber_pct" => feed.crude_fiber.unwrap_or(0.0) > 0.0,
        "dig_protein_cattle_pct_cp" => {
            feed.dig_protein_cattle.unwrap_or(0.0) > 0.0 && feed.crude_protein.unwrap_or(0.0) > 0.0
        }
        "lysine_sid" | "lysine_sid_pct" | "lysine_tid_pct" => feed.lysine.unwrap_or(0.0) > 0.0,
        "methionine_cystine" | "methionine_cystine_sid" | "methionine_cystine_tid_pct" => {
            feed.methionine_cystine.unwrap_or(0.0) > 0.0
        }
        "phosphorus_pct" => feed.phosphorus.unwrap_or(0.0) > 0.0,
        "starch_pct_dm" => feed.starch.unwrap_or(0.0) > 0.0 && feed.dry_matter.unwrap_or(0.0) > 0.0,
        "calcium_pct" => feed.calcium.unwrap_or(0.0) > 0.0,
        _ => feed.nutrient_value(key) > 0.0,
    }
}

fn metric_has_support_in_items(items: &[RationItem], key: &str) -> bool {
    let has_signal = |metric_key| {
        items
            .iter()
            .filter_map(|item| item.feed.as_ref())
            .any(|feed| feed_has_metric_signal(feed, metric_key))
    };

    match key {
        "dry_matter_intake" | "feed_intake" => true,
        "ca_p_ratio" => has_signal("calcium") && has_signal("phosphorus"),
        "methionine_cystine_lys_ratio" => has_signal("methionine_cystine") && has_signal("lysine"),
        _ => has_signal(key),
    }
}

fn supported_objective_keys(
    items: &[RationItem],
    objective_keys: &[&'static str],
) -> Vec<&'static str> {
    objective_keys
        .iter()
        .copied()
        .filter(|key| metric_has_support_in_items(items, key))
        .collect()
}

const ALL_OPTIMIZER_KEYS: &[&str] = &[
    "dry_matter_intake",
    "feed_intake",
    "energy_eke",
    "energy_oe_cattle",
    "energy_oe_pig",
    "energy_oe_poultry",
    "crude_protein",
    "crude_protein_pct",
    "dig_protein_cattle",
    "dig_protein_pig",
    "dig_protein_poultry",
    "dig_protein_cattle_pct_cp",
    "lysine",
    "lysine_sid",
    "lysine_sid_pct",
    "lysine_tid_pct",
    "methionine_cystine",
    "methionine_cystine_sid",
    "methionine_cystine_tid_pct",
    "crude_fiber",
    "starch",
    "starch_pct_dm",
    "sugar",
    "crude_fat",
    "calcium",
    "calcium_pct",
    "phosphorus",
    "phosphorus_pct",
    "magnesium",
    "potassium",
    "sodium",
    "sulfur",
    "iron",
    "copper",
    "zinc",
    "manganese",
    "cobalt",
    "iodine",
    "carotene",
    "vit_d3",
    "vit_e",
    "ca_p_ratio",
    "methionine_cystine_lys_ratio",
];

pub(super) fn constraint_tier_for_key(norms: &AnimalNorm, key: &str) -> ConstraintTier {
    let normalized = key;

    match norms.species.as_str() {
        "cattle" => {
            if matches!(
                normalized,
                "dry_matter_intake"
                    | "feed_intake"
                    | "energy_eke"
                    | "energy_oe_cattle"
                    | "crude_protein"
                    | "dig_protein_cattle"
                    | "crude_fiber"
            ) {
                ConstraintTier::Tier1
            } else if matches!(
                normalized,
                |"crude_protein_pct"| "dig_protein_cattle_pct_cp"
                    | "lysine"
                    | "methionine_cystine"
                    | "starch"
                    | "starch_pct_dm"
                    | "calcium"
                    | "calcium_pct"
                    | "phosphorus"
                    | "phosphorus_pct"
                    | "magnesium"
                    | "potassium"
                    | "sodium"
                    | "ca_p_ratio"
            ) {
                ConstraintTier::Tier2
            } else {
                ConstraintTier::Tier3
            }
        }
        "swine" => {
            if matches!(
                normalized,
                "dry_matter_intake"
                    | "feed_intake"
                    | "energy_oe_pig"
                    | "crude_protein"
                    | "crude_protein_pct"
                    | "lysine_sid"
                    | "lysine_sid_pct"
                    | "crude_fiber"
            ) {
                ConstraintTier::Tier1
            } else if matches!(
                normalized,
                "dig_protein_pig"
                    | "lysine"
                    | "methionine_cystine"
                    | "methionine_cystine_sid"
                    | "methionine_cystine_tid_pct"
                    | "calcium"
                    | "calcium_pct"
                    | "phosphorus"
                    | "phosphorus_pct"
                    | "magnesium"
                    | "sodium"
                    | "potassium"
                    | "ca_p_ratio"
                    | "methionine_cystine_lys_ratio"
            ) {
                ConstraintTier::Tier2
            } else {
                ConstraintTier::Tier3
            }
        }
        "poultry" => {
            if matches!(
                normalized,
                "dry_matter_intake"
                    | "feed_intake"
                    | "energy_oe_poultry"
                    | "crude_protein_pct"
                    | "lysine_tid_pct"
                    | "methionine_cystine_tid_pct"
                    | "crude_fiber"
            ) {
                ConstraintTier::Tier1
            } else if matches!(
                normalized,
                "crude_protein"
                    | "dig_protein_poultry"
                    | "lysine"
                    | "methionine_cystine"
                    | "calcium"
                    | "calcium_pct"
                    | "phosphorus"
                    | "phosphorus_pct"
                    | "magnesium"
                    | "sodium"
                    | "potassium"
                    | "ca_p_ratio"
                    | "methionine_cystine_lys_ratio"
            ) {
                ConstraintTier::Tier2
            } else {
                ConstraintTier::Tier3
            }
        }
        _ => ConstraintTier::Tier3,
    }
}

/// Builds an expression summing DM for feeds matching a specific group
fn group_dm_expr(
    feed_vars: &[Variable],
    items: &[RationItem],
    group: feed_groups::FeedGroup,
) -> Expression {
    feed_vars
        .iter()
        .zip(items.iter())
        .filter_map(|(variable, item)| {
            let feed = item.feed.as_ref()?;
            if feed_groups::classify_feed(feed) == group {
                let dm_share = feed.dry_matter.unwrap_or(86.0) / 100.0;
                Some(*variable * dm_share)
            } else {
                None
            }
        })
        .sum()
}

fn feed_matches_matrix_category(feed: &Feed, category: &str) -> bool {
    use feed_groups::FeedGroup;

    match category {
        "roughage" => matches!(feed_groups::classify_feed(feed), FeedGroup::Roughage),
        "succulent" => matches!(feed_groups::classify_feed(feed), FeedGroup::Succulent),
        "concentrate" => matches!(
            feed_groups::classify_feed(feed),
            FeedGroup::Concentrate | FeedGroup::Protein | FeedGroup::AnimalOrigin
        ),
        "mineral" => matches!(feed_groups::classify_feed(feed), FeedGroup::Mineral),
        "premix" => matches!(
            feed_groups::classify_feed(feed),
            FeedGroup::Premix | FeedGroup::Vitamin
        ),
        "animal_origin" => matches!(feed_groups::classify_feed(feed), FeedGroup::AnimalOrigin),
        "npn" => feed_groups::is_nonprotein_nitrogen_feed(feed),
        _ => false,
    }
}

fn matrix_category_dm_expr(
    feed_vars: &[Variable],
    items: &[RationItem],
    category: &str,
) -> Expression {
    feed_vars
        .iter()
        .zip(items.iter())
        .filter_map(|(variable, item)| {
            let feed = item.feed.as_ref()?;
            if feed_matches_matrix_category(feed, category) {
                let dm_share = feed.dry_matter.unwrap_or(86.0) / 100.0;
                Some(*variable * dm_share)
            } else {
                None
            }
        })
        .sum()
}

fn total_dm_value(items: &[RationItem], amounts: &[f64]) -> f64 {
    items
        .iter()
        .zip(amounts.iter())
        .filter_map(|(item, amount)| {
            let feed = item.feed.as_ref()?;
            Some(*amount * feed.dry_matter.unwrap_or(86.0) / 100.0)
        })
        .sum()
}

fn group_share_for_amounts(
    items: &[RationItem],
    amounts: &[f64],
    group: feed_groups::FeedGroup,
) -> Option<f64> {
    let total_dm = total_dm_value(items, amounts);
    if total_dm <= 0.0 {
        return None;
    }

    let group_dm: f64 = items
        .iter()
        .zip(amounts.iter())
        .filter_map(|(item, amount)| {
            let feed = item.feed.as_ref()?;
            if feed_groups::classify_feed(feed) == group {
                Some(*amount * feed.dry_matter.unwrap_or(86.0) / 100.0)
            } else {
                None
            }
        })
        .sum();

    Some(group_dm / total_dm)
}

#[cfg(test)]
fn matrix_category_share_for_amounts(
    items: &[RationItem],
    amounts: &[f64],
    category: &str,
) -> Option<f64> {
    let total_dm = total_dm_value(items, amounts);
    if total_dm <= 0.0 {
        return None;
    }

    let category_dm: f64 = items
        .iter()
        .zip(amounts.iter())
        .filter_map(|(item, amount)| {
            let feed = item.feed.as_ref()?;
            if feed_matches_matrix_category(feed, category) {
                Some(*amount * feed.dry_matter.unwrap_or(86.0) / 100.0)
            } else {
                None
            }
        })
        .sum();

    Some(category_dm / total_dm)
}

fn has_group_feeds(items: &[RationItem], group: feed_groups::FeedGroup) -> bool {
    items.iter().any(|item| {
        item.feed
            .as_ref()
            .map(|f| feed_groups::classify_feed(f) == group)
            .unwrap_or(false)
    })
}

fn has_matrix_category_feeds(items: &[RationItem], category: &str) -> bool {
    items.iter().any(|item| {
        item.feed
            .as_ref()
            .map(|feed| feed_matches_matrix_category(feed, category))
            .unwrap_or(false)
    })
}

fn species_max_inclusion(feed: &crate::db::feeds::Feed, species: &str) -> Option<f64> {
    let raw = match species {
        "swine" => feed.max_inclusion_pig,
        "poultry" => feed.max_inclusion_poultry,
        _ => feed.max_inclusion_cattle,
    }?;

    if raw <= 0.0 {
        None
    } else {
        Some((raw / 100.0).clamp(0.0, 1.0))
    }
}

fn apply_feed_inclusion_constraints<M: SolverModel>(
    mut problem: M,
    norms: &AnimalNorm,
    feed_vars: &[Variable],
    items: &[RationItem],
    total_feed: &Expression,
    total_dm: &Expression,
) -> M {
    let use_dm_basis = norms.species == "cattle";
    let total_basis = if use_dm_basis {
        total_dm.clone()
    } else {
        total_feed.clone()
    };

    for (variable, item) in feed_vars.iter().zip(items.iter()) {
        let Some(feed) = item.feed.as_ref() else {
            continue;
        };
        let Some(max_share) = species_max_inclusion(feed, norms.species.as_str()) else {
            continue;
        };

        let feed_basis = if use_dm_basis {
            let dm_share = feed.dry_matter.unwrap_or(86.0) / 100.0;
            *variable * dm_share
        } else {
            *variable * 1.0
        };

        problem = problem.with(constraint!(feed_basis <= total_basis.clone() * max_share));
    }

    problem
}

fn apply_category_constraints<M: SolverModel>(
    mut problem: M,
    norms: &AnimalNorm,
    feed_vars: &[Variable],
    items: &[RationItem],
) -> M {
    let Some(matrix) = RationMatrix::for_norm(norms) else {
        return problem;
    };

    let total_dm = total_dm_expr(feed_vars, items);

    for constraint in &matrix.constraints {
        if !has_matrix_category_feeds(items, constraint.feed_type.as_str()) {
            continue;
        }

        let lower = (constraint.min_pct / 100.0).clamp(0.0, 1.0);
        let upper = (constraint.max_pct / 100.0).clamp(0.0, 1.0);
        let category_dm = matrix_category_dm_expr(feed_vars, items, constraint.feed_type.as_str());

        if lower > 0.0 {
            problem = problem.with(constraint!(category_dm.clone() >= total_dm.clone() * lower));
        }
        if upper < 1.0 {
            problem = problem.with(constraint!(category_dm <= total_dm.clone() * upper));
        }
    }

    problem
}

fn should_optimize_group_shares(norms: &AnimalNorm, objective_keys: &[&'static str]) -> bool {
    match norms.species.as_str() {
        "swine" => objective_keys.iter().any(|key| {
            matches!(
                *key,
                "methionine_cystine_sid" | "calcium" | "phosphorus"
            )
        }),
        "poultry" => objective_keys
            .iter()
            .any(|key| matches!(*key, "calcium_pct" | "phosphorus")),
        _ => objective_keys
            .iter()
            .any(|key| matches!(*key, "crude_fiber" | "calcium" | "phosphorus")),
    }
}

fn should_optimize_ca_p_ratio(norms: &AnimalNorm, objective_keys: &[&'static str]) -> bool {
    match norms.species.as_str() {
        "swine" => objective_keys
            .iter()
            .any(|key| matches!(*key, "calcium" | "phosphorus")),
        "poultry" => objective_keys
            .iter()
            .any(|key| matches!(*key, "calcium_pct" | "phosphorus")),
        _ => objective_keys
            .iter()
            .any(|key| matches!(*key, "calcium" | "phosphorus")),
    }
}

fn default_ca_p_ratio_target(norms: &AnimalNorm) -> Option<f64> {
    match norms.species.as_str() {
        "cattle" => Some(1.55),
        "swine" => Some(if norms.id.contains("sow") { 1.35 } else { 1.25 }),
        "poultry" => Some(if norms.id.contains("layer") { 6.5 } else { 2.0 }),
        _ => None,
    }
}

fn target_ca_p_ratio(norms: &AnimalNorm, objective_keys: &[&'static str]) -> Option<f64> {
    if !should_optimize_ca_p_ratio(norms, objective_keys) {
        return None;
    }

    if let Some(target) = norms.nutrients_target.get("ca_p_ratio").copied() {
        return Some(target);
    }
    if let (Some(min), Some(max)) = (
        norms.nutrients_min.get("ca_p_ratio").copied(),
        norms.nutrients_max.get("ca_p_ratio").copied(),
    ) {
        return Some((min + max) / 2.0);
    }
    if let Some(min) = norms.nutrients_min.get("ca_p_ratio").copied() {
        return Some(min);
    }
    if let Some(max) = norms.nutrients_max.get("ca_p_ratio").copied() {
        return Some(max);
    }

    default_ca_p_ratio_target(norms)
}

fn ratio_target_value(norms: &AnimalNorm, key: &str) -> Option<f64> {
    if let Some(target) = norms.nutrients_target.get(key).copied() {
        return Some(target);
    }
    if let (Some(min), Some(max)) = (
        norms.nutrients_min.get(key).copied(),
        norms.nutrients_max.get(key).copied(),
    ) {
        return Some((min + max) / 2.0);
    }
    norms
        .nutrients_min
        .get(key)
        .copied()
        .or_else(|| norms.nutrients_max.get(key).copied())
}

fn should_optimize_methionine_cystine_lys_ratio(
    norms: &AnimalNorm,
    objective_keys: &[&'static str],
) -> bool {
    match norms.species.as_str() {
        "swine" => objective_keys
            .iter()
            .any(|key| matches!(*key, "methionine_cystine" | "methionine_cystine_sid")),
        "poultry" => objective_keys
            .iter()
            .any(|key| matches!(*key, "methionine_cystine_tid_pct")),
        _ => false,
    }
}

fn nutrient_target_value(norms: &AnimalNorm, key: &str) -> Option<f64> {
    norms
        .nutrients_target
        .get(key)
        .copied()
        .or_else(|| norms.nutrients_min.get(key).copied())
}

fn target_methionine_cystine_lys_ratio(
    norms: &AnimalNorm,
    objective_keys: &[&'static str],
) -> Option<f64> {
    if !should_optimize_methionine_cystine_lys_ratio(norms, objective_keys) {
        return None;
    }

    if let Some(target) = ratio_target_value(norms, "methionine_cystine_lys_ratio") {
        return Some(target);
    }

    let (numerator_key, denominator_key) = match norms.species.as_str() {
        "swine" if norms.id == "swine_finisher" => ("methionine_cystine_sid", "lysine_sid"),
        "poultry" => ("methionine_cystine_tid_pct", "lysine_tid_pct"),
        _ => return None,
    };

    let numerator = nutrient_target_value(norms, numerator_key)?;
    let denominator = nutrient_target_value(norms, denominator_key)?;
    if denominator <= 0.0 {
        None
    } else {
        Some(numerator / denominator)
    }
}

fn target_amino_ratio_goals(
    norms: &AnimalNorm,
    objective_keys: &[&'static str],
) -> Vec<(&'static str, f64)> {
    let mut goals = Vec::new();

    if let Some(target) = target_methionine_cystine_lys_ratio(norms, objective_keys) {
        goals.push(("methionine_cystine_lys_ratio", target));
    }

    goals
}

fn group_share_weight(species: &str, group: feed_groups::FeedGroup) -> f64 {
    use feed_groups::FeedGroup;

    // Weights must be competitive with nutrient priority weights (40-100 range)
    // to ensure group share soft targets actually influence the solution.
    match (species, group) {
        ("cattle", FeedGroup::Roughage | FeedGroup::Succulent | FeedGroup::Concentrate) => 55.0,
        ("cattle", FeedGroup::Protein) => 35.0,
        ("swine", FeedGroup::Concentrate | FeedGroup::Protein) => 45.0,
        ("poultry", FeedGroup::Concentrate | FeedGroup::Protein) => 45.0,
        (_, FeedGroup::Mineral | FeedGroup::Premix) => 20.0,
        (_, FeedGroup::AnimalOrigin) => 25.0,
        _ => 15.0,
    }
}

fn target_group_shares(
    norms: &AnimalNorm,
    objective_keys: &[&'static str],
    items: &[RationItem],
) -> Vec<(feed_groups::FeedGroup, f64, f64)> {
    if !should_optimize_group_shares(norms, objective_keys) {
        return Vec::new();
    }

    let mut shares = Vec::new();

    // Prefer RationMatrix opt_pct targets if available
    if let Some(matrix) = RationMatrix::for_norm(norms) {
        for constraint in matrix.constraints {
            let group = match constraint.feed_type.as_str() {
                "roughage" => feed_groups::FeedGroup::Roughage,
                "succulent" => feed_groups::FeedGroup::Succulent,
                "concentrate" => feed_groups::FeedGroup::Concentrate,
                "mineral" => feed_groups::FeedGroup::Mineral,
                "premix" => feed_groups::FeedGroup::Premix,
                "animal_origin" => feed_groups::FeedGroup::AnimalOrigin,
                _ => continue,
            };

            if has_group_feeds(items, group) {
                let weight = group_share_weight(norms.species.as_str(), group);
                shares.push((group, constraint.opt_pct / 100.0, weight));
            }
        }
        return shares;
    }

    // Fallback to legacy template_for_group
    feed_groups::template_for_group(Some(norms.id.as_str()), norms.species.as_str())
        .into_iter()
        .filter(|share| has_group_feeds(items, share.group))
        .map(|share| {
            (
                share.group,
                share.share,
                group_share_weight(norms.species.as_str(), share.group),
            )
        })
        .collect()
}

fn build_group_share_bands(
    norms: &AnimalNorm,
    items: &[RationItem],
    amounts: &[f64],
    objective_keys: &[&'static str],
    tolerance: f64,
) -> Vec<GroupShareBand> {
    target_group_shares(norms, objective_keys, items)
        .into_iter()
        .filter_map(|(group, _, _)| {
            group_share_for_amounts(items, amounts, group).map(|share| GroupShareBand {
                group,
                share,
                tolerance,
            })
        })
        .collect()
}

fn minimum_group_share_slack(group: feed_groups::FeedGroup) -> f64 {
    use feed_groups::FeedGroup;

    match group {
        FeedGroup::Mineral | FeedGroup::Premix => 0.005,
        FeedGroup::AnimalOrigin => 0.01,
        _ => 0.02,
    }
}

fn apply_group_share_bands<M: SolverModel>(
    mut problem: M,
    feed_vars: &[Variable],
    items: &[RationItem],
    bands: &[GroupShareBand],
) -> M {
    let total_dm = total_dm_expr(feed_vars, items);

    for band in bands {
        let group_dm = group_dm_expr(feed_vars, items, band.group);
        let delta = (band.share.abs() * band.tolerance).max(minimum_group_share_slack(band.group));
        let lower = (band.share - delta).max(0.0);
        let upper = (band.share + delta).min(1.0);
        problem = problem.with(constraint!(group_dm.clone() >= total_dm.clone() * lower));
        problem = problem.with(constraint!(group_dm <= total_dm.clone() * upper));
    }

    problem
}

fn optimize_minimize_cost(
    items: &[RationItem],
    norms: Option<&AnimalNorm>,
) -> anyhow::Result<DietSolution> {
    let Some(norms) = norms else {
        return Ok(calculate_current_solution(items, None));
    };

    let mut vars = variables!();
    let feed_vars: Vec<Variable> = items
        .iter()
        .map(|item| {
            let min = if item.is_locked { item.amount_kg } else { 0.0 };
            let max = if item.is_locked { item.amount_kg } else { 50.0 };
            vars.add(variable().min(min).max(max))
        })
        .collect();

    let problem = apply_hard_constraints(
        vars.minimise(cost_expr(&feed_vars, items))
            .using(default_solver),
        norms,
        &feed_vars,
        items,
    );

    match problem.solve() {
        Ok(solution) => {
            let amounts: Vec<f64> = feed_vars
                .iter()
                .map(|variable| solution.value(*variable))
                .collect();
            let mut built = build_solution(items, &amounts, Some(norms), SolutionStatus::Optimal);
            maybe_preserve_hard_compliant_current(items, norms, &mut built);
            built.applied_strategy = "minimize_cost".to_string();
            Ok(built)
        }
        Err(_) => {
            let mut built = build_solution(
                items,
                &items.iter().map(|item| item.amount_kg).collect::<Vec<_>>(),
                Some(norms),
                SolutionStatus::Infeasible,
            );
            built.applied_strategy = "minimize_cost".to_string();
            Ok(built)
        }
    }
}

fn optimize_balance_nutrients(
    items: &[RationItem],
    norms: Option<&AnimalNorm>,
    policy: MovementPolicy,
    approach: BalanceApproach,
    strategy_name: &str,
) -> anyhow::Result<DietSolution> {
    let bounds = bounded_feed_ranges(items, policy);
    optimize_balance_nutrients_with_bounds(items, norms, &bounds, approach, strategy_name)
}

fn optimize_balance_nutrients_with_bounds(
    items: &[RationItem],
    norms: Option<&AnimalNorm>,
    bounds: &[(f64, f64)],
    approach: BalanceApproach,
    strategy_name: &str,
) -> anyhow::Result<DietSolution> {
    let Some(norms) = norms else {
        return Ok(calculate_current_solution(items, None));
    };
    let exact_amounts = match approach {
        BalanceApproach::SinglePass => solve_single_pass_balance(items, norms, bounds),
        BalanceApproach::Tiered => solve_tiered_balance(items, norms, bounds)
            .or_else(|| solve_single_pass_balance(items, norms, bounds)),
    };

    let mut solution = if let Some(amounts) = exact_amounts {
        build_solution(items, &amounts, Some(norms), SolutionStatus::Optimal)
    } else if let Some(best) = select_best_achievable_balance(items, norms, bounds) {
        let mut built = build_solution(items, &best.amounts, Some(norms), SolutionStatus::Feasible);
        built.best_achievable = true;
        built.relaxed_targets = best.relaxed_targets;
        built
    } else {
        build_solution(
            items,
            &current_amounts(items),
            Some(norms),
            SolutionStatus::Infeasible,
        )
    };
    if solution.best_achievable {
        maybe_keep_current_ration(items, norms, &mut solution);
    }
    solution.applied_strategy = strategy_name.to_string();
    Ok(solution)
}

fn select_best_achievable_balance(
    items: &[RationItem],
    norms: &AnimalNorm,
    bounds: &[(f64, f64)],
) -> Option<BestAchievableSolve> {
    let objective_keys = all_objective_keys(norms);
    let full = solve_best_achievable_balance(items, norms, bounds, &objective_keys);
    let hard_only = solve_best_achievable_balance(items, norms, bounds, &[]);

    match (full, hard_only) {
        (Some(primary), Some(hard_floor)) => {
            if best_achievable_candidate_is_better(items, norms, &hard_floor, &primary) {
                Some(hard_floor)
            } else {
                Some(primary)
            }
        }
        (Some(primary), None) => Some(primary),
        (None, Some(hard_floor)) => Some(hard_floor),
        (None, None) => None,
    }
}

fn maybe_keep_current_ration(
    items: &[RationItem],
    norms: &AnimalNorm,
    solution: &mut DietSolution,
) {
    if items.is_empty() {
        return;
    }

    let current_summary = summary_for_amounts(items, &current_amounts(items));
    let candidate_summary = solution.nutrient_summary.clone();
    if !should_keep_current_solution(norms, &current_summary, &candidate_summary) {
        return;
    }

    let mut preserved = build_solution(
        items,
        &current_amounts(items),
        Some(norms),
        SolutionStatus::Feasible,
    );
    preserved.best_achievable = true;
    preserved.relaxed_targets = solution.relaxed_targets.clone();
    preserved.workflow_notes = vec![String::from("optimize.noteKeptCurrentFallback")];
    *solution = preserved;
}

fn maybe_preserve_hard_compliant_current(
    items: &[RationItem],
    norms: &AnimalNorm,
    solution: &mut DietSolution,
) {
    if items.is_empty() {
        return;
    }

    let current_summary = summary_for_amounts(items, &current_amounts(items));
    let current_score = adequacy_snapshot(norms, &current_summary);
    let candidate_score = adequacy_snapshot(norms, &solution.nutrient_summary);
    if current_score.hard_total == 0
        || current_score.hard_ok != current_score.hard_total
        || candidate_score.hard_ok >= current_score.hard_ok
    {
        return;
    }

    let mut preserved = build_solution(
        items,
        &current_amounts(items),
        Some(norms),
        SolutionStatus::Feasible,
    );
    preserved.workflow_notes = vec![String::from("optimize.noteKeptCurrentHardConstraint")];
    *solution = preserved;
}

fn should_keep_current_solution(
    norms: &AnimalNorm,
    current: &nutrient_calc::NutrientSummary,
    candidate: &nutrient_calc::NutrientSummary,
) -> bool {
    let current_in_range = intake_within_range(norms, current);
    let candidate_in_range = intake_within_range(norms, candidate);
    if current_in_range && !candidate_in_range {
        return true;
    }

    let current_score = adequacy_snapshot(norms, current);
    let candidate_score = adequacy_snapshot(norms, candidate);
    if candidate_score.nutrient_hard_ok < current_score.nutrient_hard_ok
        && candidate_score.core_ok <= current_score.core_ok
    {
        return true;
    }
    candidate_score.core_ok + 1 < current_score.core_ok
        && candidate_score.hard_ok <= current_score.hard_ok
}

fn target_within_tolerance(key: &str, actual: f64, target: f64) -> bool {
    let tolerance = minimum_metric_slack(key).max(target.abs() * 0.10);
    (actual - target).abs() <= tolerance
}

fn target_alignment_count(norms: &AnimalNorm, summary: &nutrient_calc::NutrientSummary) -> usize {
    norms
        .nutrients_target
        .iter()
        .filter(|(key, _)| !target_acts_as_minimum(key))
        .filter_map(|(key, target)| {
            actual_metric_value(norms, summary, key)
                .map(|actual| target_within_tolerance(key, actual, *target))
        })
        .filter(|ok| *ok)
        .count()
}

fn total_target_gap(norms: &AnimalNorm, summary: &nutrient_calc::NutrientSummary) -> f64 {
    norms
        .nutrients_target
        .iter()
        .filter(|(key, _)| !target_acts_as_minimum(key))
        .filter_map(|(key, target)| {
            let actual = actual_metric_value(norms, summary, key)?;
            Some((actual - target).abs() / target.abs().max(1.0))
        })
        .sum()
}

fn best_achievable_candidate_is_better(
    items: &[RationItem],
    norms: &AnimalNorm,
    candidate: &BestAchievableSolve,
    current: &BestAchievableSolve,
) -> bool {
    let candidate_summary = summary_for_amounts(items, &candidate.amounts);
    let current_summary = summary_for_amounts(items, &current.amounts);
    let candidate_score = adequacy_snapshot(norms, &candidate_summary);
    let current_score = adequacy_snapshot(norms, &current_summary);

    if candidate_score.hard_ok != current_score.hard_ok {
        return candidate_score.hard_ok > current_score.hard_ok;
    }
    if candidate_score.core_ok != current_score.core_ok {
        return candidate_score.core_ok > current_score.core_ok;
    }

    let candidate_targets = target_alignment_count(norms, &candidate_summary);
    let current_targets = target_alignment_count(norms, &current_summary);
    if candidate_targets != current_targets {
        return candidate_targets > current_targets;
    }

    let candidate_gap = total_target_gap(norms, &candidate_summary);
    let current_gap = total_target_gap(norms, &current_summary);
    if (candidate_gap - current_gap).abs() > 1e-6 {
        return candidate_gap < current_gap;
    }

    if candidate.relaxed_targets.len() != current.relaxed_targets.len() {
        return candidate.relaxed_targets.len() < current.relaxed_targets.len();
    }

    let candidate_cost = build_solution(
        items,
        &candidate.amounts,
        Some(norms),
        SolutionStatus::Feasible,
    )
    .cost_per_day;
    let current_cost = build_solution(
        items,
        &current.amounts,
        Some(norms),
        SolutionStatus::Feasible,
    )
    .cost_per_day;
    candidate_cost < current_cost
}

#[derive(Clone, Copy)]
struct AdequacySnapshot {
    nutrient_hard_ok: usize,
    hard_ok: usize,
    hard_total: usize,
    core_ok: usize,
}

fn adequacy_snapshot(
    norms: &AnimalNorm,
    summary: &nutrient_calc::NutrientSummary,
) -> AdequacySnapshot {
    let mut nutrient_hard_ok = 0usize;
    let mut hard_ok = 0usize;
    let mut core_ok = 0usize;

    for key in core_alignment_keys(norms) {
        let hard = metric_is_within_constraints(norms, summary, key);
        if hard {
            core_ok += 1;
        }
    }

    for key in norms
        .nutrients_min
        .keys()
        .chain(norms.nutrients_max.keys())
        .collect::<std::collections::HashSet<_>>()
    {
        if metric_is_within_constraints(norms, summary, key.as_str()) {
            nutrient_hard_ok += 1;
            hard_ok += 1;
        }
    }

    if intake_within_range(norms, summary) {
        core_ok += 1;
        hard_ok += 1;
    }

    let hard_total = norms
        .nutrients_min
        .keys()
        .chain(norms.nutrients_max.keys())
        .collect::<std::collections::HashSet<_>>()
        .len()
        + usize::from(norms.feed_intake_min.is_some() || norms.feed_intake_max.is_some());

    AdequacySnapshot {
        nutrient_hard_ok,
        hard_ok,
        hard_total,
        core_ok,
    }
}

fn core_alignment_keys(norms: &AnimalNorm) -> &'static [&'static str] {
    match norms.species.as_str() {
        "swine" => &[
            "energy_oe_pig",
            "crude_protein",
            "crude_protein_pct",
            "lysine_sid",
            "lysine_sid_pct",
            "methionine_cystine_sid",
            "methionine_cystine_tid_pct",
            "calcium",
            "phosphorus",
        ],
        "poultry" => &[
            "energy_oe_poultry",
            "crude_protein_pct",
            "lysine_tid_pct",
            "methionine_cystine_tid_pct",
            "calcium_pct",
            "phosphorus",
        ],
        _ => &[
            "energy_eke",
            "crude_protein",
            "dig_protein_cattle",
            "crude_fiber",
            "calcium",
            "phosphorus",
        ],
    }
}

fn metric_is_within_constraints(
    norms: &AnimalNorm,
    summary: &nutrient_calc::NutrientSummary,
    key: &str,
) -> bool {
    let Some(actual) = actual_metric_value(norms, summary, key) else {
        return false;
    };
    let meets_min = norms
        .nutrients_min
        .get(key)
        .is_none_or(|minimum| actual + 1e-6 >= *minimum);
    let meets_max = norms
        .nutrients_max
        .get(key)
        .is_none_or(|maximum| actual <= *maximum + 1e-6);
    meets_min && meets_max
}

fn intake_within_range(norms: &AnimalNorm, summary: &nutrient_calc::NutrientSummary) -> bool {
    let actual = if norms.species == "cattle" {
        summary.total_dm_kg
    } else {
        summary.total_weight_kg
    };
    norms
        .feed_intake_min
        .is_none_or(|minimum| actual + 1e-6 >= minimum)
        && norms
            .feed_intake_max
            .is_none_or(|maximum| actual <= maximum + 1e-6)
}

#[derive(Clone)]
struct RepairCandidateFeed {
    feed: Feed,
    suggested_amount_kg: f64,
    priority: u8,
    fit_score: i32,
    reasons: Vec<RepairReasonNote>,
}

struct RepairAttempt {
    solution: DietSolution,
    status_rank: u8,
    warning_count: usize,
    used_feed_count: usize,
    priority_sum: u16,
    added_total_kg: f64,
}

struct ConstructorAttempt {
    solution: DietSolution,
    status_rank: u8,
    warning_count: usize,
    feed_count: usize,
    cost_per_day: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum RepairNeedKind {
    Structural(feed_groups::FeedGroup),
    Nutrient(&'static str),
}

#[derive(Clone)]
struct RepairNeed {
    kind: RepairNeedKind,
    priority: u8,
    suggested_amount_kg: f64,
    deficit: f64,
}

#[derive(Clone)]
struct RepairReasonNote {
    priority: u8,
    message: String,
}

#[derive(Clone)]
struct SoftGoal {
    key: String,
    constraint_type: RelaxedConstraintType,
    target: f64,
    metric: MetricExpr,
    positive: Option<Variable>,
    negative: Option<Variable>,
    weight: f64,
}

struct BestAchievableSolve {
    amounts: Vec<f64>,
    relaxed_targets: Vec<RelaxedTarget>,
}

fn optimize_repair_with_additions(
    ration: &RationFull,
    norms: Option<&AnimalNorm>,
    available_feeds: &[Feed],
) -> anyhow::Result<DietSolution> {
    let base_policy = MovementPolicy {
        change_fraction: 0.50,
        change_floor_kg: 5.0,
    };
    let mut base_solution = optimize_balance_nutrients(
        &ration.items,
        norms,
        base_policy,
        BalanceApproach::Tiered,
        "repair_with_additions",
    )?;

    if matches!(base_solution.optimization_status, SolutionStatus::Optimal)
        && !base_solution.best_achievable
    {
        return Ok(base_solution);
    }

    let Some(norms) = norms else {
        return Ok(base_solution);
    };

    let screening = screening::screen_current_feed_set(&ration.items, available_feeds, norms);
    let candidate_feeds =
        repair_candidates_from_context(ration, norms, &screening.recommendations, available_feeds);
    if candidate_feeds.is_empty() {
        return Ok(base_solution);
    }

    let max_added_feeds = max_repair_additions(ration.items.len());
    let mut best_attempt: Option<RepairAttempt> = None;
    for combination in repair_combinations(&candidate_feeds, max_added_feeds) {
        let (items, bounds) =
            build_repair_problem(ration, norms.species.as_str(), &combination, base_policy);
        let solution = optimize_balance_nutrients_with_bounds(
            &items,
            Some(norms),
            &bounds,
            BalanceApproach::Tiered,
            "repair_with_additions",
        )?;
        if !matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ) {
            continue;
        }

        let attempt = build_repair_attempt(solution, &combination);
        if attempt.used_feed_count == 0 {
            continue;
        }
        if best_attempt
            .as_ref()
            .map(|current| repair_attempt_is_better(&attempt, current))
            .unwrap_or(true)
        {
            best_attempt = Some(attempt);
        }
    }

    if let Some(best) = best_attempt {
        return Ok(best.solution);
    }

    base_solution.recommendations = screening.recommendations;
    Ok(base_solution)
}

pub(crate) fn construct_ration_from_library(
    ration: &RationFull,
    norms: &AnimalNorm,
    available_feeds: &[Feed],
) -> anyhow::Result<Option<DietSolution>> {
    let (slots, notes) = super::auto_populate::build_auto_populate_candidate_slots(
        ration.ration.animal_group_id.as_deref(),
        Some(norms),
        available_feeds,
    );
    let candidate_slots = slots
        .into_iter()
        .filter(|slot| !slot.candidates.is_empty())
        .filter(|slot| !should_skip_constructor_group(slot.group))
        .collect::<Vec<_>>();

    let missing_required = notes
        .iter()
        .filter(|note| {
            note.contains("roughage")
                || note.contains("concentrate")
                || note.contains("protein")
                || note.contains("mineral")
        })
        .count();
    if candidate_slots.is_empty() || missing_required > 0 {
        return Ok(None);
    }

    let mut tested = 0usize;
    let mut current = Vec::new();
    let mut best_attempt = None;
    collect_constructor_attempts(
        ration,
        norms,
        &candidate_slots,
        0,
        &mut current,
        &mut tested,
        &mut best_attempt,
    )?;

    Ok(best_attempt.map(|attempt: ConstructorAttempt| {
        let mut solution = attempt.solution;
        solution.applied_strategy = "library_constructor".to_string();
        solution.auto_populated = true;
        solution.workflow_notes = vec![format!(
            "Constructed a starter ration from {} library candidate set{}.",
            tested,
            if tested == 1 { "" } else { "s" }
        )];
        solution
    }))
}

fn should_skip_constructor_group(group: feed_groups::FeedGroup) -> bool {
    matches!(group, feed_groups::FeedGroup::Vitamin)
}

fn collect_constructor_attempts(
    ration: &RationFull,
    norms: &AnimalNorm,
    slots: &[super::auto_populate::AutoPopulateCandidateSlot],
    slot_index: usize,
    current: &mut Vec<super::auto_populate::AutoPopulateItem>,
    tested: &mut usize,
    best_attempt: &mut Option<ConstructorAttempt>,
) -> anyhow::Result<()> {
    if slot_index == slots.len() {
        *tested += 1;
        let items = build_constructor_items(ration, current);
        let bounds = build_constructor_bounds(current, norms);
        let solution = optimize_balance_nutrients_with_bounds(
            &items,
            Some(norms),
            &bounds,
            BalanceApproach::Tiered,
            "library_constructor",
        )?;

        if !matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ) {
            return Ok(());
        }

        let attempt = ConstructorAttempt {
            status_rank: solution_status_rank(solution.optimization_status.clone()),
            warning_count: solution.warnings.len(),
            feed_count: solution.items.len(),
            cost_per_day: solution.cost_per_day,
            solution,
        };
        if best_attempt
            .as_ref()
            .map(|current_attempt| constructor_attempt_is_better(&attempt, current_attempt))
            .unwrap_or(true)
        {
            *best_attempt = Some(attempt);
        }
        return Ok(());
    }

    for candidate in &slots[slot_index].candidates {
        current.push(candidate.clone());
        collect_constructor_attempts(
            ration,
            norms,
            slots,
            slot_index + 1,
            current,
            tested,
            best_attempt,
        )?;
        current.pop();
    }

    Ok(())
}

fn build_constructor_items(
    ration: &RationFull,
    candidates: &[super::auto_populate::AutoPopulateItem],
) -> Vec<RationItem> {
    candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            Some(RationItem {
                id: None,
                ration_id: ration.ration.id.unwrap_or_default(),
                feed_id: candidate.feed.id?,
                feed: Some(candidate.feed.clone()),
                amount_kg: candidate.amount_kg,
                is_locked: false,
                sort_order: index as i32,
            })
        })
        .collect()
}

fn build_constructor_bounds(
    candidates: &[super::auto_populate::AutoPopulateItem],
    norms: &AnimalNorm,
) -> Vec<(f64, f64)> {
    candidates
        .iter()
        .map(|candidate| constructor_feed_bounds(candidate, norms))
        .collect()
}

fn constructor_feed_bounds(
    candidate: &super::auto_populate::AutoPopulateItem,
    norms: &AnimalNorm,
) -> (f64, f64) {
    let species = norms.species.as_str();
    let feed = &candidate.feed;
    let default_cap = match (species, candidate.group) {
        ("cattle", feed_groups::FeedGroup::Roughage) => 18.0,
        ("cattle", feed_groups::FeedGroup::Succulent) => 30.0,
        ("cattle", feed_groups::FeedGroup::Concentrate) => 10.0,
        ("cattle", feed_groups::FeedGroup::Protein | feed_groups::FeedGroup::AnimalOrigin) => 4.0,
        ("cattle", feed_groups::FeedGroup::Mineral) => 1.0,
        ("cattle", feed_groups::FeedGroup::Premix | feed_groups::FeedGroup::Vitamin) => 0.15,
        ("swine", feed_groups::FeedGroup::Concentrate) => 3.5,
        ("swine", feed_groups::FeedGroup::Protein | feed_groups::FeedGroup::AnimalOrigin) => 1.2,
        ("swine", feed_groups::FeedGroup::Mineral) => 0.35,
        ("swine", feed_groups::FeedGroup::Premix | feed_groups::FeedGroup::Vitamin) => 0.08,
        ("poultry", feed_groups::FeedGroup::Concentrate) => 0.12,
        ("poultry", feed_groups::FeedGroup::Protein | feed_groups::FeedGroup::AnimalOrigin) => 0.04,
        ("poultry", feed_groups::FeedGroup::Mineral) => 0.02,
        ("poultry", feed_groups::FeedGroup::Premix | feed_groups::FeedGroup::Vitamin) => 0.01,
        _ => candidate.amount_kg.max(0.5) * 2.5,
    };
    let seeded_cap = match candidate.group {
        feed_groups::FeedGroup::Mineral
        | feed_groups::FeedGroup::Premix
        | feed_groups::FeedGroup::Vitamin => candidate.amount_kg.max(0.01) * 4.0,
        _ => candidate.amount_kg.max(0.1) * 2.5,
    };
    let inclusion_cap = species_max_inclusion(feed, species)
        .map(|share| {
            let intake_cap = norms
                .feed_intake_max
                .or(norms.feed_intake_min)
                .unwrap_or(candidate.amount_kg.max(1.0));
            intake_cap * share / 100.0
        })
        .unwrap_or(0.0);
    let upper = default_cap
        .max(seeded_cap)
        .max(inclusion_cap)
        .max(candidate.amount_kg);

    // Set a minimum floor for mineral/premix feeds to prevent the optimizer
    // from zeroing them out — these provide essential trace minerals/vitamins
    let lower = match candidate.group {
        feed_groups::FeedGroup::Mineral => match species {
            "poultry" => 0.002,
            "swine" => 0.02,
            _ => 0.03,
        },
        feed_groups::FeedGroup::Premix | feed_groups::FeedGroup::Vitamin => match species {
            "poultry" => 0.001,
            "swine" => 0.005,
            _ => 0.01,
        },
        _ => 0.0,
    };
    (lower, upper)
}

fn constructor_attempt_is_better(
    candidate: &ConstructorAttempt,
    current: &ConstructorAttempt,
) -> bool {
    (
        candidate.status_rank,
        candidate.warning_count,
        (candidate.cost_per_day * 100.0).round() as i64,
        candidate.feed_count,
    ) < (
        current.status_rank,
        current.warning_count,
        (current.cost_per_day * 100.0).round() as i64,
        current.feed_count,
    )
}

fn repair_candidates_from_context(
    ration: &RationFull,
    norms: &AnimalNorm,
    recommendations: &[FeedRecommendation],
    available_feeds: &[Feed],
) -> Vec<RepairCandidateFeed> {
    let mut merged = std::collections::HashMap::<i64, RepairCandidateFeed>::new();
    let existing_feed_ids = ration
        .items
        .iter()
        .map(|item| item.feed_id)
        .collect::<std::collections::HashSet<_>>();
    let stage_context = repair_stage_context(ration, norms);
    let needs = repair_needs_from_context(ration, norms, available_feeds);

    for need in &needs {
        for candidate in ranked_repair_candidates_for_need(
            need,
            norms,
            stage_context.as_str(),
            &existing_feed_ids,
            available_feeds,
        ) {
            merge_repair_candidate(&mut merged, candidate);
        }
    }

    if merged.len() < repair_candidate_budget(ration.items.len()) {
        for candidate in repair_candidates_from_screening(
            recommendations,
            available_feeds,
            stage_context.as_str(),
        ) {
            merge_repair_candidate(&mut merged, candidate);
        }
    }

    let mut candidates = merged.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        (
            left.priority,
            -left.fit_score,
            (left.suggested_amount_kg * 1000.0).round() as i64,
        )
            .cmp(&(
                right.priority,
                -right.fit_score,
                (right.suggested_amount_kg * 1000.0).round() as i64,
            ))
    });
    candidates.truncate(repair_candidate_budget(ration.items.len()));
    candidates
}

fn repair_reason_message(need: &RepairNeed) -> String {
    use feed_groups::FeedGroup;

    match need.kind {
        RepairNeedKind::Structural(FeedGroup::Roughage) => {
            "Added as the missing roughage source.".to_string()
        }
        RepairNeedKind::Structural(FeedGroup::Succulent) => {
            "Added as the missing succulent base.".to_string()
        }
        RepairNeedKind::Structural(FeedGroup::Concentrate) => {
            "Added as the missing concentrate source.".to_string()
        }
        RepairNeedKind::Structural(FeedGroup::Protein) => {
            "Added as the missing protein source.".to_string()
        }
        RepairNeedKind::Structural(FeedGroup::AnimalOrigin) => {
            "Added as the missing animal-origin protein source.".to_string()
        }
        RepairNeedKind::Structural(FeedGroup::Mineral) => {
            "Added as the missing mineral source.".to_string()
        }
        RepairNeedKind::Structural(FeedGroup::Premix | FeedGroup::Vitamin) => {
            "Added as the missing vitamin-mineral premix.".to_string()
        }
        RepairNeedKind::Structural(FeedGroup::Other) => {
            "Added as a missing supporting ingredient.".to_string()
        }
        RepairNeedKind::Nutrient(
            "energy_eke" | "energy_oe_cattle" | "energy_oe_pig" | "energy_oe_poultry",
        ) => "Supports the energy deficit.".to_string(),
        RepairNeedKind::Nutrient(
            "crude_protein"
            | "crude_protein_pct"
            | "dig_protein_cattle",
        ) => "Supports the protein deficit.".to_string(),
        RepairNeedKind::Nutrient("lysine" | "lysine_sid" | "lysine_sid_pct" | "lysine_tid_pct") => {
            "Supports lysine coverage.".to_string()
        }
        RepairNeedKind::Nutrient(
            "methionine_cystine" | "methionine_cystine_sid" | "methionine_cystine_tid_pct",
        ) => "Supports sulfur amino acid coverage.".to_string(),
        RepairNeedKind::Nutrient("crude_fiber" | "crude_fiber_pct") => {
            "Supports structural fiber coverage.".to_string()
        }
        RepairNeedKind::Nutrient("starch_pct_dm") => "Supports starch balance.".to_string(),
        RepairNeedKind::Nutrient("calcium" | "calcium_pct") => {
            "Supports calcium coverage.".to_string()
        }
        RepairNeedKind::Nutrient("phosphorus" | "phosphorus_pct") => {
            "Supports phosphorus coverage.".to_string()
        }
        RepairNeedKind::Nutrient("vit_d3") => "Supports vitamin D3 coverage.".to_string(),
        RepairNeedKind::Nutrient("vit_e") => "Supports vitamin E coverage.".to_string(),
        RepairNeedKind::Nutrient(_) => "Supports the remaining nutrient gap.".to_string(),
    }
}

fn screening_reason_note(recommendation: &FeedRecommendation) -> RepairReasonNote {
    RepairReasonNote {
        priority: recommendation.priority,
        message: recommendation.reason.clone(),
    }
}

fn repair_stage_context(ration: &RationFull, norms: &AnimalNorm) -> String {
    ration
        .ration
        .animal_group_id
        .clone()
        .unwrap_or_else(|| norms.id.clone())
}

fn repair_needs_from_context(
    ration: &RationFull,
    norms: &AnimalNorm,
    available_feeds: &[Feed],
) -> Vec<RepairNeed> {
    let present_groups = ration
        .items
        .iter()
        .filter_map(|item| item.feed.as_ref())
        .map(feed_groups::classify_feed)
        .collect::<std::collections::HashSet<_>>();
    let starter_plan = super::auto_populate::build_auto_populate_plan(
        ration.ration.animal_group_id.as_deref(),
        Some(norms),
        available_feeds,
    );
    let mut needs = starter_plan
        .items
        .into_iter()
        .filter(|item| !present_groups.contains(&item.group))
        .map(|item| RepairNeed {
            kind: RepairNeedKind::Structural(item.group),
            priority: starter_group_priority(item.group, norms.species.as_str()),
            suggested_amount_kg: item.amount_kg.max(0.0),
            deficit: 0.0,
        })
        .collect::<Vec<_>>();

    let current_summary =
        summary_for_amounts(ration.items.as_slice(), &current_amounts(&ration.items));
    let baseline_total = screening::baseline_total_weight(norms, &current_summary);

    for (key, _, priority) in screening::screening_targets(norms) {
        let Some(required) = norms
            .nutrients_target
            .get(key)
            .copied()
            .or_else(|| norms.nutrients_min.get(key).copied())
        else {
            continue;
        };
        let Some(actual) = actual_metric_value(norms, &current_summary, key) else {
            continue;
        };
        if actual + 1e-6 >= required {
            continue;
        }

        needs.push(RepairNeed {
            kind: RepairNeedKind::Nutrient(key),
            priority,
            suggested_amount_kg: 0.0,
            deficit: (required - actual)
                .max(0.0)
                .min(baseline_total.max(1.0) * 10.0),
        });
    }

    needs.sort_by_key(|need| need.priority);
    needs
}

fn starter_group_priority(group: feed_groups::FeedGroup, species: &str) -> u8 {
    use feed_groups::FeedGroup;

    match (species, group) {
        ("cattle", FeedGroup::Roughage | FeedGroup::Succulent | FeedGroup::Concentrate) => 1,
        ("swine", FeedGroup::Concentrate | FeedGroup::Protein) => 1,
        ("poultry", FeedGroup::Concentrate | FeedGroup::Protein) => 1,
        (_, FeedGroup::Mineral | FeedGroup::Premix | FeedGroup::Vitamin) => 2,
        (_, FeedGroup::AnimalOrigin) => 3,
        _ => 2,
    }
}

fn repair_candidates_from_screening(
    recommendations: &[FeedRecommendation],
    available_feeds: &[Feed],
    stage_context: &str,
) -> Vec<RepairCandidateFeed> {
    recommendations
        .iter()
        .filter_map(|recommendation| {
            available_feeds
                .iter()
                .find(|feed| feed.id == Some(recommendation.feed_id))
                .cloned()
                .map(|feed| RepairCandidateFeed {
                    fit_score: repair_fit_score_from_screening(
                        &feed,
                        recommendation.priority,
                        stage_context,
                    ),
                    feed,
                    suggested_amount_kg: recommendation.suggested_amount_kg.max(0.0),
                    priority: recommendation.priority,
                    reasons: vec![screening_reason_note(recommendation)],
                })
        })
        .collect()
}

fn merge_repair_candidate(
    merged: &mut std::collections::HashMap<i64, RepairCandidateFeed>,
    candidate: RepairCandidateFeed,
) {
    let Some(feed_id) = candidate.feed.id else {
        return;
    };

    merged
        .entry(feed_id)
        .and_modify(|current| {
            merge_repair_reason_notes(&mut current.reasons, &candidate.reasons);
            if repair_candidate_is_better(&candidate, current) {
                current.feed = candidate.feed.clone();
                current.suggested_amount_kg = candidate.suggested_amount_kg;
                current.priority = candidate.priority;
                current.fit_score = candidate.fit_score;
            }
        })
        .or_insert(candidate);
}

fn merge_repair_reason_notes(target: &mut Vec<RepairReasonNote>, source: &[RepairReasonNote]) {
    for note in source {
        if target
            .iter()
            .any(|existing| existing.message == note.message)
        {
            continue;
        }
        target.push(note.clone());
    }
    target.sort_by(|left, right| {
        (left.priority, left.message.as_str()).cmp(&(right.priority, right.message.as_str()))
    });
}

fn repair_candidate_is_better(
    candidate: &RepairCandidateFeed,
    current: &RepairCandidateFeed,
) -> bool {
    (
        candidate.priority,
        -candidate.fit_score,
        (candidate.suggested_amount_kg * 1000.0).round() as i64,
    ) < (
        current.priority,
        -current.fit_score,
        (current.suggested_amount_kg * 1000.0).round() as i64,
    )
}

fn repair_candidate_budget(existing_item_count: usize) -> usize {
    match existing_item_count {
        0..=1 => 8,
        2..=3 => 7,
        _ => 6,
    }
}

fn ranked_repair_candidates_for_need(
    need: &RepairNeed,
    norms: &AnimalNorm,
    stage_context: &str,
    existing_feed_ids: &std::collections::HashSet<i64>,
    available_feeds: &[Feed],
) -> Vec<RepairCandidateFeed> {
    let species = norms.species.as_str();
    let baseline_total = norms
        .feed_intake_max
        .or(norms.feed_intake_min)
        .unwrap_or_else(|| match species {
            "swine" => 3.0,
            "poultry" => 0.12,
            _ => 25.0,
        });

    let mut ranked = available_feeds
        .iter()
        .filter(|feed| feed.id.is_some())
        .filter(|feed| {
            if let Some(id) = feed.id {
                !existing_feed_ids.contains(&id)
            } else {
                false
            }
        })
        .filter(|feed| feed_groups::is_feed_allowed_for_context(feed, species, Some(stage_context)))
        .filter(|feed| feed_matches_repair_need(feed, need, species))
        .filter_map(|feed| {
            let score = repair_candidate_score(feed, need, species, stage_context);
            if score <= 0.0 {
                return None;
            }

            let suggested_amount_kg = repair_suggested_amount(feed, need, baseline_total);
            if suggested_amount_kg <= 0.0 {
                return None;
            }

            Some(RepairCandidateFeed {
                feed: feed.clone(),
                suggested_amount_kg: (suggested_amount_kg * 1000.0).round() / 1000.0,
                priority: need.priority,
                fit_score: (score * 100.0).round() as i32,
                reasons: vec![RepairReasonNote {
                    priority: need.priority,
                    message: repair_reason_message(need),
                }],
            })
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        (
            -left.fit_score,
            (left.suggested_amount_kg * 1000.0).round() as i64,
        )
            .cmp(&(
                -right.fit_score,
                (right.suggested_amount_kg * 1000.0).round() as i64,
            ))
    });
    ranked.truncate(repair_candidates_per_need(need));
    ranked
}

fn repair_matches_group(key: &str, group: feed_groups::FeedGroup, species: &str) -> bool {
    feed_groups::preferred_groups_for_nutrient(key, species).contains(&group)
}

fn feed_matches_repair_need(feed: &Feed, need: &RepairNeed, species: &str) -> bool {
    let group = feed_groups::classify_feed(feed);
    match need.kind {
        RepairNeedKind::Structural(required_group) => group == required_group,
        RepairNeedKind::Nutrient(key) => {
            repair_matches_group(key, group, species)
                && screening::nutrient_density(feed, key) > 0.0
        }
    }
}

fn repair_candidate_group_bonus(
    need: &RepairNeed,
    group: feed_groups::FeedGroup,
    species: &str,
) -> f64 {
    use feed_groups::FeedGroup;

    match need.kind {
        RepairNeedKind::Structural(required_group) if group == required_group => 40.0,
        RepairNeedKind::Structural(_) => 0.0,
        RepairNeedKind::Nutrient(key) => match key {
            "energy_eke" | "energy_oe_cattle" => match group {
                FeedGroup::Concentrate => 28.0,
                FeedGroup::Succulent => 22.0,
                FeedGroup::Roughage => 16.0,
                _ => 0.0,
            },
            "energy_oe_pig" | "energy_oe_poultry" => match group {
                FeedGroup::Concentrate => 30.0,
                FeedGroup::Protein => 12.0,
                _ => 0.0,
            },
            "crude_protein"
            | "crude_protein_pct"
            | "lysine"
            | "lysine_sid"
            | "lysine_sid_pct"
            | "lysine_tid_pct"
            | "methionine_cystine"
            | "methionine_cystine_sid"
            | "methionine_cystine_tid_pct" => match group {
                FeedGroup::Protein => 34.0,
                FeedGroup::AnimalOrigin => 26.0,
                FeedGroup::Concentrate => 10.0,
                _ => 0.0,
            },
            "crude_fiber" | "crude_fiber_pct" => match group {
                FeedGroup::Roughage => 34.0,
                FeedGroup::Succulent => 24.0,
                _ => 0.0,
            },
            "calcium" | "calcium_pct" | "phosphorus" | "phosphorus_pct" => match group {
                FeedGroup::Mineral => 36.0,
                FeedGroup::Premix => 12.0,
                _ => 0.0,
            },
            "vit_d3" | "vit_e" => match group {
                FeedGroup::Premix => 38.0,
                FeedGroup::Vitamin => 24.0,
                FeedGroup::Mineral => 12.0,
                _ => 0.0,
            },
            _ if species == "cattle" => match group {
                FeedGroup::Concentrate => 18.0,
                FeedGroup::Roughage | FeedGroup::Succulent => 12.0,
                _ => 0.0,
            },
            _ => match group {
                FeedGroup::Concentrate => 18.0,
                FeedGroup::Protein => 12.0,
                _ => 0.0,
            },
        },
    }
}

fn repair_candidate_score(
    feed: &Feed,
    need: &RepairNeed,
    species: &str,
    stage_context: &str,
) -> f64 {
    let group = feed_groups::classify_feed(feed);
    let stage_bonus = feed_groups::stage_fit_bonus(feed, stage_context);
    let species_bonus = feed_groups::species_fit_bonus(feed, species);
    let price_bonus = if feed.price_per_ton.unwrap_or(0.0) > 0.0 {
        4.0
    } else {
        0.0
    };

    let base = match need.kind {
        RepairNeedKind::Structural(required_group) => match required_group {
            feed_groups::FeedGroup::Premix | feed_groups::FeedGroup::Vitamin => {
                feed_groups::vitamin_density(feed).max(1.0).log10() * 40.0
            }
            _ => feed_groups::score_feed_for_group(feed, required_group, species),
        },
        RepairNeedKind::Nutrient(key) if matches!(key, "vit_d3" | "vit_e") => {
            screening::nutrient_density(feed, key).max(1.0).log10() * 40.0
        }
        RepairNeedKind::Nutrient(key) => screening::nutrient_density(feed, key),
    };

    base + repair_candidate_group_bonus(need, group, species)
        + stage_bonus
        + species_bonus
        + price_bonus
}

fn repair_suggested_amount(feed: &Feed, need: &RepairNeed, baseline_total: f64) -> f64 {
    match need.kind {
        RepairNeedKind::Structural(_) => need.suggested_amount_kg.max(0.0),
        RepairNeedKind::Nutrient(key) => {
            screening::estimate_suggested_amount(feed, key, need.deficit, baseline_total)
        }
    }
}

fn repair_candidates_per_need(need: &RepairNeed) -> usize {
    match need.kind {
        RepairNeedKind::Structural(feed_groups::FeedGroup::Mineral)
        | RepairNeedKind::Structural(feed_groups::FeedGroup::Premix)
        | RepairNeedKind::Structural(feed_groups::FeedGroup::Vitamin) => 1,
        RepairNeedKind::Structural(_) => 2,
        RepairNeedKind::Nutrient(
            "calcium"
            | "calcium_pct"
            | "phosphorus"
            | "phosphorus_pct"
            | "vit_d3"
            | "vit_e",
        ) => 1,
        RepairNeedKind::Nutrient(_) => 2,
    }
}

fn repair_fit_score_from_screening(feed: &Feed, priority: u8, stage_context: &str) -> i32 {
    let species_hint = if feed_groups::is_feed_species_appropriate(feed, "cattle") {
        "cattle"
    } else if feed_groups::is_feed_species_appropriate(feed, "swine") {
        "swine"
    } else {
        "poultry"
    };
    let score = feed_groups::species_fit_bonus(feed, species_hint)
        + feed_groups::stage_fit_bonus(feed, stage_context)
        + (10_u8.saturating_sub(priority) as f64);
    (score * 100.0).round() as i32
}

fn repair_combinations(
    candidates: &[RepairCandidateFeed],
    max_added_feeds: usize,
) -> Vec<Vec<RepairCandidateFeed>> {
    let mut combinations = Vec::new();

    let max_size = max_added_feeds.max(1).min(candidates.len());
    for size in 1..=max_size {
        let mut current = Vec::new();
        collect_repair_combinations(candidates, size, 0, &mut current, &mut combinations);
    }

    combinations
}

fn collect_repair_combinations(
    candidates: &[RepairCandidateFeed],
    target_size: usize,
    start_index: usize,
    current: &mut Vec<RepairCandidateFeed>,
    combinations: &mut Vec<Vec<RepairCandidateFeed>>,
) {
    if current.len() == target_size {
        combinations.push(current.clone());
        return;
    }

    for index in start_index..candidates.len() {
        current.push(candidates[index].clone());
        collect_repair_combinations(candidates, target_size, index + 1, current, combinations);
        current.pop();
    }
}

fn max_repair_additions(existing_item_count: usize) -> usize {
    match existing_item_count {
        0..=1 => 4,
        2 => 3,
        3..=4 => 2,
        _ => 2,
    }
}

fn build_repair_problem(
    ration: &RationFull,
    species: &str,
    candidates: &[RepairCandidateFeed],
    policy: MovementPolicy,
) -> (Vec<RationItem>, Vec<(f64, f64)>) {
    let mut items = ration.items.clone();
    let mut bounds = bounded_feed_ranges(&items, policy);

    for (offset, candidate) in candidates.iter().enumerate() {
        let Some(feed_id) = candidate.feed.id else {
            continue;
        };
        items.push(RationItem {
            id: None,
            ration_id: ration.ration.id.unwrap_or_default(),
            feed_id,
            feed: Some(candidate.feed.clone()),
            amount_kg: 0.0,
            is_locked: false,
            sort_order: (ration.items.len() + offset + 1) as i32,
        });
        bounds.push((
            repair_feed_lower_bound(&candidate.feed, candidate.suggested_amount_kg, species),
            repair_feed_upper_bound(&candidate.feed, candidate.suggested_amount_kg, species),
        ));
    }

    (items, bounds)
}

fn repair_feed_lower_bound(feed: &Feed, suggested_amount_kg: f64, species: &str) -> f64 {
    use feed_groups::FeedGroup;
    let group = feed_groups::classify_feed(feed);

    let floor = match (species, group) {
        ("poultry", FeedGroup::Premix | FeedGroup::Vitamin) => 0.001,
        ("poultry", FeedGroup::Mineral) => 0.005,
        ("poultry", FeedGroup::Protein | FeedGroup::AnimalOrigin) => 0.02,
        ("poultry", _) => 0.05,
        ("swine", FeedGroup::Premix | FeedGroup::Vitamin) => 0.005,
        ("swine", FeedGroup::Mineral) => 0.01,
        ("swine", FeedGroup::Protein | FeedGroup::AnimalOrigin) => 0.05,
        ("swine", _) => 0.10,
        (_, FeedGroup::Premix | FeedGroup::Vitamin) => 0.01,
        (_, FeedGroup::Mineral) => 0.05,
        (_, FeedGroup::Protein | FeedGroup::AnimalOrigin) => 0.10,
        (_, FeedGroup::Concentrate) => 0.25,
        (_, FeedGroup::Roughage | FeedGroup::Succulent) => 0.50,
        _ => 0.10,
    };

    let cap = repair_feed_upper_bound(feed, suggested_amount_kg, species);
    (suggested_amount_kg * 0.1).clamp(floor, cap * 0.5)
}

fn repair_feed_upper_bound(feed: &Feed, suggested_amount_kg: f64, species: &str) -> f64 {
    use feed_groups::FeedGroup;
    let group = feed_groups::classify_feed(feed);

    let (floor, cap) = match (species, group) {
        ("poultry", FeedGroup::Premix | FeedGroup::Vitamin) => (0.003, 0.05),
        ("poultry", FeedGroup::Mineral) => (0.01, 0.15),
        ("poultry", FeedGroup::Protein | FeedGroup::AnimalOrigin) => (0.03, 0.25),
        ("poultry", _) => (0.05, 0.50),
        ("swine", FeedGroup::Premix | FeedGroup::Vitamin) => (0.01, 0.15),
        ("swine", FeedGroup::Mineral) => (0.03, 0.50),
        ("swine", FeedGroup::Protein | FeedGroup::AnimalOrigin) => (0.10, 1.50),
        ("swine", _) => (0.25, 2.00),
        (_, FeedGroup::Premix | FeedGroup::Vitamin) => (0.05, 0.50),
        (_, FeedGroup::Mineral) => (0.10, 2.00),
        (_, FeedGroup::Protein | FeedGroup::AnimalOrigin) => (0.25, 4.00),
        (_, FeedGroup::Concentrate) => (0.50, 5.00),
        (_, FeedGroup::Roughage | FeedGroup::Succulent) => (1.00, 8.00),
        _ => (0.25, 3.00),
    };

    (suggested_amount_kg.max(floor) * 3.0).min(cap)
}

fn prune_repair_reason_messages(notes: &[RepairReasonNote]) -> Vec<String> {
    notes
        .iter()
        .take(3)
        .map(|note| note.message.clone())
        .collect()
}

fn build_auto_added_feeds(
    solution: &DietSolution,
    candidates: &[RepairCandidateFeed],
) -> Vec<AutoAddedFeed> {
    let mut added = candidates
        .iter()
        .filter_map(|candidate| {
            let feed_id = candidate.feed.id?;
            let item = solution
                .items
                .iter()
                .find(|item| item.feed_id == feed_id && item.amount_kg > 0.001)?;

            Some(AutoAddedFeed {
                feed_id,
                feed_name: item.feed_name.clone(),
                amount_kg: item.amount_kg,
                reasons: prune_repair_reason_messages(&candidate.reasons),
            })
        })
        .collect::<Vec<_>>();

    added.sort_by(|left, right| left.feed_name.cmp(&right.feed_name));
    added
}

fn build_repair_attempt(
    mut solution: DietSolution,
    candidates: &[RepairCandidateFeed],
) -> RepairAttempt {
    let used_candidates = candidates
        .iter()
        .filter(|candidate| {
            let Some(feed_id) = candidate.feed.id else {
                return false;
            };
            solution
                .items
                .iter()
                .any(|item| item.feed_id == feed_id && item.amount_kg > 0.001)
        })
        .cloned()
        .collect::<Vec<_>>();

    solution.auto_added_feeds = build_auto_added_feeds(&solution, &used_candidates);

    RepairAttempt {
        status_rank: solution_status_rank(solution.optimization_status.clone()),
        warning_count: solution.warnings.len(),
        used_feed_count: used_candidates.len(),
        priority_sum: used_candidates
            .iter()
            .map(|candidate| candidate.priority as u16)
            .sum(),
        added_total_kg: used_candidates
            .iter()
            .filter_map(|candidate| {
                let feed_id = candidate.feed.id?;
                solution
                    .items
                    .iter()
                    .find(|item| item.feed_id == feed_id)
                    .map(|item| item.amount_kg)
            })
            .sum(),
        solution,
    }
}

fn repair_attempt_is_better(candidate: &RepairAttempt, current: &RepairAttempt) -> bool {
    (
        candidate.status_rank,
        candidate.warning_count,
        candidate.used_feed_count,
        candidate.priority_sum,
        (candidate.added_total_kg * 1000.0).round() as i64,
        (candidate.solution.cost_per_day * 100.0).round() as i64,
    ) < (
        current.status_rank,
        current.warning_count,
        current.used_feed_count,
        current.priority_sum,
        (current.added_total_kg * 1000.0).round() as i64,
        (current.solution.cost_per_day * 100.0).round() as i64,
    )
}

fn solution_status_rank(status: SolutionStatus) -> u8 {
    match status {
        SolutionStatus::Optimal => 0,
        SolutionStatus::Feasible => 1,
        SolutionStatus::Infeasible => 2,
        SolutionStatus::Unbounded => 3,
        SolutionStatus::Error => 4,
    }
}

fn solve_tiered_balance(
    items: &[RationItem],
    norms: &AnimalNorm,
    bounds: &[(f64, f64)],
) -> Option<Vec<f64>> {
    let tiers = tier_keys(norms);
    let mut bands = Vec::new();
    let mut group_bands = Vec::new();
    let mut last_amounts = None;
    let mut applied_tier_count = 0usize;

    for keys in tiers.iter().filter(|keys| !keys.is_empty()) {
        let next = solve_target_pass(items, norms, bounds, keys, &bands, &group_bands)?;
        let summary = summary_for_amounts(items, &next);
        let tolerance = if applied_tier_count == 0 { 0.02 } else { 0.05 };
        bands.extend(build_metric_bands(norms, keys, &summary, tolerance));
        group_bands.extend(build_group_share_bands(
            norms, items, &next, keys, tolerance,
        ));
        last_amounts = Some(next);
        applied_tier_count += 1;
    }

    if bands.is_empty() {
        return last_amounts.or_else(|| solve_single_pass_balance(items, norms, bounds));
    }

    solve_cost_pass(items, norms, bounds, &bands, &group_bands).or(last_amounts)
}

fn solve_single_pass_balance(
    items: &[RationItem],
    norms: &AnimalNorm,
    bounds: &[(f64, f64)],
) -> Option<Vec<f64>> {
    let objective_keys = all_objective_keys(norms);
    let supported_keys = supported_objective_keys(items, &objective_keys);
    solve_target_pass(items, norms, bounds, &supported_keys, &[], &[])
}

fn solve_target_pass(
    items: &[RationItem],
    norms: &AnimalNorm,
    bounds: &[(f64, f64)],
    objective_keys: &[&'static str],
    locked_bands: &[MetricBand],
    locked_group_bands: &[GroupShareBand],
) -> Option<Vec<f64>> {
    let objective_keys = supported_objective_keys(items, objective_keys);
    let mut vars = variables!();
    let feed_vars: Vec<Variable> = bounds
        .iter()
        .map(|(min, max)| vars.add(variable().min(*min).max(*max)))
        .collect();

    let total_feed = total_feed_expr(&feed_vars);
    let total_dm = total_dm_expr(&feed_vars, items);
    let cost = cost_expr(&feed_vars, items);

    let mut target_deviations: Vec<TargetDeviation> = objective_keys
        .iter()
        .filter_map(|key| {
            let target = norms
                .nutrients_target
                .get(*key)
                .copied()
                .or_else(|| norms.nutrients_min.get(*key).copied())?;
            let metric = resolve_metric_expr(norms, key, &feed_vars, items)?;
            Some(TargetDeviation {
                metric,
                target,
                positive: vars.add(variable().min(0.0)),
                negative: vars.add(variable().min(0.0)),
                weight: nutrient_priority_weight(norms, key) / target.abs().max(1.0),
            })
        })
        .collect();
    if metric_has_support_in_items(items, "ca_p_ratio") {
        if let Some(target_ratio) = target_ca_p_ratio(norms, &objective_keys) {
            if let Some(metric) = resolve_metric_expr(norms, "ca_p_ratio", &feed_vars, items) {
                target_deviations.push(TargetDeviation {
                    metric,
                    target: target_ratio,
                    positive: vars.add(variable().min(0.0)),
                    negative: vars.add(variable().min(0.0)),
                    weight: nutrient_priority_weight(norms, "ca_p_ratio")
                        / target_ratio.abs().max(1.0),
                });
            }
        }
    }
    for (ratio_key, target_ratio) in target_amino_ratio_goals(norms, &objective_keys) {
        if !metric_has_support_in_items(items, ratio_key) {
            continue;
        }
        if let Some(metric) = resolve_metric_expr(norms, ratio_key, &feed_vars, items) {
            target_deviations.push(TargetDeviation {
                metric,
                target: target_ratio,
                positive: vars.add(variable().min(0.0)),
                negative: vars.add(variable().min(0.0)),
                weight: nutrient_priority_weight(norms, ratio_key) / target_ratio.abs().max(1.0),
            });
        }
    }

    let target_objective: Expression = if target_deviations.is_empty() {
        0.0.into()
    } else {
        target_deviations
            .iter()
            .map(|deviation| (deviation.positive + deviation.negative) * deviation.weight)
            .sum()
    };

    let group_share_deviations: Vec<GroupShareDeviation> =
        target_group_shares(norms, &objective_keys, items)
            .into_iter()
            .map(|(group, share, weight)| GroupShareDeviation {
                group,
                share,
                positive: vars.add(variable().min(0.0)),
                negative: vars.add(variable().min(0.0)),
                weight,
            })
            .collect();

    let group_share_objective: Expression = if group_share_deviations.is_empty() {
        0.0.into()
    } else {
        group_share_deviations
            .iter()
            .map(|deviation| (deviation.positive + deviation.negative) * deviation.weight)
            .sum()
    };

    let objective = (target_objective + group_share_objective) * 100.0 + cost * 0.0001;

    let mut problem = apply_hard_constraints(
        vars.minimise(objective).using(default_solver),
        norms,
        &feed_vars,
        items,
    );
    for deviation in &target_deviations {
        problem = problem.with(constraint!(
            deviation.metric.lhs()
                - deviation
                    .metric
                    .rhs(deviation.target, &total_feed, &total_dm)
                == deviation.positive - deviation.negative
        ));
    }
    for deviation in &group_share_deviations {
        let group_dm = group_dm_expr(&feed_vars, items, deviation.group);
        problem = problem.with(constraint!(
            group_dm - total_dm.clone() * deviation.share
                == deviation.positive - deviation.negative
        ));
    }
    problem = apply_metric_bands(problem, norms, &feed_vars, items, locked_bands);
    problem = apply_group_share_bands(problem, &feed_vars, items, locked_group_bands);

    match problem.solve() {
        Ok(solution) => Some(
            feed_vars
                .iter()
                .map(|variable| solution.value(*variable))
                .collect(),
        ),
        Err(_) => None,
    }
}

fn solve_best_achievable_balance(
    items: &[RationItem],
    norms: &AnimalNorm,
    bounds: &[(f64, f64)],
    objective_keys: &[&'static str],
) -> Option<BestAchievableSolve> {
    let objective_keys = supported_objective_keys(items, objective_keys);
    let mut vars = variables!();
    let feed_vars: Vec<Variable> = bounds
        .iter()
        .map(|(min, max)| vars.add(variable().min(*min).max(*max)))
        .collect();

    let total_feed = total_feed_expr(&feed_vars);
    let total_dm = total_dm_expr(&feed_vars, items);
    let intake_expr = if norms.species == "cattle" {
        total_dm.clone()
    } else {
        total_feed.clone()
    };
    let intake_key = if norms.species == "cattle" {
        "dry_matter_intake"
    } else {
        "feed_intake"
    };
    let cost = cost_expr(&feed_vars, items);

    let mut soft_goals = Vec::new();
    if let Some(min_intake) = norms.feed_intake_min {
        soft_goals.push(SoftGoal {
            key: intake_key.to_string(),
            constraint_type: RelaxedConstraintType::Min,
            target: min_intake,
            metric: MetricExpr::Absolute(intake_expr.clone()),
            positive: None,
            negative: Some(vars.add(variable().min(0.0))),
            weight: relaxed_goal_weight(norms, intake_key, RelaxedConstraintType::Min, min_intake),
        });
    }
    if let Some(max_intake) = norms.feed_intake_max {
        soft_goals.push(SoftGoal {
            key: intake_key.to_string(),
            constraint_type: RelaxedConstraintType::Max,
            target: max_intake,
            metric: MetricExpr::Absolute(intake_expr.clone()),
            positive: Some(vars.add(variable().min(0.0))),
            negative: None,
            weight: relaxed_goal_weight(norms, intake_key, RelaxedConstraintType::Max, max_intake),
        });
    }

    for (key, min_value) in &norms.nutrients_min {
        if !metric_has_support_in_items(items, key) {
            continue;
        }
        let Some(metric) = resolve_metric_expr(norms, key, &feed_vars, items) else {
            continue;
        };
        soft_goals.push(SoftGoal {
            key: key.clone(),
            constraint_type: RelaxedConstraintType::Min,
            target: *min_value,
            metric,
            positive: None,
            negative: Some(vars.add(variable().min(0.0))),
            weight: relaxed_goal_weight(norms, key, RelaxedConstraintType::Min, *min_value),
        });
    }

    for (key, target_value) in &norms.nutrients_target {
        if norms.nutrients_min.contains_key(key) || !target_acts_as_minimum(key) {
            continue;
        }
        if !metric_has_support_in_items(items, key) {
            continue;
        }
        let Some(metric) = resolve_metric_expr(norms, key, &feed_vars, items) else {
            continue;
        };
        soft_goals.push(SoftGoal {
            key: key.clone(),
            constraint_type: RelaxedConstraintType::Min,
            target: *target_value,
            metric,
            positive: None,
            negative: Some(vars.add(variable().min(0.0))),
            weight: relaxed_goal_weight(norms, key, RelaxedConstraintType::Min, *target_value),
        });
    }

    for (key, max_value) in &norms.nutrients_max {
        if !metric_has_support_in_items(items, key) {
            continue;
        }
        let Some(metric) = resolve_metric_expr(norms, key, &feed_vars, items) else {
            continue;
        };
        soft_goals.push(SoftGoal {
            key: key.clone(),
            constraint_type: RelaxedConstraintType::Max,
            target: *max_value,
            metric,
            positive: Some(vars.add(variable().min(0.0))),
            negative: None,
            weight: relaxed_goal_weight(norms, key, RelaxedConstraintType::Max, *max_value),
        });
    }

    for key in &objective_keys {
        if !metric_has_support_in_items(items, key) {
            continue;
        }
        let Some(target) = norms
            .nutrients_target
            .get(*key)
            .copied()
            .or_else(|| norms.nutrients_min.get(*key).copied())
        else {
            continue;
        };
        if norms.nutrients_min.contains_key(*key) || target_acts_as_minimum(key) {
            continue;
        }
        let Some(metric) = resolve_metric_expr(norms, key, &feed_vars, items) else {
            continue;
        };
        soft_goals.push(SoftGoal {
            key: (*key).to_string(),
            constraint_type: RelaxedConstraintType::Target,
            target,
            metric,
            positive: Some(vars.add(variable().min(0.0))),
            negative: Some(vars.add(variable().min(0.0))),
            weight: relaxed_goal_weight(norms, key, RelaxedConstraintType::Target, target),
        });
    }

    if metric_has_support_in_items(items, "ca_p_ratio") {
        if let Some(target_ratio) = target_ca_p_ratio(norms, &objective_keys) {
            if let Some(metric) = resolve_metric_expr(norms, "ca_p_ratio", &feed_vars, items) {
                soft_goals.push(SoftGoal {
                    key: "ca_p_ratio".to_string(),
                    constraint_type: RelaxedConstraintType::Target,
                    target: target_ratio,
                    metric,
                    positive: Some(vars.add(variable().min(0.0))),
                    negative: Some(vars.add(variable().min(0.0))),
                    weight: relaxed_goal_weight(
                        norms,
                        "ca_p_ratio",
                        RelaxedConstraintType::Target,
                        target_ratio,
                    ),
                });
            }
        }
    }
    for (ratio_key, target_ratio) in target_amino_ratio_goals(norms, &objective_keys) {
        if !metric_has_support_in_items(items, ratio_key) {
            continue;
        }
        if let Some(metric) = resolve_metric_expr(norms, ratio_key, &feed_vars, items) {
            soft_goals.push(SoftGoal {
                key: ratio_key.to_string(),
                constraint_type: RelaxedConstraintType::Target,
                target: target_ratio,
                metric,
                positive: Some(vars.add(variable().min(0.0))),
                negative: Some(vars.add(variable().min(0.0))),
                weight: relaxed_goal_weight(
                    norms,
                    ratio_key,
                    RelaxedConstraintType::Target,
                    target_ratio,
                ),
            });
        }
    }

    let group_share_deviations: Vec<GroupShareDeviation> =
        target_group_shares(norms, &objective_keys, items)
            .into_iter()
            .map(|(group, share, weight)| GroupShareDeviation {
                group,
                share,
                positive: vars.add(variable().min(0.0)),
                negative: vars.add(variable().min(0.0)),
                weight,
            })
            .collect();

    let goal_objective: Expression = soft_goals
        .iter()
        .map(|goal| {
            let mut goal_expr: Expression = 0.0.into();
            if let Some(positive) = goal.positive {
                goal_expr += positive * goal.weight;
            }
            if let Some(negative) = goal.negative {
                goal_expr += negative * goal.weight;
            }
            goal_expr
        })
        .sum();
    let group_share_objective: Expression = group_share_deviations
        .iter()
        .map(|deviation| (deviation.positive + deviation.negative) * deviation.weight)
        .sum();
    let objective = (goal_objective + group_share_objective) * 100.0 + cost * 0.0001;

    let mut problem = apply_practical_constraints_with_options(
        vars.minimise(objective).using(default_solver),
        norms,
        &feed_vars,
        items,
        false,
    );

    for goal in &soft_goals {
        match goal.constraint_type {
            RelaxedConstraintType::Min => {
                let slack = goal.negative.expect("min goals must use negative slack");
                problem = problem.with(constraint!(
                    goal.metric.lhs() + slack
                        >= goal.metric.rhs(goal.target, &total_feed, &total_dm)
                ));
            }
            RelaxedConstraintType::Max => {
                let slack = goal.positive.expect("max goals must use positive slack");
                problem = problem.with(constraint!(
                    goal.metric.lhs() - slack
                        <= goal.metric.rhs(goal.target, &total_feed, &total_dm)
                ));
            }
            RelaxedConstraintType::Target => {
                problem = problem.with(constraint!(
                    goal.metric.lhs() - goal.metric.rhs(goal.target, &total_feed, &total_dm)
                        == goal.positive.unwrap() - goal.negative.unwrap()
                ));
            }
        }
    }

    for deviation in &group_share_deviations {
        let group_dm = group_dm_expr(&feed_vars, items, deviation.group);
        problem = problem.with(constraint!(
            group_dm - total_dm.clone() * deviation.share
                == deviation.positive - deviation.negative
        ));
    }

    let solution = problem.solve().ok()?;
    let amounts: Vec<f64> = feed_vars
        .iter()
        .map(|variable| solution.value(*variable))
        .collect();
    let summary = summary_for_amounts(items, &amounts);
    let relaxed_targets = collect_relaxed_targets(norms, &summary, &soft_goals);

    Some(BestAchievableSolve {
        amounts,
        relaxed_targets,
    })
}

fn solve_cost_pass(
    items: &[RationItem],
    norms: &AnimalNorm,
    bounds: &[(f64, f64)],
    locked_bands: &[MetricBand],
    locked_group_bands: &[GroupShareBand],
) -> Option<Vec<f64>> {
    let mut vars = variables!();
    let feed_vars: Vec<Variable> = bounds
        .iter()
        .map(|(min, max)| vars.add(variable().min(*min).max(*max)))
        .collect();
    let mut problem = apply_hard_constraints(
        vars.minimise(cost_expr(&feed_vars, items))
            .using(default_solver),
        norms,
        &feed_vars,
        items,
    );
    problem = apply_metric_bands(problem, norms, &feed_vars, items, locked_bands);
    problem = apply_group_share_bands(problem, &feed_vars, items, locked_group_bands);

    match problem.solve() {
        Ok(solution) => Some(
            feed_vars
                .iter()
                .map(|variable| solution.value(*variable))
                .collect(),
        ),
        Err(_) => None,
    }
}

fn apply_metric_bands<M: SolverModel>(
    mut problem: M,
    norms: &AnimalNorm,
    feed_vars: &[Variable],
    items: &[RationItem],
    bands: &[MetricBand],
) -> M {
    let total_feed = total_feed_expr(feed_vars);
    let total_dm = total_dm_expr(feed_vars, items);

    for band in bands {
        let Some(metric) = resolve_metric_expr(norms, band.key, feed_vars, items) else {
            continue;
        };
        let absolute_slack = minimum_metric_slack(band.key);
        let delta = (band.value.abs() * band.tolerance).max(absolute_slack);
        let lower = (band.value - delta).max(0.0);
        let upper = band.value + delta;
        problem = problem.with(constraint!(
            metric.lhs() >= metric.rhs(lower, &total_feed, &total_dm)
        ));
        problem = problem.with(constraint!(
            metric.lhs() <= metric.rhs(upper, &total_feed, &total_dm)
        ));
    }

    problem
}

fn minimum_metric_slack(key: &str) -> f64 {
    match key {
        "dry_matter_intake" | "feed_intake" => 0.10,
        "ca_p_ratio" => 0.05,
        "methionine_cystine_lys_ratio" => 0.02,
        "energy_eke" | "energy_oe_cattle" | "energy_oe_pig" | "energy_oe_poultry" => 0.2,
        "crude_protein_pct"
        | "lysine_sid_pct"
        | "lysine_tid_pct"
        | "methionine_cystine_tid_pct"
        | "crude_fiber_pct"
        | "calcium_pct"
        | "phosphorus_pct"
        | "starch_pct_dm" => 0.05,
        "vit_d3" => 250.0,
        "vit_e" => 5.0,
        _ => 1.0,
    }
}

fn relaxed_goal_weight(
    norms: &AnimalNorm,
    key: &str,
    constraint_type: RelaxedConstraintType,
    target: f64,
) -> f64 {
    let base = match key {
        "dry_matter_intake" | "feed_intake" => 110.0,
        _ => nutrient_priority_weight(norms, key),
    };
    let multiplier = match constraint_type {
        RelaxedConstraintType::Target => 1.0,
        RelaxedConstraintType::Min | RelaxedConstraintType::Max => 10.0,
    };
    base * multiplier / target.abs().max(1.0)
}

fn collect_relaxed_targets(
    norms: &AnimalNorm,
    summary: &nutrient_calc::NutrientSummary,
    soft_goals: &[SoftGoal],
) -> Vec<RelaxedTarget> {
    let mut relaxed = soft_goals
        .iter()
        .filter_map(|goal| {
            let actual = actual_metric_value(norms, summary, goal.key.as_str())?;
            let delta = actual - goal.target;
            let tolerance = minimum_metric_slack(goal.key.as_str());
            let was_relaxed = match goal.constraint_type {
                RelaxedConstraintType::Min => delta < -tolerance,
                RelaxedConstraintType::Max => delta > tolerance,
                RelaxedConstraintType::Target => delta.abs() > tolerance,
            };
            if !was_relaxed {
                return None;
            }

            Some(RelaxedTarget {
                key: goal.key.clone(),
                constraint_type: goal.constraint_type.clone(),
                target: goal.target,
                actual,
                delta,
            })
        })
        .collect::<Vec<_>>();

    relaxed.sort_by(|left, right| {
        right
            .delta
            .abs()
            .partial_cmp(&left.delta.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    relaxed
}

fn bounded_feed_ranges(items: &[RationItem], policy: MovementPolicy) -> Vec<(f64, f64)> {
    items
        .iter()
        .map(|item| {
            if item.is_locked {
                return (item.amount_kg, item.amount_kg);
            }

            let delta = (item.amount_kg * policy.change_fraction).max(policy.change_floor_kg);

            // Allow most feeds to go to zero for maximum flexibility.
            // But keep a small floor for mineral/premix feeds that provide
            // essential trace minerals and vitamins the optimizer can't get elsewhere.
            let group = item
                .feed
                .as_ref()
                .map(feed_groups::classify_feed)
                .unwrap_or(feed_groups::FeedGroup::Other);
            let min_floor = match group {
                feed_groups::FeedGroup::Mineral | feed_groups::FeedGroup::Premix => {
                    item.amount_kg * 0.2 // keep at least 20% of initial amount
                }
                _ => 0.0,
            };
            let min = (item.amount_kg - delta).max(min_floor);
            let max = (item.amount_kg + delta).max(policy.change_floor_kg);
            (min, max)
        })
        .collect()
}

fn tier_keys(norms: &AnimalNorm) -> [Vec<&'static str>; 3] {
    let mut tier1 = Vec::new();
    let mut tier2 = Vec::new();
    let mut tier3 = Vec::new();

    for &key in ALL_OPTIMIZER_KEYS {
        if !key_present_in_norms(norms, key) {
            continue;
        }
        match constraint_tier_for_key(norms, key) {
            ConstraintTier::Tier1 => tier1.push(key),
            ConstraintTier::Tier2 => tier2.push(key),
            ConstraintTier::Tier3 => tier3.push(key),
        }
    }

    [tier1, tier2, tier3]
}

fn nutrient_priority_weight(norms: &AnimalNorm, key: &str) -> f64 {
    let base_weight: f64 = match constraint_tier_for_key(norms, key) {
        ConstraintTier::Tier1 => 70.0,
        ConstraintTier::Tier2 => 45.0,
        ConstraintTier::Tier3 => 18.0,
    };

    if key.starts_with("vit_") || key == "carotene" {
        return base_weight.max(25.0);
    }
    if matches!(
        key,
        "magnesium"
            | "potassium"
            | "sodium"
            | "sulfur"
            | "iron"
            | "copper"
            | "zinc"
            | "manganese"
            | "cobalt"
            | "iodine"
    ) {
        return base_weight.max(35.0);
    }

    match norms.species.as_str() {
        "swine" => match key {
            "energy_oe_pig" => 100.0,
            "lysine_sid" | "lysine_sid_pct" | "crude_protein_pct" | "crude_protein" => 90.0,
            "methionine_cystine_sid" | "methionine_cystine_tid_pct" | "methionine_cystine" => 80.0,
            "methionine_cystine_lys_ratio" => 85.0,
            "calcium" | "phosphorus" | "ca_p_ratio" => 55.0,
            _ => base_weight,
        },
        "poultry" => match key {
            "energy_oe_poultry" | "lysine_tid_pct" => 100.0,
            "crude_protein_pct" | "methionine_cystine_tid_pct" | "methionine_cystine" => 92.0,
            "methionine_cystine_lys_ratio" => 95.0,
            "calcium_pct" | "ca_p_ratio" => 62.0,
            "phosphorus_pct" | "phosphorus" => 55.0,
            _ => base_weight,
        },
        _ => match key {
            "energy_eke" | "energy_oe_cattle" => 100.0,
            "crude_protein"
            | "dig_protein_cattle"
            | "dig_protein_cattle_pct_cp"
            | "lysine"
            | "methionine_cystine" => 82.0,
            "crude_fiber" | "crude_fiber_pct" => 62.0,
            "starch" | "starch_pct_dm" => 52.0,
            "calcium" | "phosphorus" | "ca_p_ratio" => 42.0,
            _ => base_weight,
        },
    }
}

fn build_metric_bands(
    norms: &AnimalNorm,
    keys: &[&'static str],
    summary: &nutrient_calc::NutrientSummary,
    tolerance: f64,
) -> Vec<MetricBand> {
    let mut bands: Vec<MetricBand> = keys
        .iter()
        .filter_map(|key| {
            actual_metric_value(norms, summary, key).map(|value| MetricBand {
                key,
                value,
                tolerance,
            })
        })
        .collect();

    if should_optimize_ca_p_ratio(norms, keys) {
        if let Some(value) = actual_metric_value(norms, summary, "ca_p_ratio") {
            bands.push(MetricBand {
                key: "ca_p_ratio",
                value,
                tolerance,
            });
        }
    }
    for (ratio_key, _) in target_amino_ratio_goals(norms, keys) {
        if let Some(value) = actual_metric_value(norms, summary, ratio_key) {
            bands.push(MetricBand {
                key: ratio_key,
                value,
                tolerance,
            });
        }
    }

    bands
}

pub(super) fn actual_metric_value(
    norms: &AnimalNorm,
    summary: &nutrient_calc::NutrientSummary,
    key: &str,
) -> Option<f64> {
    match key {
        "dry_matter_intake" => Some(summary.total_dm_kg),
        "feed_intake" => Some(summary.total_weight_kg),
        "methionine_cystine_lys_ratio" if summary.lysine > 0.0 => {
            if summary.methionine_cystine > 0.0 {
                Some(summary.methionine_cystine / summary.lysine)
            } else {
                None
            }
        }
        "ca_p_ratio" if summary.phosphorus > 0.0 => Some(summary.ca_p_ratio),
        _ => nutrient_calc::metric_value_for_norm(summary, norms, key),
    }
}

fn current_amounts(items: &[RationItem]) -> Vec<f64> {
    items.iter().map(|item| item.amount_kg).collect()
}

fn summary_for_amounts(items: &[RationItem], amounts: &[f64]) -> nutrient_calc::NutrientSummary {
    let updated_items: Vec<RationItem> = items
        .iter()
        .zip(amounts.iter())
        .map(|(item, amount)| RationItem {
            amount_kg: *amount,
            ..item.clone()
        })
        .collect();
    nutrient_calc::calculate_nutrients(&updated_items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::norm_resolution::{
        resolved_default_norm, resolved_norm_group_id, NormResolveRequest, ResolveAnimalProperties,
    };
    use crate::db::feeds::Feed;
    use std::collections::HashMap;

    fn make_item(feed_id: i64, feed: Feed, amount_kg: f64) -> RationItem {
        RationItem {
            id: Some(feed_id),
            ration_id: 1,
            feed_id,
            feed: Some(feed),
            amount_kg,
            is_locked: false,
            sort_order: feed_id as i32,
        }
    }

    fn normalized_metric_gap(
        norms: &AnimalNorm,
        summary: &nutrient_calc::NutrientSummary,
        key: &str,
    ) -> f64 {
        let Some(actual) = actual_metric_value(norms, summary, key) else {
            return 0.0;
        };

        let mut gap = 0.0;

        if let Some(target) = norms
            .nutrients_target
            .get(key)
            .copied()
            .filter(|value| *value > 0.0)
        {
            gap += (actual - target).abs() / target.max(1.0);
        }
        if let Some(minimum) = norms
            .nutrients_min
            .get(key)
            .copied()
            .filter(|value| *value > 0.0)
        {
            if actual < minimum {
                gap += (minimum - actual) / minimum.max(1.0);
            }
        }
        if let Some(maximum) = norms
            .nutrients_max
            .get(key)
            .copied()
            .filter(|value| *value > 0.0)
        {
            if actual > maximum {
                gap += (actual - maximum) / maximum.max(1.0);
            }
        }

        gap
    }

    fn intake_gap(norms: &AnimalNorm, summary: &nutrient_calc::NutrientSummary) -> f64 {
        let actual = if norms.species == "cattle" {
            summary.total_dm_kg
        } else {
            summary.total_weight_kg
        };

        let mut gap = 0.0;
        if let (Some(minimum), Some(maximum)) = (norms.feed_intake_min, norms.feed_intake_max) {
            let midpoint = (minimum + maximum) / 2.0;
            gap += (actual - midpoint).abs() / midpoint.max(1.0);
        }
        if let Some(minimum) = norms.feed_intake_min.filter(|value| *value > 0.0) {
            if actual < minimum {
                gap += (minimum - actual) / minimum.max(1.0);
            }
        }
        if let Some(maximum) = norms.feed_intake_max.filter(|value| *value > 0.0) {
            if actual > maximum {
                gap += (actual - maximum) / maximum.max(1.0);
            }
        }

        gap
    }

    fn core_alignment_score(
        norms: &AnimalNorm,
        summary: &nutrient_calc::NutrientSummary,
        keys: &[&str],
    ) -> f64 {
        keys.iter()
            .map(|key| normalized_metric_gap(norms, summary, key))
            .sum::<f64>()
            + intake_gap(norms, summary)
    }

    #[test]
    fn test_current_solution() {
        let items = vec![];
        let solution = calculate_current_solution(&items, None);
        assert_eq!(solution.optimization_status, SolutionStatus::Feasible);
        assert_eq!(solution.cost_per_day, 0.0);
    }

    #[test]
    fn test_balance_nutrients_moves_feeds() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(9.1),
            crude_protein: Some(140.0),
            crude_fiber: Some(520.0),
            calcium: Some(8.0),
            phosphorus: Some(2.5),
            price_per_ton: Some(8000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(2),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(13.2),
            crude_protein: Some(115.0),
            crude_fiber: Some(180.0),
            calcium: Some(0.6),
            phosphorus: Some(3.5),
            price_per_ton: Some(14000.0),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(3),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(12.9),
            crude_protein: Some(440.0),
            crude_fiber: Some(120.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            price_per_ton: Some(32000.0),
            ..Default::default()
        };

        let items = vec![
            make_item(1, hay, 10.0),
            make_item(2, barley, 7.0),
            make_item(3, soy, 0.8),
        ];

        let mut min = HashMap::new();
        let mut target = HashMap::new();
        let mut max = HashMap::new();
        min.insert("energy_eke".to_string(), 14.0);
        target.insert("energy_eke".to_string(), 14.5);
        min.insert("crude_protein".to_string(), 2800.0);
        target.insert("crude_protein".to_string(), 3000.0);
        min.insert("crude_fiber_pct".to_string(), 28.0);
        max.insert("crude_fiber_pct".to_string(), 38.0);
        min.insert("calcium".to_string(), 90.0);
        target.insert("calcium".to_string(), 100.0);
        min.insert("phosphorus".to_string(), 55.0);
        target.insert("phosphorus".to_string(), 60.0);

        let norms = AnimalNorm {
            id: "cattle_dairy_test".to_string(),
            species: "cattle".to_string(),
            nutrients_min: min,
            nutrients_max: max,
            nutrients_target: target,
            feed_intake_min: Some(14.0),
            feed_intake_max: Some(18.0),
            ..Default::default()
        };

        let solution = optimize_balance_nutrients(
            &items,
            Some(&norms),
            MovementPolicy {
                change_fraction: 0.60,
                change_floor_kg: 5.0,
            },
            BalanceApproach::Tiered,
            "test",
        )
        .unwrap();

        let total_delta: f64 = solution
            .items
            .iter()
            .zip(items.iter())
            .map(|(optimized, original)| (optimized.amount_kg - original.amount_kg).abs())
            .sum();

        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));
        assert!(total_delta > 0.5);
    }

    #[test]
    fn test_crude_fiber_pct_constraint_is_respected() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(8.5),
            crude_protein: Some(110.0),
            crude_fiber: Some(620.0),
            ..Default::default()
        };
        let grain = Feed {
            id: Some(2),
            name_ru: "Corn".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(87.0),
            energy_oe_cattle: Some(13.5),
            crude_protein: Some(95.0),
            crude_fiber: Some(120.0),
            ..Default::default()
        };

        let items = vec![make_item(1, hay, 4.0), make_item(2, grain, 12.0)];

        let mut min = HashMap::new();
        let mut target = HashMap::new();
        min.insert("energy_eke".to_string(), 12.5);
        target.insert("energy_eke".to_string(), 13.0);
        min.insert("crude_fiber_pct".to_string(), 30.0);
        let norms = AnimalNorm {
            id: "cattle_dairy_test".to_string(),
            species: "cattle".to_string(),
            nutrients_min: min,
            nutrients_target: target,
            feed_intake_min: Some(12.0),
            feed_intake_max: Some(16.0),
            ..Default::default()
        };

        let solution = optimize_balance_nutrients(
            &items,
            Some(&norms),
            MovementPolicy {
                change_fraction: 0.60,
                change_floor_kg: 5.0,
            },
            BalanceApproach::Tiered,
            "test",
        )
        .unwrap();

        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "crude_fiber_pct").unwrap()
                >= 29.9
        );
    }

    #[test]
    fn test_max_inclusion_limit_is_enforced_for_swine() {
        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("swine_finisher".to_string()),
                animal_count: 1,
                name: "Swine ration".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(
                    1,
                    Feed {
                        id: Some(1),
                        name_ru: "Barley".to_string(),
                        category: "grain".to_string(),
                        dry_matter: Some(86.0),
                        energy_oe_pig: Some(13.3),
                        crude_protein: Some(115.0),
                        lysine: Some(4.5),
                        price_per_ton: Some(15000.0),
                        ..Default::default()
                    },
                    2.1,
                ),
                make_item(
                    2,
                    Feed {
                        id: Some(2),
                        name_ru: "Soybean meal".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(88.0),
                        energy_oe_pig: Some(12.8),
                        crude_protein: Some(440.0),
                        lysine: Some(28.0),
                        methionine_cystine: Some(12.0),
                        max_inclusion_pig: Some(10.0),
                        price_per_ton: Some(32000.0),
                        ..Default::default()
                    },
                    0.55,
                ),
                make_item(
                    3,
                    Feed {
                        id: Some(3),
                        name_ru: "Sunflower meal".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(89.0),
                        energy_oe_pig: Some(10.8),
                        crude_protein: Some(340.0),
                        lysine: Some(12.0),
                        methionine_cystine: Some(11.0),
                        price_per_ton: Some(25000.0),
                        ..Default::default()
                    },
                    0.35,
                ),
            ],
        };

        let norms = AnimalNorm {
            id: "swine_finisher".to_string(),
            species: "swine".to_string(),
            nutrients_min: HashMap::from([("energy_oe_pig".to_string(), 12.7)]),
            nutrients_target: HashMap::from([
                ("lysine_sid".to_string(), 7.5),
                ("crude_protein_pct".to_string(), 16.5),
            ]),
            feed_intake_min: Some(2.4),
            feed_intake_max: Some(3.2),
            ..Default::default()
        };

        let solution = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();
        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));

        let total_feed: f64 = solution.items.iter().map(|item| item.amount_kg).sum();
        let soy_amount = solution
            .items
            .iter()
            .find(|item| item.feed_id == 2)
            .map(|item| item.amount_kg)
            .unwrap_or(0.0);

        assert!(soy_amount <= total_feed * 0.10 + 1e-6);
    }

    #[test]
    fn test_group_share_targets_pull_cattle_ration_toward_template() {
        let silage = Feed {
            id: Some(1),
            name_ru: "Corn silage".to_string(),
            category: "silage".to_string(),
            dry_matter: Some(32.0),
            energy_oe_cattle: Some(10.8),
            crude_protein: Some(75.0),
            crude_fiber: Some(150.0),
            price_per_ton: Some(5000.0),
            ..Default::default()
        };
        let hay = Feed {
            id: Some(2),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(8.8),
            crude_protein: Some(110.0),
            crude_fiber: Some(600.0),
            price_per_ton: Some(8000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(3),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(13.2),
            crude_protein: Some(115.0),
            crude_fiber: Some(180.0),
            price_per_ton: Some(14000.0),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(4),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(12.9),
            crude_protein: Some(440.0),
            crude_fiber: Some(120.0),
            price_per_ton: Some(32000.0),
            ..Default::default()
        };

        let items = vec![
            make_item(1, silage, 4.0),
            make_item(2, hay, 9.0),
            make_item(3, barley, 6.0),
            make_item(4, soy, 0.8),
        ];

        let norms = AnimalNorm {
            id: "cattle_dairy".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("energy_eke".to_string(), 12.5),
                ("crude_fiber_pct".to_string(), 28.0),
            ]),
            nutrients_max: HashMap::from([("crude_fiber_pct".to_string(), 40.0)]),
            nutrients_target: HashMap::from([("energy_eke".to_string(), 13.0)]),
            feed_intake_min: Some(14.0),
            feed_intake_max: Some(20.0),
            ..Default::default()
        };

        let solution = optimize_balance_nutrients(
            &items,
            Some(&norms),
            MovementPolicy {
                change_fraction: 0.75,
                change_floor_kg: 5.0,
            },
            BalanceApproach::Tiered,
            "test",
        )
        .unwrap();

        assert_eq!(solution.optimization_status, SolutionStatus::Optimal);

        let initial_amounts = items.iter().map(|item| item.amount_kg).collect::<Vec<_>>();
        let initial_succulent_share =
            group_share_for_amounts(&items, &initial_amounts, feed_groups::FeedGroup::Succulent)
                .unwrap_or(0.0);

        let optimized_amounts = items
            .iter()
            .map(|item| {
                solution
                    .items
                    .iter()
                    .find(|optimized| optimized.feed_id == item.feed_id)
                    .map(|optimized| optimized.amount_kg)
                    .unwrap_or(0.0)
            })
            .collect::<Vec<_>>();

        let succulent_share = group_share_for_amounts(
            &items,
            &optimized_amounts,
            feed_groups::FeedGroup::Succulent,
        )
        .unwrap_or(0.0);
        // Soft constraint pull with actual dairy matrix opt_pct (15%) might be slightly lower due to hard constraints (e.g., energy/cost),
        // but it should stay non-zero and bounded.
        assert!(
            succulent_share >= 0.05,
            "succulent share moved from {initial_succulent_share:.3} to {succulent_share:.3}"
        );
    }

    #[test]
    fn test_dairy_ration_matrix_enforces_roughage_minimum() {
        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Dairy matrix".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(
                    1,
                    Feed {
                        id: Some(1),
                        name_ru: "Alfalfa hay".to_string(),
                        category: "roughage".to_string(),
                        dry_matter: Some(88.0),
                        energy_oe_cattle: Some(9.4),
                        crude_protein: Some(180.0),
                        crude_fiber: Some(460.0),
                        price_per_ton: Some(11000.0),
                        ..Default::default()
                    },
                    1.5,
                ),
                make_item(
                    2,
                    Feed {
                        id: Some(2),
                        name_ru: "Corn silage".to_string(),
                        category: "silage".to_string(),
                        dry_matter: Some(32.0),
                        energy_oe_cattle: Some(10.7),
                        crude_protein: Some(80.0),
                        crude_fiber: Some(320.0),
                        price_per_ton: Some(6000.0),
                        ..Default::default()
                    },
                    10.0,
                ),
                make_item(
                    3,
                    Feed {
                        id: Some(3),
                        name_ru: "Barley".to_string(),
                        category: "grain".to_string(),
                        dry_matter: Some(86.0),
                        energy_oe_cattle: Some(13.1),
                        crude_protein: Some(118.0),
            crude_fiber: Some(180.0),
                        price_per_ton: Some(18000.0),
                        ..Default::default()
                    },
                    6.0,
                ),
                make_item(
                    4,
                    Feed {
                        id: Some(4),
                        name_ru: "Soybean meal".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(89.0),
                        energy_oe_cattle: Some(12.4),
                        crude_protein: Some(430.0),
                        crude_fiber: Some(110.0),
                        price_per_ton: Some(34000.0),
                        ..Default::default()
                    },
                    0.7,
                ),
                make_item(
                    5,
                    Feed {
                        id: Some(5),
                        name_ru: "Feed chalk".to_string(),
                        category: "mineral".to_string(),
                        dry_matter: Some(99.0),
                        calcium: Some(360.0),
                        price_per_ton: Some(12000.0),
                        ..Default::default()
                    },
                    0.05,
                ),
                make_item(
                    6,
                    Feed {
                        id: Some(6),
                        name_ru: "Premix P60".to_string(),
                        category: "premix".to_string(),
                        carotene: Some(1_000_000.0),
                        price_per_ton: Some(70000.0),
                        ..Default::default()
                    },
                    0.02,
                ),
            ],
        };

        let norms = AnimalNorm {
            id: "cattle_dairy".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("energy_eke".to_string(), 10.0),
                ("crude_fiber_pct".to_string(), 28.0),
                ("calcium".to_string(), 40.0),
            ]),
            nutrients_max: HashMap::from([("crude_fiber_pct".to_string(), 40.0)]),
            nutrients_target: HashMap::from([("energy_eke".to_string(), 10.8)]),
            feed_intake_min: Some(12.0),
            feed_intake_max: Some(18.0),
            ..Default::default()
        };

        let solution = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();
        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));

        let optimized_amounts = solution
            .items
            .iter()
            .map(|item| item.amount_kg)
            .collect::<Vec<_>>();
        let roughage_share = group_share_for_amounts(
            &ration.items,
            &optimized_amounts,
            feed_groups::FeedGroup::Roughage,
        )
        .unwrap_or(0.0);

        assert!(
            roughage_share >= 0.35 - 1e-6,
            "roughage share should respect dairy matrix, got {roughage_share:.3}"
        );
    }

    #[test]
    fn test_swine_ration_matrix_limits_roughage_share() {
        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("swine_finisher".to_string()),
                animal_count: 1,
                name: "Swine matrix".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(
                    1,
                    Feed {
                        id: Some(1),
                        name_ru: "Barley".to_string(),
                        category: "grain".to_string(),
                        dry_matter: Some(86.0),
                        energy_oe_pig: Some(13.2),
                        crude_protein: Some(115.0),
                        price_per_ton: Some(15000.0),
                        ..Default::default()
                    },
                    1.8,
                ),
                make_item(
                    2,
                    Feed {
                        id: Some(2),
                        name_ru: "Soybean meal".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(89.0),
                        energy_oe_pig: Some(12.8),
                        crude_protein: Some(430.0),
                        lysine: Some(27.0),
                        price_per_ton: Some(33000.0),
                        ..Default::default()
                    },
                    0.4,
                ),
                make_item(
                    3,
                    Feed {
                        id: Some(3),
                        name_ru: "Hay meal".to_string(),
                        category: "roughage".to_string(),
                        dry_matter: Some(88.0),
                        energy_oe_pig: Some(8.4),
                        crude_protein: Some(120.0),
                        crude_fiber: Some(260.0),
                        price_per_ton: Some(9000.0),
                        ..Default::default()
                    },
                    0.5,
                ),
                make_item(
                    4,
                    Feed {
                        id: Some(4),
                        name_ru: "Feed chalk".to_string(),
                        category: "mineral".to_string(),
                        dry_matter: Some(99.0),
                        calcium: Some(360.0),
                        price_per_ton: Some(9000.0),
                        ..Default::default()
                    },
                    0.03,
                ),
                make_item(
                    5,
                    Feed {
                        id: Some(5),
                        name_ru: "Premix starter".to_string(),
                        category: "premix".to_string(),
                        carotene: Some(900000.0),
                        price_per_ton: Some(70000.0),
                        ..Default::default()
                    },
                    0.02,
                ),
            ],
        };

        let norms = AnimalNorm {
            id: "swine_finisher".to_string(),
            species: "swine".to_string(),
            nutrients_min: HashMap::from([
                ("energy_oe_pig".to_string(), 12.4),
                ("crude_protein_pct".to_string(), 16.0),
            ]),
            nutrients_target: HashMap::from([
                ("lysine_sid".to_string(), 7.2),
                ("calcium".to_string(), 7.0),
            ]),
            feed_intake_min: Some(2.0),
            feed_intake_max: Some(3.0),
            ..Default::default()
        };

        let solution = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();
        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));

        let optimized_amounts = solution
            .items
            .iter()
            .map(|item| item.amount_kg)
            .collect::<Vec<_>>();
        let roughage_share =
            matrix_category_share_for_amounts(&ration.items, &optimized_amounts, "roughage")
                .unwrap_or(0.0);

        assert!(
            roughage_share <= 0.05 + 1e-6,
            "roughage share should respect swine matrix, got {roughage_share:.3}"
        );
    }

    #[test]
    fn test_broiler_ration_matrix_limits_roughage_share() {
        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("poultry_broiler".to_string()),
                animal_count: 1,
                name: "Broiler matrix".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(
                    1,
                    Feed {
                        id: Some(1),
                        name_ru: "Corn".to_string(),
                        category: "grain".to_string(),
                        dry_matter: Some(88.0),
                        energy_oe_poultry: Some(12.8),
                        crude_protein: Some(90.0),
                        price_per_ton: Some(14000.0),
                        ..Default::default()
                    },
                    0.09,
                ),
                make_item(
                    2,
                    Feed {
                        id: Some(2),
                        name_ru: "Soybean meal".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(89.0),
                        energy_oe_poultry: Some(11.9),
                        crude_protein: Some(440.0),
                        lysine: Some(29.0),
                        price_per_ton: Some(32000.0),
                        ..Default::default()
                    },
                    0.025,
                ),
                make_item(
                    3,
                    Feed {
                        id: Some(3),
                        name_ru: "Grass meal".to_string(),
                        category: "roughage".to_string(),
                        dry_matter: Some(89.0),
                        energy_oe_poultry: Some(7.0),
                        crude_protein: Some(140.0),
                        crude_fiber: Some(260.0),
                        price_per_ton: Some(9000.0),
                        ..Default::default()
                    },
                    0.02,
                ),
                make_item(
                    4,
                    Feed {
                        id: Some(4),
                        name_ru: "Feed chalk".to_string(),
                        category: "mineral".to_string(),
                        dry_matter: Some(99.0),
                        calcium: Some(360.0),
                        price_per_ton: Some(9000.0),
                        ..Default::default()
                    },
                    0.003,
                ),
                make_item(
                    5,
                    Feed {
                        id: Some(5),
                        name_ru: "Broiler premix".to_string(),
                        category: "premix".to_string(),
                        carotene: Some(900000.0),
                        price_per_ton: Some(70000.0),
                        ..Default::default()
                    },
                    0.002,
                ),
            ],
        };

        let norms = AnimalNorm {
            id: "poultry_broiler".to_string(),
            species: "poultry".to_string(),
            nutrients_min: HashMap::from([
                ("energy_oe_poultry".to_string(), 12.1),
                ("crude_protein_pct".to_string(), 20.0),
                ("calcium_pct".to_string(), 0.9),
            ]),
            nutrients_target: HashMap::from([("lysine_tid_pct".to_string(), 1.05)]),
            feed_intake_min: Some(0.10),
            feed_intake_max: Some(0.16),
            ..Default::default()
        };

        let solution = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();
        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));

        let optimized_amounts = solution
            .items
            .iter()
            .map(|item| item.amount_kg)
            .collect::<Vec<_>>();
        let roughage_share =
            matrix_category_share_for_amounts(&ration.items, &optimized_amounts, "roughage")
                .unwrap_or(0.0);

        assert!(
            roughage_share <= 0.03 + 1e-6,
            "roughage share should respect broiler matrix, got {roughage_share:.3}"
        );
    }

    #[test]
    fn test_ca_p_ratio_target_adds_calcium_source_for_cattle() {
        let silage = Feed {
            id: Some(1),
            name_ru: "Corn silage".to_string(),
            category: "silage".to_string(),
            dry_matter: Some(32.0),
            energy_oe_cattle: Some(10.8),
            crude_protein: Some(70.0),
            crude_fiber: Some(150.0),
            calcium: Some(2.0),
            phosphorus: Some(2.5),
            price_per_ton: Some(5000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(2),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(13.2),
            crude_protein: Some(115.0),
            crude_fiber: Some(180.0),
            calcium: Some(0.6),
            phosphorus: Some(3.6),
            price_per_ton: Some(14000.0),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(3),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            dry_matter: Some(99.0),
            calcium: Some(380.0),
            phosphorus: Some(0.0),
            price_per_ton: Some(10500.0),
            ..Default::default()
        };

        let items = vec![
            make_item(1, silage, 10.0),
            make_item(2, barley, 6.0),
            make_item(3, chalk, 0.03),
        ];

        let norms = AnimalNorm {
            id: "cattle_dairy".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("energy_eke".to_string(), 8.0),
                ("calcium".to_string(), 30.0),
                ("phosphorus".to_string(), 40.0),
            ]),
            nutrients_target: HashMap::from([("energy_eke".to_string(), 8.5)]),
            feed_intake_min: Some(8.0),
            feed_intake_max: Some(14.0),
            ..Default::default()
        };

        let initial_summary = summary_for_amounts(
            &items,
            &items.iter().map(|item| item.amount_kg).collect::<Vec<_>>(),
        );
        let solution = optimize_balance_nutrients(
            &items,
            Some(&norms),
            MovementPolicy {
                change_fraction: 0.75,
                change_floor_kg: 5.0,
            },
            BalanceApproach::Tiered,
            "test",
        )
        .unwrap();

        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));
        assert!(solution.nutrient_summary.ca_p_ratio > initial_summary.ca_p_ratio + 0.4);

        let chalk_amount = solution
            .items
            .iter()
            .find(|item| item.feed_id == 3)
            .map(|item| item.amount_kg)
            .unwrap_or(0.0);
        assert!(chalk_amount > 0.08, "chalk amount was {chalk_amount:.3}");
    }

    #[test]
    fn test_best_achievable_solution_is_returned_for_sparse_ration() {
        let wheat = Feed {
            id: Some(1),
            name_ru: "Wheat".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(88.0),
            energy_oe_poultry: Some(12.5),
            crude_protein: Some(130.0),
            calcium: Some(0.8),
            phosphorus: Some(3.0),
            ..Default::default()
        };
        let items = vec![make_item(1, wheat, 0.10)];

        let mut min = HashMap::new();
        min.insert("calcium_pct".to_string(), 5.0);
        let norms = AnimalNorm {
            id: "poultry_test".to_string(),
            species: "poultry".to_string(),
            nutrients_min: min,
            ..Default::default()
        };

        let solution = optimize_balance_nutrients(
            &items,
            Some(&norms),
            MovementPolicy {
                change_fraction: 0.50,
                change_floor_kg: 0.05,
            },
            BalanceApproach::Tiered,
            "test",
        )
        .unwrap();

        assert_eq!(solution.optimization_status, SolutionStatus::Feasible);
        assert!(solution.best_achievable);
        assert!(!solution.relaxed_targets.is_empty());
        assert!(solution
            .relaxed_targets
            .iter()
            .any(|target| target.key == "calcium_pct"));
    }

    #[test]
    fn test_repair_mode_can_add_missing_feed_from_library() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(9.0),
            crude_protein: Some(110.0),
            crude_fiber: Some(560.0),
            calcium: Some(4.0),
            phosphorus: Some(2.0),
            price_per_ton: Some(8000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(2),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(13.1),
            crude_protein: Some(115.0),
            crude_fiber: Some(180.0),
            calcium: Some(0.6),
            phosphorus: Some(3.6),
            price_per_ton: Some(14000.0),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(3),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            dry_matter: Some(99.0),
            calcium: Some(380.0),
            phosphorus: Some(0.0),
            price_per_ton: Some(10500.0),
            ..Default::default()
        };
        let layer_premix = Feed {
            id: Some(4),
            name_ru: "Premix for layers".to_string(),
            category: "premix".to_string(),
            subcategory: Some("layer_premix".to_string()),
            dry_matter: Some(95.0),
            calcium: Some(120.0),
            phosphorus: Some(40.0),
            carotene: Some(1_500_000.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Repair test".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(1, hay.clone(), 8.0),
                make_item(2, barley.clone(), 6.0),
            ],
        };

        let norms = AnimalNorm {
            id: "cattle_dairy".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("energy_eke".to_string(), 10.0),
                ("calcium".to_string(), 70.0),
                ("phosphorus".to_string(), 32.0),
                ("crude_fiber_pct".to_string(), 28.0),
            ]),
            nutrients_max: HashMap::from([("crude_fiber_pct".to_string(), 45.0)]),
            feed_intake_min: Some(10.0),
            feed_intake_max: Some(16.0),
            ..Default::default()
        };

        let tiered = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();
        assert_eq!(tiered.optimization_status, SolutionStatus::Feasible);
        assert!(tiered.best_achievable);
        assert!(tiered
            .relaxed_targets
            .iter()
            .any(|target| target.key == "calcium"));

        let library = vec![hay, barley, chalk, layer_premix];
        let repaired = optimize_with_library(
            &ration,
            OptimizationMode::RepairWithAdditions,
            Some(&norms),
            Some(&library),
        )
        .unwrap();

        assert_eq!(repaired.optimization_status, SolutionStatus::Optimal);
        assert_eq!(repaired.applied_strategy, "repair_with_additions");
        assert!(repaired
            .items
            .iter()
            .any(|item| item.feed_id == 3 && item.amount_kg > 0.05));
        assert!(!repaired.items.iter().any(|item| item.feed_id == 4));
        let chalk_note = repaired
            .auto_added_feeds
            .iter()
            .find(|feed| feed.feed_id == 3)
            .expect("repair result should explain the added chalk");
        assert!(chalk_note
            .reasons
            .iter()
            .any(|reason| reason.contains("calcium") || reason.contains("mineral")));
    }

    #[test]
    fn test_repair_mode_completes_single_feed_cattle_ration_from_library() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(9.0),
            crude_protein: Some(120.0),
            crude_fiber: Some(560.0),
            calcium: Some(4.0),
            phosphorus: Some(2.0),
            price_per_ton: Some(8000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(2),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(13.2),
            crude_protein: Some(115.0),
            crude_fiber: Some(180.0),
            calcium: Some(0.6),
            phosphorus: Some(3.6),
            price_per_ton: Some(14000.0),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(3),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(89.0),
            energy_oe_cattle: Some(12.8),
            crude_protein: Some(430.0),
            lysine: Some(28.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            price_per_ton: Some(32000.0),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(4),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            dry_matter: Some(99.0),
            calcium: Some(380.0),
            phosphorus: Some(0.0),
            price_per_ton: Some(10500.0),
            ..Default::default()
        };
        let layer_premix = Feed {
            id: Some(5),
            name_ru: "Премикс для кур-несушек".to_string(),
            category: "premix".to_string(),
            subcategory: Some("layer_premix".to_string()),
            calcium: Some(120.0),
            carotene: Some(1_500_000.0),
            price_per_ton: Some(68000.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Single hay".to_string(),
                ..Default::default()
            },
            items: vec![make_item(1, hay.clone(), 10.0)],
        };

        let norms = AnimalNorm {
            id: "cattle_dairy".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("energy_eke".to_string(), 9.8),
                ("crude_protein".to_string(), 1800.0),
                ("calcium".to_string(), 70.0),
                ("phosphorus".to_string(), 30.0),
                ("crude_fiber_pct".to_string(), 28.0),
            ]),
            nutrients_max: HashMap::from([("crude_fiber_pct".to_string(), 45.0)]),
            feed_intake_min: Some(12.0),
            feed_intake_max: Some(18.0),
            ..Default::default()
        };

        let repaired = optimize_with_library(
            &ration,
            OptimizationMode::RepairWithAdditions,
            Some(&norms),
            Some(&[hay, barley, soy, chalk, layer_premix]),
        )
        .unwrap();

        assert!(matches!(
            repaired.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));
        assert!(repaired.items.len() >= 3);
        assert!(repaired.items.iter().any(|item| item.feed_id == 2));
        assert!(repaired.items.iter().any(|item| item.feed_id == 3));
        assert!(repaired.items.iter().any(|item| item.feed_id == 4));
        assert!(!repaired.items.iter().any(|item| item.feed_id == 5));
        assert!(repaired.auto_added_feeds.len() >= 3);
        assert!(repaired
            .auto_added_feeds
            .iter()
            .all(|feed| feed.feed_id != 5));
    }

    #[test]
    fn test_repair_candidates_include_starter_template_feeds_for_empty_ration() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(9.1),
            crude_protein: Some(140.0),
            crude_fiber: Some(520.0),
            calcium: Some(8.0),
            phosphorus: Some(2.5),
            price_per_ton: Some(8000.0),
            ..Default::default()
        };
        let silage = Feed {
            id: Some(2),
            name_ru: "Corn silage".to_string(),
            category: "silage".to_string(),
            dry_matter: Some(32.0),
            energy_oe_cattle: Some(10.8),
            crude_protein: Some(85.0),
            crude_fiber: Some(380.0),
            calcium: Some(2.2),
            phosphorus: Some(2.4),
            price_per_ton: Some(6000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(3),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(12.4),
            crude_protein: Some(120.0),
            starch: Some(590.0),
            calcium: Some(0.6),
            phosphorus: Some(3.8),
            price_per_ton: Some(17000.0),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(4),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(89.0),
            crude_protein: Some(430.0),
            lysine: Some(28.0),
            methionine_cystine: Some(6.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            price_per_ton: Some(36000.0),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(5),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            calcium: Some(360.0),
            price_per_ton: Some(12000.0),
            ..Default::default()
        };
        let premix = Feed {
            id: Some(6),
            name_ru: "Premix P60 for cattle".to_string(),
            category: "premix".to_string(),
            carotene: Some(1_200_000.0),
            vit_d3: Some(250_000.0),
            vit_e: Some(2000.0),
            iodine: Some(8.0),
            price_per_ton: Some(70000.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Empty starter".to_string(),
                ..Default::default()
            },
            items: vec![],
        };

        let norms = AnimalNorm {
            id: "cattle_dairy".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("energy_eke".to_string(), 10.0),
                ("crude_protein".to_string(), 1800.0),
                ("calcium".to_string(), 70.0),
                ("phosphorus".to_string(), 28.0),
                ("crude_fiber_pct".to_string(), 28.0),
            ]),
            nutrients_max: HashMap::from([("crude_fiber_pct".to_string(), 45.0)]),
            feed_intake_min: Some(12.0),
            feed_intake_max: Some(18.0),
            ..Default::default()
        };

        let library = vec![hay, silage, barley, soy, chalk, premix];
        let screening = screening::screen_current_feed_set(&ration.items, &library, &norms);
        let candidates =
            repair_candidates_from_context(&ration, &norms, &screening.recommendations, &library);
        let candidate_ids = candidates
            .iter()
            .filter_map(|candidate| candidate.feed.id)
            .collect::<Vec<_>>();

        assert!(candidate_ids.contains(&1));
        assert!(candidate_ids.contains(&3));
        assert!(candidate_ids.contains(&4));
        assert!(candidate_ids.contains(&5));
    }

    #[test]
    fn test_repair_candidates_prioritize_role_fit_and_sow_stage_fit() {
        let barley = Feed {
            id: Some(1),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_pig: Some(13.0),
            crude_protein: Some(115.0),
            price_per_ton: Some(15000.0),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(2),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(88.0),
            crude_protein: Some(430.0),
            lysine: Some(28.0),
            price_per_ton: Some(34000.0),
            ..Default::default()
        };
        let starter_premix = Feed {
            id: Some(3),
            name_ru: "Премикс стартер для поросят".to_string(),
            category: "premix".to_string(),
            vit_e: Some(9000.0),
            price_per_ton: Some(68000.0),
            ..Default::default()
        };
        let sow_premix = Feed {
            id: Some(4),
            name_ru: "Премикс для свиноматок".to_string(),
            category: "premix".to_string(),
            vit_e: Some(8000.0),
            price_per_ton: Some(66000.0),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(5),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            calcium: Some(360.0),
            price_per_ton: Some(11000.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("swine_sow".to_string()),
                animal_count: 1,
                name: "Sow repair".to_string(),
                ..Default::default()
            },
            items: vec![make_item(1, barley.clone(), 2.8)],
        };

        let norms = AnimalNorm {
            id: "swine_sow_lactating".to_string(),
            species: "swine".to_string(),
            nutrients_min: HashMap::from([
                ("energy_oe_pig".to_string(), 12.5),
                ("crude_protein_pct".to_string(), 15.5),
                ("calcium".to_string(), 7.0),
                ("vit_e".to_string(), 1000.0),
            ]),
            ..Default::default()
        };

        let library = vec![barley, soy, starter_premix, sow_premix, chalk];
        let screening = screening::screen_current_feed_set(&ration.items, &library, &norms);
        let candidates =
            repair_candidates_from_context(&ration, &norms, &screening.recommendations, &library);
        let candidate_ids = candidates
            .iter()
            .filter_map(|candidate| candidate.feed.id)
            .collect::<Vec<_>>();

        let soy_index = candidate_ids.iter().position(|id| *id == 2).unwrap();
        let sow_index = candidate_ids.iter().position(|id| *id == 4).unwrap();

        assert!(
            soy_index < sow_index,
            "protein source should lead repair candidates"
        );
        assert!(
            !candidate_ids.contains(&3),
            "wrong-stage starter premix should drop out of the top repair candidates"
        );
    }

    #[test]
    fn test_library_constructor_builds_blank_cattle_ration() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(9.1),
            crude_protein: Some(140.0),
            crude_fiber: Some(520.0),
            calcium: Some(8.0),
            phosphorus: Some(2.5),
            price_per_ton: Some(8000.0),
            ..Default::default()
        };
        let silage = Feed {
            id: Some(2),
            name_ru: "Corn silage".to_string(),
            category: "silage".to_string(),
            dry_matter: Some(32.0),
            energy_oe_cattle: Some(10.8),
            crude_protein: Some(85.0),
            crude_fiber: Some(380.0),
            calcium: Some(2.2),
            phosphorus: Some(2.4),
            price_per_ton: Some(6000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(3),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(12.4),
            crude_protein: Some(120.0),
            starch: Some(590.0),
            calcium: Some(0.6),
            phosphorus: Some(3.8),
            price_per_ton: Some(17000.0),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(4),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(89.0),
            crude_protein: Some(430.0),
            lysine: Some(28.0),
            methionine_cystine: Some(6.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            price_per_ton: Some(36000.0),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(5),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            calcium: Some(360.0),
            phosphorus: Some(1.0),
            price_per_ton: Some(12000.0),
            ..Default::default()
        };
        let premix = Feed {
            id: Some(6),
            name_ru: "Premix P60 for cattle".to_string(),
            category: "premix".to_string(),
            carotene: Some(1_200_000.0),
            vit_d3: Some(250_000.0),
            vit_e: Some(2000.0),
            iodine: Some(8.0),
            price_per_ton: Some(70000.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Blank cattle".to_string(),
                ..Default::default()
            },
            items: vec![],
        };

        let norms = AnimalNorm {
            id: "cattle_dairy".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("energy_eke".to_string(), 9.5),
                ("crude_protein".to_string(), 1700.0),
                ("calcium".to_string(), 65.0),
                ("phosphorus".to_string(), 26.0),
                ("crude_fiber_pct".to_string(), 28.0),
            ]),
            nutrients_max: HashMap::from([("crude_fiber_pct".to_string(), 45.0)]),
            feed_intake_min: Some(12.0),
            feed_intake_max: Some(18.0),
            ..Default::default()
        };

        let solution = construct_ration_from_library(
            &ration,
            &norms,
            &[hay, silage, barley, soy, chalk, premix],
        )
        .unwrap()
        .expect("constructor should produce a blank ration");

        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));
        assert!(solution.items.len() >= 4);
        assert_eq!(solution.applied_strategy, "library_constructor");
        assert!(solution.auto_populated);
    }

    #[test]
    fn test_single_pass_balance_is_selectable() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(9.1),
            crude_protein: Some(140.0),
            crude_fiber: Some(520.0),
            calcium: Some(8.0),
            phosphorus: Some(2.5),
            price_per_ton: Some(8000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(2),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(13.2),
            crude_protein: Some(115.0),
            crude_fiber: Some(180.0),
            calcium: Some(0.6),
            phosphorus: Some(3.5),
            price_per_ton: Some(14000.0),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(3),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(12.9),
            crude_protein: Some(440.0),
            crude_fiber: Some(120.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            price_per_ton: Some(32000.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Test ration".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(1, hay, 10.0),
                make_item(2, barley, 7.0),
                make_item(3, soy, 0.8),
            ],
        };

        let mut min = HashMap::new();
        let mut target = HashMap::new();
        let mut max = HashMap::new();
        min.insert("energy_eke".to_string(), 14.0);
        target.insert("energy_eke".to_string(), 14.5);
        min.insert("crude_protein".to_string(), 2800.0);
        target.insert("crude_protein".to_string(), 3000.0);
        min.insert("crude_fiber_pct".to_string(), 28.0);
        max.insert("crude_fiber_pct".to_string(), 38.0);
        min.insert("calcium".to_string(), 90.0);
        target.insert("calcium".to_string(), 100.0);
        min.insert("phosphorus".to_string(), 55.0);
        target.insert("phosphorus".to_string(), 60.0);

        let norms = AnimalNorm {
            id: "cattle_dairy_test".to_string(),
            species: "cattle".to_string(),
            nutrients_min: min,
            nutrients_max: max,
            nutrients_target: target,
            feed_intake_min: Some(14.0),
            feed_intake_max: Some(18.0),
            ..Default::default()
        };

        let solution =
            optimize(&ration, OptimizationMode::SinglePassBalance, Some(&norms)).unwrap();
        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));
        assert_eq!(solution.applied_strategy, "single_pass_balance");
    }

    #[test]
    fn test_balance_alias_matches_tiered_mode() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(9.1),
            crude_protein: Some(140.0),
            crude_fiber: Some(520.0),
            calcium: Some(8.0),
            phosphorus: Some(2.5),
            price_per_ton: Some(8000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(2),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(13.2),
            crude_protein: Some(115.0),
            crude_fiber: Some(180.0),
            calcium: Some(0.6),
            phosphorus: Some(3.5),
            price_per_ton: Some(14000.0),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(3),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(12.9),
            crude_protein: Some(440.0),
            crude_fiber: Some(120.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            price_per_ton: Some(32000.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Test ration".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(1, hay, 10.0),
                make_item(2, barley, 7.0),
                make_item(3, soy, 0.8),
            ],
        };

        let mut min = HashMap::new();
        let mut target = HashMap::new();
        let mut max = HashMap::new();
        min.insert("energy_eke".to_string(), 14.0);
        target.insert("energy_eke".to_string(), 14.5);
        min.insert("crude_protein".to_string(), 2800.0);
        target.insert("crude_protein".to_string(), 3000.0);
        min.insert("crude_fiber_pct".to_string(), 28.0);
        max.insert("crude_fiber_pct".to_string(), 38.0);
        min.insert("calcium".to_string(), 90.0);
        target.insert("calcium".to_string(), 100.0);
        min.insert("phosphorus".to_string(), 55.0);
        target.insert("phosphorus".to_string(), 60.0);

        let norms = AnimalNorm {
            id: "cattle_dairy_test".to_string(),
            species: "cattle".to_string(),
            nutrients_min: min,
            nutrients_max: max,
            nutrients_target: target,
            feed_intake_min: Some(14.0),
            feed_intake_max: Some(18.0),
            ..Default::default()
        };

        let legacy = optimize(&ration, OptimizationMode::BalanceNutrients, Some(&norms)).unwrap();
        let tiered = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();

        assert_eq!(legacy.optimization_status, tiered.optimization_status);
        assert_eq!(legacy.applied_strategy, "priority_tiered_balance");
        assert_eq!(tiered.applied_strategy, "priority_tiered_balance");
        assert_eq!(legacy.items.len(), tiered.items.len());
        for (legacy_item, tiered_item) in legacy.items.iter().zip(tiered.items.iter()) {
            assert_eq!(legacy_item.feed_id, tiered_item.feed_id);
            assert!((legacy_item.amount_kg - tiered_item.amount_kg).abs() < 1e-6);
        }
    }

    #[test]
    fn test_actual_metric_value_uses_per_kg_for_swine_finisher_minerals() {
        let norms = AnimalNorm {
            id: "swine_finisher".to_string(),
            species: "swine".to_string(),
            ..Default::default()
        };
        let summary = nutrient_calc::NutrientSummary {
            total_weight_kg: 2.0,
            calcium: 14.0,
            phosphorus: 6.0,
            lysine: 15.0,
            methionine_cystine: 9.0,
            crude_protein: 320.0,
            ..Default::default()
        };

        assert_eq!(actual_metric_value(&norms, &summary, "calcium"), Some(7.0));
        assert_eq!(actual_metric_value(&norms, &summary, "phosphorus"), Some(3.0));
        assert_eq!(
            actual_metric_value(&norms, &summary, "lysine_sid"),
            Some(7.5)
        );
        assert_eq!(
            actual_metric_value(&norms, &summary, "methionine_cystine_sid"),
            Some(4.5)
        );
        assert_eq!(
            actual_metric_value(&norms, &summary, "crude_protein"),
            Some(160.0)
        );
    }

    #[test]
    fn test_actual_metric_value_uses_per_kg_for_poultry_energy() {
        let norms = AnimalNorm {
            id: "poultry_layer_peak".to_string(),
            species: "poultry".to_string(),
            ..Default::default()
        };
        let summary = nutrient_calc::NutrientSummary {
            total_weight_kg: 0.1,
            energy_oe_poultry: 1.13,
            ..Default::default()
        };

        assert!(
            (actual_metric_value(&norms, &summary, "energy_oe_poultry").unwrap() - 11.3).abs()
                < 1e-6
        );
    }

    #[test]
    fn test_minimize_cost_keeps_poultry_energy_hard_constraint_with_dm_based_feed_values() {
        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("poultry_broiler".to_string()),
                animal_count: 1,
                name: "Broiler starter benchmark".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(
                    1,
                    Feed {
                        id: Some(1),
                        name_ru: "Кукуруза".to_string(),
                        category: "grain".to_string(),
                        dry_matter: Some(88.0),
                        energy_oe_poultry: Some(15.0),
                        crude_protein: Some(90.0),
                        calcium: Some(0.3),
                        phosphorus: Some(1.2),
                        price_per_ton: Some(0.0),
                        ..Default::default()
                    },
                    0.012,
                ),
                make_item(
                    2,
                    Feed {
                        id: Some(2),
                        name_ru: "Шрот соевый".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(90.0),
                        energy_oe_poultry: Some(16.1),
                        crude_protein: Some(439.0),
                        lysine: Some(27.7),
                        methionine_cystine: Some(11.9),
                        calcium: Some(2.7),
                        phosphorus: Some(2.5),
                        price_per_ton: Some(0.0),
                        ..Default::default()
                    },
                    0.009,
                ),
                make_item(
                    3,
                    Feed {
                        id: Some(3),
                        name_ru: "Мука рыбная".to_string(),
                        category: "animal_origin".to_string(),
                        dry_matter: Some(90.0),
                        energy_oe_poultry: Some(12.0),
                        crude_protein: Some(600.0),
                        lysine: Some(45.0),
                        methionine_cystine: Some(20.0),
                        calcium: Some(50.0),
                        phosphorus: Some(20.0),
                        price_per_ton: Some(0.0),
                        ..Default::default()
                    },
                    0.0015,
                ),
                make_item(
                    4,
                    Feed {
                        id: Some(4),
                        name_ru: "Мел кормовой".to_string(),
                        category: "mineral".to_string(),
                        dry_matter: Some(99.0),
                        calcium: Some(380.0),
                        price_per_ton: Some(0.0),
                        ..Default::default()
                    },
                    0.0004,
                ),
                make_item(
                    5,
                    Feed {
                        id: Some(5),
                        name_ru: "Монокальций фосфат".to_string(),
                        category: "mineral".to_string(),
                        dry_matter: Some(98.0),
                        calcium: Some(170.0),
                        phosphorus: Some(220.0),
                        price_per_ton: Some(0.0),
                        ..Default::default()
                    },
                    0.0003,
                ),
            ],
        };

        let norms = AnimalNorm {
            id: "poultry_broiler_starter".to_string(),
            species: "poultry".to_string(),
            nutrients_min: HashMap::from([
                ("energy_oe_poultry".to_string(), 12.5),
                ("crude_protein_pct".to_string(), 22.7),
                ("calcium_pct".to_string(), 1.01),
                ("phosphorus_pct".to_string(), 0.48),
            ]),
            nutrients_target: HashMap::from([
                ("energy_oe_poultry".to_string(), 12.6),
                ("crude_protein_pct".to_string(), 23.2),
                ("lysine_tid_pct".to_string(), 1.35),
                ("methionine_cystine_tid_pct".to_string(), 1.0),
            ]),
            ..Default::default()
        };

        let solution = optimize(&ration, OptimizationMode::MinimizeCost, Some(&norms)).unwrap();

        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "energy_oe_poultry").unwrap()
                >= 12.5 - 1e-6,
            "energy should remain within hard poultry minimum"
        );
    }

    #[test]
    fn test_should_keep_current_solution_when_best_achievable_loses_hard_energy_without_core_gain()
    {
        let norms = AnimalNorm {
            id: "cattle_dairy_fresh".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([("energy_eke".to_string(), 24.2)]),
            feed_intake_min: Some(19.0),
            feed_intake_max: Some(23.0),
            ..Default::default()
        };
        let current = nutrient_calc::NutrientSummary {
            energy_eke: 24.6,
            total_dm_kg: 24.8,
            ..Default::default()
        };
        let candidate = nutrient_calc::NutrientSummary {
            energy_eke: 19.5,
            total_dm_kg: 19.8,
            ..Default::default()
        };

        assert!(should_keep_current_solution(&norms, &current, &candidate));
    }

    #[test]
    fn test_actual_metric_value_supports_amino_ratio_metrics() {
        let norms = AnimalNorm {
            id: "swine_grower".to_string(),
            species: "swine".to_string(),
            ..Default::default()
        };
        let summary = nutrient_calc::NutrientSummary {
            lysine: 20.0,
            methionine_cystine: 12.0,
            ..Default::default()
        };

        let metcys_ratio =
            actual_metric_value(&norms, &summary, "methionine_cystine_lys_ratio").unwrap();
        assert!((metcys_ratio - 0.60).abs() < 1e-6);
    }

    #[test]
    fn test_actual_metric_value_supports_only_supported_ratio_numerator() {
        let norms = AnimalNorm {
            id: "poultry_broiler_starter".to_string(),
            species: "poultry".to_string(),
            ..Default::default()
        };
        let summary = nutrient_calc::NutrientSummary {
            lysine: 18.0,
            methionine_cystine: 9.0,
            ..Default::default()
        };

        assert_eq!(
            actual_metric_value(&norms, &summary, "methionine_cystine_lys_ratio"),
            Some(0.5)
        );
    }

    #[test]
    fn test_actual_metric_value_supports_cattle_digestible_protein_metrics() {
        let norms = AnimalNorm {
            id: "cattle_dairy_early_lact".to_string(),
            species: "cattle".to_string(),
            ..Default::default()
        };
        let summary = nutrient_calc::NutrientSummary {
            crude_protein: 3200.0,
            dig_protein_cattle: 2200.0,
            dig_protein_cattle_pct_cp: 68.75,
            ..Default::default()
        };

        assert_eq!(
            actual_metric_value(&norms, &summary, "dig_protein_cattle"),
            Some(2200.0)
        );
        assert_eq!(
            actual_metric_value(&norms, &summary, "dig_protein_cattle_pct_cp"),
            Some(68.75)
        );
    }

    #[test]
    fn test_swine_grower_uses_percentage_lysine_tier_keys() {
        let grower = crate::norms::get_norms_for_group("swine_grower").unwrap();
        let finisher = crate::norms::get_norms_for_group("swine_finisher").unwrap();

        let grower_tier1 = tier_keys(&grower)[0]
            .clone()
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        let finisher_tier1 = tier_keys(&finisher)[0]
            .clone()
            .into_iter()
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(
            grower_tier1,
            std::collections::HashSet::from([
                "energy_oe_pig",
                "lysine_sid_pct",
                "crude_protein_pct"
            ])
        );
        assert_eq!(
            finisher_tier1,
            std::collections::HashSet::from(["energy_oe_pig", "lysine_sid", "crude_protein"])
        );
    }

    #[test]
    fn test_swine_grower_ratio_targets_improve_amino_profile() {
        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("swine_grower".to_string()),
                animal_count: 1,
                name: "Swine grower amino profile".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(
                    1,
                    Feed {
                        id: Some(1),
                        name_ru: "Corn".to_string(),
                        category: "grain".to_string(),
                        dry_matter: Some(88.0),
                        energy_oe_pig: Some(14.0),
                        crude_protein: Some(90.0),
                        lysine: Some(2.0),
                        methionine_cystine: Some(1.5),
                        price_per_ton: Some(12000.0),
                        ..Default::default()
                    },
                    1.60,
                ),
                make_item(
                    2,
                    Feed {
                        id: Some(2),
                        name_ru: "Soybean meal".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(88.0),
                        energy_oe_pig: Some(12.7),
                        crude_protein: Some(430.0),
                        lysine: Some(30.0),
                        methionine_cystine: Some(12.0),
                        price_per_ton: Some(30000.0),
                        ..Default::default()
                    },
                    0.35,
                ),
                make_item(
                    3,
                    Feed {
                        id: Some(3),
                        name_ru: "Fish meal".to_string(),
                        category: "animal_meal".to_string(),
                        dry_matter: Some(92.0),
                        energy_oe_pig: Some(12.0),
                        crude_protein: Some(620.0),
                        lysine: Some(25.0),
                        methionine_cystine: Some(25.0),
                        price_per_ton: Some(52000.0),
                        ..Default::default()
                    },
                    0.05,
                ),
            ],
        };

        let norms = AnimalNorm {
            id: "swine_grower".to_string(),
            species: "swine".to_string(),
            nutrients_min: HashMap::from([("energy_oe_pig".to_string(), 22.0)]),
            nutrients_target: HashMap::from([
                ("lysine_sid_pct".to_string(), 1.0),
                ("methionine_cystine_lys_ratio".to_string(), 0.40),
            ]),
            feed_intake_min: Some(1.9),
            feed_intake_max: Some(2.1),
            ..Default::default()
        };
        let initial_summary = summary_for_amounts(
            &ration.items,
            &ration
                .items
                .iter()
                .map(|item| item.amount_kg)
                .collect::<Vec<_>>(),
        );

        let solution = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();

        assert_eq!(solution.optimization_status, SolutionStatus::Optimal);
        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "lysine_sid_pct").unwrap()
                >= 1.0
        );
        let initial_methionine_ratio = actual_metric_value(
            &norms,
            &initial_summary,
            "methionine_cystine_lys_ratio",
        )
        .unwrap();
        let optimized_methionine_ratio = actual_metric_value(
            &norms,
            &solution.nutrient_summary,
            "methionine_cystine_lys_ratio",
        )
        .unwrap();
        assert!(optimized_methionine_ratio > initial_methionine_ratio);

        let fish_meal_amount = solution
            .items
            .iter()
            .find(|item| item.feed_id == 3)
            .map(|item| item.amount_kg)
            .unwrap_or(0.0);
        assert!(
            fish_meal_amount > 0.10,
            "fish meal amount was {fish_meal_amount:.3}"
        );
    }

    #[test]
    fn test_swine_finisher_optimization_respects_per_kg_mineral_norms() {
        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("swine_finisher".to_string()),
                animal_count: 1,
                name: "Swine finisher".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(
                    1,
                    Feed {
                        id: Some(1),
                        name_ru: "Barley".to_string(),
                        category: "grain".to_string(),
                        dry_matter: Some(86.0),
                        energy_oe_pig: Some(13.5),
                        crude_protein: Some(115.0),
                        lysine: Some(4.2),
                        methionine_cystine: Some(3.8),
                        calcium: Some(0.6),
                        phosphorus: Some(3.5),
                        price_per_ton: Some(14000.0),
                        ..Default::default()
                    },
                    2.2,
                ),
                make_item(
                    2,
                    Feed {
                        id: Some(2),
                        name_ru: "Soybean meal".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(88.0),
                        energy_oe_pig: Some(12.7),
                        crude_protein: Some(440.0),
                        lysine: Some(28.0),
                        methionine_cystine: Some(12.0),
                        calcium: Some(3.0),
                        phosphorus: Some(6.5),
                        price_per_ton: Some(32000.0),
                        ..Default::default()
                    },
                    0.35,
                ),
                make_item(
                    3,
                    Feed {
                        id: Some(3),
                        name_ru: "Feed chalk".to_string(),
                        category: "mineral".to_string(),
                        calcium: Some(360.0),
                        price_per_ton: Some(9000.0),
                        ..Default::default()
                    },
                    0.01,
                ),
            ],
        };

        let norms = crate::norms::get_norms_for_group("swine_finisher").unwrap();
        let solution = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();

        assert_eq!(solution.optimization_status, SolutionStatus::Optimal);
        assert!(actual_metric_value(&norms, &solution.nutrient_summary, "calcium").unwrap() >= 6.5);
        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "phosphorus").unwrap()
                >= 2.5
        );
        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "lysine_sid").unwrap()
                >= 7.5 - 1e-6
        );
    }

    #[test]
    fn test_swine_ratio_goal_increases_methionine_cystine_balance() {
        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("swine_finisher".to_string()),
                animal_count: 1,
                name: "Swine amino ratio".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(
                    1,
                    Feed {
                        id: Some(1),
                        name_ru: "Corn".to_string(),
                        category: "grain".to_string(),
                        dry_matter: Some(87.0),
                        energy_oe_pig: Some(14.2),
                        crude_protein: Some(90.0),
                        lysine: Some(2.0),
                        methionine_cystine: Some(1.6),
                        price_per_ton: Some(12000.0),
                        ..Default::default()
                    },
                    2.35,
                ),
                make_item(
                    2,
                    Feed {
                        id: Some(2),
                        name_ru: "Soybean meal".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(88.0),
                        energy_oe_pig: Some(12.8),
                        crude_protein: Some(440.0),
                        lysine: Some(28.0),
                        methionine_cystine: Some(12.0),
                        price_per_ton: Some(28000.0),
                        ..Default::default()
                    },
                    0.42,
                ),
                make_item(
                    3,
                    Feed {
                        id: Some(3),
                        name_ru: "Fish meal".to_string(),
                        category: "animal_meal".to_string(),
                        dry_matter: Some(92.0),
                        energy_oe_pig: Some(11.8),
                        crude_protein: Some(620.0),
                        lysine: Some(35.0),
                        methionine_cystine: Some(25.0),
                        price_per_ton: Some(52000.0),
                        ..Default::default()
                    },
                    0.03,
                ),
            ],
        };

        let norms = AnimalNorm {
            id: "swine_finisher".to_string(),
            species: "swine".to_string(),
            nutrients_min: HashMap::from([
                ("energy_oe_pig".to_string(), 30.0),
                ("lysine_sid".to_string(), 6.4),
                ("methionine_cystine_sid".to_string(), 3.4),
            ]),
            nutrients_target: HashMap::from([
                ("energy_oe_pig".to_string(), 31.0),
                ("lysine_sid".to_string(), 6.6),
                ("methionine_cystine_sid".to_string(), 3.5),
                ("methionine_cystine_lys_ratio".to_string(), 0.60),
            ]),
            feed_intake_min: Some(2.6),
            feed_intake_max: Some(3.0),
            ..Default::default()
        };

        let initial_ratio = actual_metric_value(
            &norms,
            &summary_for_amounts(
                &ration.items,
                &ration
                    .items
                    .iter()
                    .map(|item| item.amount_kg)
                    .collect::<Vec<_>>(),
            ),
            "methionine_cystine_lys_ratio",
        )
        .unwrap();

        let solution = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();

        assert_eq!(solution.optimization_status, SolutionStatus::Optimal);
        let optimized_ratio = actual_metric_value(
            &norms,
            &solution.nutrient_summary,
            "methionine_cystine_lys_ratio",
        )
        .unwrap();
        let fish_meal_amount = solution
            .items
            .iter()
            .find(|item| item.feed_id == 3)
            .map(|item| item.amount_kg)
            .unwrap_or(0.0);

        assert!(optimized_ratio > initial_ratio + 0.015);
        assert!(
            optimized_ratio >= 0.57,
            "optimized ratio was {optimized_ratio:.3}"
        );
        assert!(
            fish_meal_amount > 0.05,
            "fish meal amount was {fish_meal_amount:.3}"
        );
    }

    #[test]
    fn test_dairy_35_preset_like_ration_improves_core_alignment_without_protein_collapse() {
        let silage = Feed {
            id: Some(1),
            name_ru: "Силос кукурузный".to_string(),
            category: "silage".to_string(),
            dry_matter: Some(35.0),
            energy_oe_cattle: Some(10.8),
            crude_protein: Some(85.0),
            dig_protein_cattle: Some(55.0),
            crude_fiber: Some(170.0),
            calcium: Some(2.2),
            phosphorus: Some(2.5),
            price_per_ton: Some(5000.0),
            ..Default::default()
        };
        let haylage = Feed {
            id: Some(2),
            name_ru: "Сенаж люцерновый".to_string(),
            category: "haylage".to_string(),
            dry_matter: Some(45.0),
            energy_oe_cattle: Some(10.2),
            crude_protein: Some(180.0),
            dig_protein_cattle: Some(120.0),
            crude_fiber: Some(230.0),
            calcium: Some(15.0),
            phosphorus: Some(2.8),
            price_per_ton: Some(7800.0),
            ..Default::default()
        };
        let hay = Feed {
            id: Some(3),
            name_ru: "Сено луговое".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(8.7),
            crude_protein: Some(110.0),
            dig_protein_cattle: Some(65.0),
            crude_fiber: Some(420.0),
            calcium: Some(5.5),
            phosphorus: Some(2.2),
            price_per_ton: Some(9000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(4),
            name_ru: "Ячмень дробленый".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(13.2),
            crude_protein: Some(115.0),
            dig_protein_cattle: Some(82.0),
            crude_fiber: Some(110.0),
            starch: Some(540.0),
            calcium: Some(0.6),
            phosphorus: Some(3.6),
            price_per_ton: Some(16000.0),
            ..Default::default()
        };
        let corn = Feed {
            id: Some(5),
            name_ru: "Кукуруза зерно".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(14.3),
            crude_protein: Some(90.0),
            dig_protein_cattle: Some(70.0),
            crude_fiber: Some(70.0),
            starch: Some(650.0),
            calcium: Some(0.4),
            phosphorus: Some(2.8),
            price_per_ton: Some(17500.0),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(6),
            name_ru: "Шрот соевый".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(89.0),
            energy_oe_cattle: Some(12.8),
            crude_protein: Some(440.0),
            dig_protein_cattle: Some(330.0),
            crude_fiber: Some(90.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            price_per_ton: Some(36000.0),
            ..Default::default()
        };
        let sunflower = Feed {
            id: Some(7),
            name_ru: "Шрот подсолнечный".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(90.0),
            energy_oe_cattle: Some(11.0),
            crude_protein: Some(360.0),
            dig_protein_cattle: Some(250.0),
            crude_fiber: Some(160.0),
            calcium: Some(3.6),
            phosphorus: Some(9.0),
            price_per_ton: Some(24000.0),
            ..Default::default()
        };
        let beet_pulp = Feed {
            id: Some(8),
            name_ru: "Жом свекловичный сухой".to_string(),
            category: "fiber_byproduct".to_string(),
            dry_matter: Some(90.0),
            energy_oe_cattle: Some(11.5),
            crude_protein: Some(95.0),
            dig_protein_cattle: Some(65.0),
            crude_fiber: Some(200.0),
            calcium: Some(7.0),
            phosphorus: Some(1.0),
            price_per_ton: Some(19000.0),
            ..Default::default()
        };
        let bran = Feed {
            id: Some(9),
            name_ru: "Отруби пшеничные".to_string(),
            category: "bran".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(10.5),
            crude_protein: Some(160.0),
            dig_protein_cattle: Some(110.0),
            crude_fiber: Some(180.0),
            phosphorus: Some(10.0),
            price_per_ton: Some(13500.0),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(10),
            name_ru: "Мел кормовой".to_string(),
            category: "mineral".to_string(),
            dry_matter: Some(99.0),
            calcium: Some(380.0),
            price_per_ton: Some(10500.0),
            ..Default::default()
        };
        let premix = Feed {
            id: Some(11),
            name_ru: "Премикс П60-1".to_string(),
            category: "premix".to_string(),
            subcategory: Some("cattle_premix".to_string()),
            dry_matter: Some(95.0),
            calcium: Some(120.0),
            phosphorus: Some(40.0),
            iodine: Some(25.0),
            carotene: Some(800_000.0),
            vit_d3: Some(250_000.0),
            vit_e: Some(5_000.0),
            price_per_ton: Some(72000.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Dairy 35 preset-like".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(1, silage, 20.0),
                make_item(2, haylage, 5.0),
                make_item(3, hay, 2.0),
                make_item(4, barley, 3.0),
                make_item(5, corn, 2.5),
                make_item(6, soy, 2.0),
                make_item(7, sunflower, 1.5),
                make_item(8, beet_pulp, 1.5),
                make_item(9, bran, 0.8),
                make_item(10, chalk, 0.12),
                make_item(11, premix, 0.15),
            ],
        };

        let req = NormResolveRequest {
            norm_preset_id: Some("cattle_dairy_35".to_string()),
            animal_properties: Some(ResolveAnimalProperties {
                species: Some("cattle".to_string()),
                production_type: Some("dairy".to_string()),
                breed: Some("Holstein".to_string()),
                sex: Some("female".to_string()),
                live_weight_kg: Some(640.0),
                milk_yield_kg: Some(35.0),
                milk_fat_pct: Some(3.7),
                ..Default::default()
            }),
        };

        let resolved_group = resolved_norm_group_id(Some("cattle_dairy"), &req);
        assert_eq!(resolved_group, "cattle_dairy_fresh");
        let norms =
            resolved_default_norm(Some("cattle_dairy"), &req, Some(&resolved_group)).unwrap();

        let initial_summary = summary_for_amounts(
            &ration.items,
            &ration
                .items
                .iter()
                .map(|item| item.amount_kg)
                .collect::<Vec<_>>(),
        );
        let core_keys = [
            "energy_eke",
            "crude_protein",
            "dig_protein_cattle",
            "crude_fiber_pct",
            "calcium",
            "phosphorus",
        ];
        let initial_score = core_alignment_score(&norms, &initial_summary, &core_keys);

        let solution = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();

        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));

        let optimized_score = core_alignment_score(&norms, &solution.nutrient_summary, &core_keys);
        assert!(
            optimized_score <= initial_score,
            "optimized score {optimized_score:.3} should not exceed initial score {initial_score:.3}"
        );
        assert!(
            normalized_metric_gap(&norms, &solution.nutrient_summary, "energy_eke")
                <= normalized_metric_gap(&norms, &initial_summary, "energy_eke") + 0.05
        );
        assert!(
            normalized_metric_gap(&norms, &solution.nutrient_summary, "dig_protein_cattle")
                <= normalized_metric_gap(&norms, &initial_summary, "dig_protein_cattle") + 0.05
        );
        assert!(
            normalized_metric_gap(&norms, &solution.nutrient_summary, "crude_fiber_pct")
                <= normalized_metric_gap(&norms, &initial_summary, "crude_fiber_pct") + 0.05
        );
        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "dig_protein_cattle").unwrap()
                >= *norms.nutrients_min.get("dig_protein_cattle").unwrap()
        );
        assert!(
            intake_gap(&norms, &solution.nutrient_summary)
                <= intake_gap(&norms, &initial_summary) + 0.10,
            "optimized intake gap {:.3} should stay close to initial {:.3}",
            intake_gap(&norms, &solution.nutrient_summary),
            intake_gap(&norms, &initial_summary)
        );
    }

    #[test]
    fn test_dynamic_beef_stocker_ration_keeps_digestible_protein_and_intake_aligned() {
        let silage = Feed {
            id: Some(1),
            name_ru: "Силос кукурузный".to_string(),
            category: "silage".to_string(),
            dry_matter: Some(35.0),
            energy_oe_cattle: Some(10.8),
            crude_protein: Some(85.0),
            dig_protein_cattle: Some(55.0),
            crude_fiber: Some(180.0),
            calcium: Some(2.2),
            phosphorus: Some(2.5),
            price_per_ton: Some(5000.0),
            ..Default::default()
        };
        let hay = Feed {
            id: Some(2),
            name_ru: "Сено луговое".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(8.6),
            crude_protein: Some(110.0),
            dig_protein_cattle: Some(65.0),
            crude_fiber: Some(420.0),
            calcium: Some(5.5),
            phosphorus: Some(2.2),
            price_per_ton: Some(9000.0),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(3),
            name_ru: "Ячмень дробленый".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(13.1),
            crude_protein: Some(115.0),
            dig_protein_cattle: Some(82.0),
            crude_fiber: Some(110.0),
            calcium: Some(0.6),
            phosphorus: Some(3.6),
            price_per_ton: Some(16000.0),
            ..Default::default()
        };
        let soybean_meal = Feed {
            id: Some(4),
            name_ru: "Шрот соевый".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(89.0),
            energy_oe_cattle: Some(12.8),
            crude_protein: Some(440.0),
            dig_protein_cattle: Some(330.0),
            crude_fiber: Some(90.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            price_per_ton: Some(36000.0),
            ..Default::default()
        };
        let sunflower_meal = Feed {
            id: Some(5),
            name_ru: "Шрот подсолнечный".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(90.0),
            energy_oe_cattle: Some(11.0),
            crude_protein: Some(360.0),
            dig_protein_cattle: Some(250.0),
            crude_fiber: Some(160.0),
            calcium: Some(3.6),
            phosphorus: Some(9.0),
            price_per_ton: Some(24000.0),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(6),
            name_ru: "Мел кормовой".to_string(),
            category: "mineral".to_string(),
            dry_matter: Some(99.0),
            calcium: Some(380.0),
            price_per_ton: Some(10500.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: crate::db::rations::Ration {
                id: Some(1),
                animal_group_id: Some("cattle_beef".to_string()),
                animal_count: 1,
                name: "Beef stocker".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(1, silage, 8.0),
                make_item(2, hay, 2.0),
                make_item(3, barley, 2.0),
                make_item(4, soybean_meal, 0.7),
                make_item(5, sunflower_meal, 0.5),
                make_item(6, chalk, 0.04),
            ],
        };

        let req = NormResolveRequest {
            norm_preset_id: None,
            animal_properties: Some(ResolveAnimalProperties {
                species: Some("cattle".to_string()),
                production_type: Some("beef".to_string()),
                breed: Some("Aberdeen Angus".to_string()),
                sex: Some("male".to_string()),
                live_weight_kg: Some(280.0),
                daily_gain_g: Some(850.0),
                ..Default::default()
            }),
        };

        let resolved_group = resolved_norm_group_id(Some("cattle_beef"), &req);
        assert_eq!(resolved_group, "cattle_beef_stocker");
        let norms =
            resolved_default_norm(Some("cattle_beef"), &req, Some(&resolved_group)).unwrap();

        let initial_summary = summary_for_amounts(
            &ration.items,
            &ration
                .items
                .iter()
                .map(|item| item.amount_kg)
                .collect::<Vec<_>>(),
        );
        let core_keys = [
            "energy_eke",
            "crude_protein",
            "dig_protein_cattle",
            "calcium",
            "phosphorus",
        ];
        let initial_score = core_alignment_score(&norms, &initial_summary, &core_keys);

        let solution = optimize(&ration, OptimizationMode::TieredBalance, Some(&norms)).unwrap();

        assert_eq!(solution.optimization_status, SolutionStatus::Optimal);
        assert!(!solution.best_achievable);

        let optimized_score = core_alignment_score(&norms, &solution.nutrient_summary, &core_keys);
        assert!(
            optimized_score <= initial_score + 0.02,
            "optimized score {optimized_score:.3} should stay at or below initial score {initial_score:.3}"
        );
        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "dig_protein_cattle").unwrap()
                >= *norms.nutrients_min.get("dig_protein_cattle").unwrap()
        );
        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "energy_eke").unwrap()
                >= *norms.nutrients_min.get("energy_eke").unwrap()
        );
        assert!(
            solution.nutrient_summary.total_dm_kg >= norms.feed_intake_min.unwrap() - 0.25
                && solution.nutrient_summary.total_dm_kg <= norms.feed_intake_max.unwrap() + 0.25,
            "optimized DM intake {:.2} should remain close to [{:.2}, {:.2}]",
            solution.nutrient_summary.total_dm_kg,
            norms.feed_intake_min.unwrap(),
            norms.feed_intake_max.unwrap()
        );
    }

    #[test]
    fn test_tier_keys_cover_audit_nutrients_for_cattle() {
        let norms = AnimalNorm {
            id: "cattle_dairy_test".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("crude_fiber".to_string(), 2500.0),
                ("magnesium".to_string(), 28.0),
                ("sodium".to_string(), 20.0),
                ("sulfur".to_string(), 24.0),
                ("vit_d3".to_string(), 70000.0),
            ]),
            ..Default::default()
        };

        let [tier1, tier2, tier3] = tier_keys(&norms);

        assert!(tier1.contains(&"crude_fiber"));
        assert!(tier2.contains(&"magnesium"));
        assert!(tier2.contains(&"sodium"));
        assert!(tier3.contains(&"sulfur"));
        assert!(tier3.contains(&"vit_d3"));
    }

    #[test]
    fn test_optimizer_supports_intake_and_missing_audit_mineral_keys() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            dry_matter: Some(88.0),
            energy_oe_cattle: Some(8.8),
            crude_protein: Some(120.0),
            crude_fiber: Some(320.0),
            magnesium: Some(2.6),
            sodium: Some(0.5),
            sulfur: Some(0.4),
            ..Default::default()
        };
        let silage = Feed {
            id: Some(2),
            name_ru: "Corn silage".to_string(),
            category: "silage".to_string(),
            dry_matter: Some(32.0),
            energy_oe_cattle: Some(10.8),
            crude_protein: Some(78.0),
            crude_fiber: Some(210.0),
            magnesium: Some(1.8),
            sodium: Some(0.4),
            sulfur: Some(0.3),
            ..Default::default()
        };
        let mineral = Feed {
            id: Some(3),
            name_ru: "Mineral mix".to_string(),
            category: "mineral".to_string(),
            dry_matter: Some(98.0),
            magnesium: Some(35.0),
            sodium: Some(45.0),
            sulfur: Some(18.0),
            ..Default::default()
        };

        let items = vec![
            make_item(1, hay, 7.0),
            make_item(2, silage, 10.0),
            make_item(3, mineral, 0.12),
        ];

        let norms = AnimalNorm {
            id: "cattle_dairy_test".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("dry_matter_intake".to_string(), 8.0),
                ("energy_eke".to_string(), 9.0),
                ("crude_fiber".to_string(), 2500.0),
            ]),
            nutrients_target: HashMap::from([
                ("magnesium".to_string(), 24.0),
                ("sodium".to_string(), 6.0),
                ("sulfur".to_string(), 4.0),
            ]),
            ..Default::default()
        };

        let solution = optimize_balance_nutrients(
            &items,
            Some(&norms),
            MovementPolicy {
                change_fraction: 0.60,
                change_floor_kg: 2.0,
            },
            BalanceApproach::Tiered,
            "test",
        )
        .unwrap();

        assert_eq!(solution.optimization_status, SolutionStatus::Optimal);
        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "dry_matter_intake").is_some()
        );
        assert!(actual_metric_value(&norms, &solution.nutrient_summary, "magnesium").is_some());
        assert!(actual_metric_value(&norms, &solution.nutrient_summary, "sodium").is_some());
        assert!(actual_metric_value(&norms, &solution.nutrient_summary, "sulfur").is_some());
        assert!(
            actual_metric_value(&norms, &solution.nutrient_summary, "dry_matter_intake").unwrap()
                >= 8.0
        );
    }
}
