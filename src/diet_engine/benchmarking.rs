use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::Instant;

use anyhow::Result;
use serde::Serialize;

use crate::db::{
    feeds::Feed,
    rations::{Ration, RationFull, RationItem},
};
use crate::norms::{self, factorial::ConstraintTier, AnimalNorm};

use super::auto_populate::{build_auto_populate_plan, plan_to_ration_items, AutoPopulatePlan};
use super::feed_groups::{
    assess_feed_suitability, classify_feed, group_label, is_feed_allowed_for_context,
    required_groups_for_species, validate_group_coverage, FeedGroup, FeedSuitabilityStatus,
};
use super::nutrient_calc::NutrientSummary;
use super::optimizer::{
    actual_metric_value, constraint_tier_for_key, feed_has_metric_signal, DietSolution,
    OptimizedItem, SolutionStatus,
};
use super::screening::screen_current_feed_set;
use super::{construct_ration_from_library, optimize_ration_with_library, OptimizationMode};

const TARGET_ALIGNMENT_WEIGHT: f64 = 0.25;

#[derive(Debug, Clone)]
pub struct BenchmarkPriceInfo {
    pub kind: String,
    pub is_precise_source: bool,
    pub benchmark_level: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkCase {
    pub id: String,
    pub label: String,
    pub species: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkRun {
    pub methodology_version: String,
    pub source_feed_count: usize,
    pub priced_feed_count: usize,
    pub direct_price_anchor_count: usize,
    pub benchmark_priced_feed_count: usize,
    pub cases: Vec<BenchmarkCaseResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkCaseResult {
    pub case: BenchmarkCase,
    pub library: LibraryDiagnostics,
    pub sparse_seed: Option<SparseSeedAssessment>,
    pub workflows: Vec<WorkflowAssessment>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryDiagnostics {
    pub allowed_feed_count: usize,
    pub required_groups: Vec<String>,
    pub allowed_groups: Vec<String>,
    pub missing_required_groups: Vec<String>,
    pub metric_support_counts: BTreeMap<String, usize>,
    pub direct_price_feed_count: usize,
    pub benchmark_price_feed_count: usize,
    pub unpriced_feed_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SparseSeedAssessment {
    pub feed_count: usize,
    pub total_amount_kg: f64,
    pub retained_mass_share_pct: f64,
    pub category_shares_pct: BTreeMap<String, f64>,
    pub feeds: Vec<BenchFeedAmount>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowAssessment {
    pub intent: String,
    pub entry_state: String,
    pub optimization_status: String,
    pub applied_strategy: String,
    pub runtime_ms: f64,
    pub auto_populated: bool,
    pub best_achievable: bool,
    pub feed_count: usize,
    pub total_amount_kg: f64,
    pub category_richness: usize,
    pub category_shares_pct: BTreeMap<String, f64>,
    pub suitability: SuitabilitySummary,
    pub required_group_coverage: RequiredGroupCoverage,
    pub cost_per_day_rub: f64,
    pub rub_per_kg_as_fed: f64,
    pub price_coverage_pct: f64,
    pub direct_price_share_pct: f64,
    pub benchmark_price_share_pct: f64,
    pub price_confidence_score: f64,
    pub evaluable_constraints: usize,
    pub unevaluable_constraints: Vec<String>,
    pub hard_pass_rate: f64,
    pub tier1_pass_rate: f64,
    pub tier2_pass_rate: f64,
    pub tier3_pass_rate: f64,
    pub norm_coverage_index: f64,
    pub deficiency_index: f64,
    pub excess_index: f64,
    pub target_gap_index: f64,
    pub constraint_violation_index: f64,
    pub auxiliary_composite_index: f64,
    pub post_screening: ScreeningAssessment,
    pub top_issues: Vec<ConstraintAssessment>,
    pub feeds: Vec<BenchFeedAmount>,
    pub workflow_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuitabilitySummary {
    pub appropriate_share_pct: f64,
    pub conditional_share_pct: f64,
    pub restricted_share_pct: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequiredGroupCoverage {
    pub covered_groups: Vec<String>,
    pub missing_groups: Vec<String>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreeningAssessment {
    pub can_meet_reference: bool,
    pub limiting_nutrients: Vec<String>,
    pub recommendations: Vec<BenchRecommendation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchRecommendation {
    pub feed_id: i64,
    pub feed_name: String,
    pub reason: String,
    pub suggested_amount_kg: f64,
    pub category: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConstraintAssessment {
    pub key: String,
    pub tier: u8,
    pub actual: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub target: Option<f64>,
    pub relative_deficit: f64,
    pub relative_excess: f64,
    pub relative_target_gap: f64,
    pub support_feed_count: usize,
    pub hard_pass: bool,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchFeedAmount {
    pub feed_id: i64,
    pub feed_name: String,
    pub amount_kg: f64,
    pub cost_per_day: f64,
    pub price_provenance_kind: String,
}

#[derive(Debug, Clone)]
struct SparseCandidate {
    feed_id: i64,
    feed_name: String,
    amount_kg: f64,
    cost_per_day: f64,
    group: FeedGroup,
    feed: Feed,
}

#[derive(Debug, Clone)]
struct SparseSeedInternal {
    ration: RationFull,
    assessment: SparseSeedAssessment,
}

pub fn default_benchmark_cases() -> Vec<BenchmarkCase> {
    vec![
        case("cattle_dairy_fresh", "Fresh dairy cows", "cattle"),
        case("cattle_dairy_early_lact", "Early-lactation dairy cows", "cattle"),
        case("cattle_dairy_dry_early", "Dry cows, early dry period", "cattle"),
        case("cattle_dairy_heifer_12_18", "Dairy heifers, 12-18 months", "cattle"),
        case("cattle_beef_stocker", "Beef stockers", "cattle"),
        case("cattle_beef_finisher", "Beef finishers", "cattle"),
        case("cattle_beef_500", "Beef cattle 500 kg", "cattle"),
        case("cattle_beef_600", "Beef cattle 600 kg", "cattle"),
        case("cattle_beef_700", "Beef cattle 700 kg", "cattle"),
        case("cattle_beef_800", "Beef cattle 800 kg", "cattle"),
        case("cattle_beef_900", "Beef cattle 900 kg", "cattle"),
        case("cattle_beef_1000", "Beef cattle 1000 kg", "cattle"),
        case("cattle_beef_1100", "Beef cattle 1100 kg", "cattle"),
        case("cattle_beef_1200", "Beef cattle 1200 kg", "cattle"),
        case("swine_piglet_nursery", "Nursery piglets", "swine"),
        case("swine_grower", "Grower pigs", "swine"),
        case("swine_finisher", "Finisher pigs", "swine"),
        case("swine_sow_gestating", "Gestating sows", "swine"),
        case("swine_sow_lactating", "Lactating sows", "swine"),
        case("poultry_broiler_starter", "Broiler starter", "poultry"),
        case("poultry_broiler_grower", "Broiler grower", "poultry"),
        case("poultry_broiler_finisher", "Broiler finisher", "poultry"),
        case("poultry_layer_peak", "Laying hens", "poultry"),
    ]
}

pub fn run_publication_benchmark(
    feeds: &[Feed],
    price_info: Option<&HashMap<i64, BenchmarkPriceInfo>>,
    cases: &[BenchmarkCase],
) -> Result<BenchmarkRun> {
    let feed_index = feed_index(feeds);
    let mut results = Vec::with_capacity(cases.len());

    for (index, case) in cases.iter().enumerate() {
        eprintln!(
            "[{}/{}] benchmarking {}",
            index + 1,
            cases.len(),
            case.id
        );
        results.push(run_case(case, feeds, price_info, &feed_index)?);
    }

    let priced_feed_count = feeds
        .iter()
        .filter(|feed| feed_has_any_price(feed, price_info))
        .count();
    let direct_price_anchor_count = feeds
        .iter()
        .filter_map(|feed| feed.id)
        .filter(|feed_id| {
            price_info
                .and_then(|map| map.get(feed_id))
                .map(|info| !is_benchmark_price(info))
                .unwrap_or(false)
        })
        .count();
    let benchmark_priced_feed_count = feeds
        .iter()
        .filter_map(|feed| feed.id)
        .filter(|feed_id| {
            price_info
                .and_then(|map| map.get(feed_id))
                .map(is_benchmark_price)
                .unwrap_or(false)
        })
        .count();

    Ok(BenchmarkRun {
        methodology_version: "2026-03-24-workflow-economics-agent-v2".to_string(),
        source_feed_count: feeds.len(),
        priced_feed_count,
        direct_price_anchor_count,
        benchmark_priced_feed_count,
        cases: results,
    })
}

fn case(id: &str, label: &str, species: &str) -> BenchmarkCase {
    BenchmarkCase {
        id: id.to_string(),
        label: label.to_string(),
        species: species.to_string(),
    }
}

fn run_case(
    case: &BenchmarkCase,
    feeds: &[Feed],
    price_info: Option<&HashMap<i64, BenchmarkPriceInfo>>,
    feed_index: &HashMap<i64, Feed>,
) -> Result<BenchmarkCaseResult> {
    let norms = norms::get_norms_for_group(case.id.as_str())
        .ok_or_else(|| anyhow::anyhow!("No norms found for {}", case.id))?;
    let allowed_feeds: Vec<Feed> = feeds
        .iter()
        .filter(|feed| {
            is_feed_allowed_for_context(feed, case.species.as_str(), Some(case.id.as_str()))
        })
        .cloned()
        .collect();
    let library = assess_library(case, &norms, &allowed_feeds, price_info);
    let mut notes = Vec::new();
    let mut workflows = Vec::new();

    let build_started = Instant::now();
    let built =
        construct_ration_from_library(&empty_ration(case.id.as_str()), &norms, &allowed_feeds)?;
    let build_runtime_ms = build_started.elapsed().as_secs_f64() * 1000.0;

    let sparse_seed = derive_sparse_seed(
        case.id.as_str(),
        built.as_ref(),
        &norms,
        &allowed_feeds,
        feed_index,
        price_info,
    );

    if let Some(seed) = &sparse_seed {
        let selected_started = Instant::now();
        match optimize_ration_with_library(
            &seed.ration,
            OptimizationMode::TieredBalance,
            Some(&norms),
            Some(&allowed_feeds),
        ) {
            Ok(mut solution) => {
                solution
                    .workflow_notes
                    .extend(seed.assessment.notes.iter().cloned());
                solution.workflow_notes.push(
                    "Sparse benchmark kept only the user-entered starter feeds.".to_string(),
                );
                workflows.push(assess_diet_solution(
                    "selected_only",
                    "sparse",
                    &solution,
                    selected_started.elapsed().as_secs_f64() * 1000.0,
                    &norms,
                    case.id.as_str(),
                    case.species.as_str(),
                    &allowed_feeds,
                    feed_index,
                    price_info,
                ));
            }
            Err(error) => notes.push(format!("Selected-only benchmark failed: {error}")),
        }

        let starter_plan =
            build_auto_populate_plan(Some(case.id.as_str()), Some(&norms), &allowed_feeds);
        let mut working_ration = seed.ration.clone();
        let added_from_library = merge_starter_items(&mut working_ration, &starter_plan);
        let completion_started = Instant::now();
        match optimize_ration_with_library(
            &working_ration,
            OptimizationMode::RepairWithAdditions,
            Some(&norms),
            Some(&allowed_feeds),
        ) {
            Ok(mut solution) => {
                solution.auto_populated = added_from_library > 0;
                solution
                    .workflow_notes
                    .extend(seed.assessment.notes.iter().cloned());
                solution
                    .workflow_notes
                    .extend(starter_plan.notes.iter().cloned());
                if added_from_library > 0 {
                    solution.workflow_notes.push(format!(
                        "Library completion added {} starter feeds before optimization.",
                        added_from_library
                    ));
                }
                workflows.push(assess_diet_solution(
                    "complete_from_library",
                    "sparse",
                    &solution,
                    completion_started.elapsed().as_secs_f64() * 1000.0,
                    &norms,
                    case.id.as_str(),
                    case.species.as_str(),
                    &allowed_feeds,
                    feed_index,
                    price_info,
                ));
            }
            Err(error) => notes.push(format!("Library-completion benchmark failed: {error}")),
        }
    } else {
        notes.push(
            "Sparse starter ration could not be derived for paired workflow comparison."
                .to_string(),
        );
    }

    match built {
        Some(mut built_solution) => {
            built_solution.workflow_notes.insert(
                0,
                "Empty-ration benchmark delegated starter construction to the feed library."
                    .to_string(),
            );
            workflows.push(assess_diet_solution(
                "build_from_library",
                "empty",
                &built_solution,
                build_runtime_ms,
                &norms,
                case.id.as_str(),
                case.species.as_str(),
                &allowed_feeds,
                feed_index,
                price_info,
            ));
        }
        None => {
            notes.push("Build-from-library benchmark did not return a solution.".to_string());
        }
    }

    Ok(BenchmarkCaseResult {
        case: case.clone(),
        library,
        sparse_seed: sparse_seed.map(|seed| seed.assessment),
        workflows,
        notes,
    })
}

fn assess_library(
    case: &BenchmarkCase,
    norms: &AnimalNorm,
    allowed_feeds: &[Feed],
    price_info: Option<&HashMap<i64, BenchmarkPriceInfo>>,
) -> LibraryDiagnostics {
    let required_groups =
        required_groups_for_species(case.species.as_str(), Some(case.id.as_str()));
    let allowed_group_set = allowed_feeds
        .iter()
        .map(|feed| group_label(classify_feed(feed)).to_string())
        .collect::<BTreeSet<_>>();
    let present_groups = allowed_feeds.iter().map(classify_feed).collect::<Vec<_>>();
    let missing_required = validate_group_coverage(&present_groups, &required_groups)
        .into_iter()
        .map(group_label)
        .map(str::to_string)
        .collect::<Vec<_>>();

    let mut metric_support_counts = BTreeMap::new();
    for key in benchmark_metric_keys(norms) {
        let count = allowed_feeds
            .iter()
            .filter(|feed| feed_has_metric_signal(feed, key.as_str()))
            .count();
        metric_support_counts.insert(key, count);
    }

    let mut direct_price_feed_count = 0usize;
    let mut benchmark_price_feed_count = 0usize;
    let mut unpriced_feed_count = 0usize;

    for feed in allowed_feeds {
        match feed_price_bucket(feed, price_info) {
            PriceBucket::Direct => direct_price_feed_count += 1,
            PriceBucket::Benchmark => benchmark_price_feed_count += 1,
            PriceBucket::Unpriced => unpriced_feed_count += 1,
        }
    }

    LibraryDiagnostics {
        allowed_feed_count: allowed_feeds.len(),
        required_groups: required_groups
            .iter()
            .map(|group| group_label(*group).to_string())
            .collect(),
        allowed_groups: allowed_group_set.into_iter().collect(),
        missing_required_groups: missing_required,
        metric_support_counts,
        direct_price_feed_count,
        benchmark_price_feed_count,
        unpriced_feed_count,
    }
}

fn derive_sparse_seed(
    group_id: &str,
    built_solution: Option<&DietSolution>,
    norms: &AnimalNorm,
    allowed_feeds: &[Feed],
    feed_index: &HashMap<i64, Feed>,
    price_info: Option<&HashMap<i64, BenchmarkPriceInfo>>,
) -> Option<SparseSeedInternal> {
    let (candidates, source_note, original_total) = if let Some(solution) = built_solution {
        let candidates = sparse_candidates_from_solution(solution, feed_index);
        let total = solution.items.iter().map(|item| item.amount_kg).sum::<f64>();
        (candidates, "Derived from build-from-library output.", total)
    } else {
        let plan = build_auto_populate_plan(Some(group_id), Some(norms), allowed_feeds);
        let total = plan.items.iter().map(|item| item.amount_kg).sum::<f64>();
        (
            sparse_candidates_from_plan(&plan),
            "Derived from auto-populate starter plan because build-from-library returned no solution.",
            total,
        )
    };

    if candidates.is_empty() || original_total <= 0.0 {
        return None;
    }

    let selected = select_sparse_candidates(&candidates);
    if selected.is_empty() {
        return None;
    }

    let total_amount_kg = selected.iter().map(|candidate| candidate.amount_kg).sum::<f64>();
    let retained_mass_share_pct = 100.0 * total_amount_kg / original_total.max(1e-9);
    let mut category_shares_pct = BTreeMap::new();
    for candidate in &selected {
        *category_shares_pct
            .entry(group_label(candidate.group).to_string())
            .or_insert(0.0) += 100.0 * candidate.amount_kg / total_amount_kg.max(1e-9);
    }

    let ration = RationFull {
        ration: Ration {
            id: Some(1),
            name: format!("Sparse seed {group_id}"),
            animal_group_id: Some(group_id.to_string()),
            animal_count: 1,
            description: Some("Benchmark sparse starter".to_string()),
            status: "draft".to_string(),
            created_at: None,
            updated_at: None,
        },
        items: selected
            .iter()
            .enumerate()
            .map(|(index, candidate)| RationItem {
                id: None,
                ration_id: 1,
                feed_id: candidate.feed_id,
                feed: Some(candidate.feed.clone()),
                amount_kg: candidate.amount_kg,
                is_locked: false,
                sort_order: index as i32,
            })
            .collect(),
    };

    Some(SparseSeedInternal {
        ration,
        assessment: SparseSeedAssessment {
            feed_count: selected.len(),
            total_amount_kg,
            retained_mass_share_pct,
            category_shares_pct,
            feeds: selected
                .iter()
                .map(|candidate| BenchFeedAmount {
                    feed_id: candidate.feed_id,
                    feed_name: candidate.feed_name.clone(),
                    amount_kg: candidate.amount_kg,
                    cost_per_day: candidate.cost_per_day,
                    price_provenance_kind: candidate
                        .feed
                        .id
                        .and_then(|feed_id| price_info.and_then(|map| map.get(&feed_id)))
                        .map(price_kind_label)
                        .unwrap_or_else(|| "unpriced".to_string()),
                })
                .collect(),
            notes: vec![
                source_note.to_string(),
                format!(
                    "Sparse seed retained {:.1}% of the original ration mass.",
                    retained_mass_share_pct
                ),
                "Two-feed seed keeps one forage-like and one concentrate-like component when available."
                    .to_string(),
            ],
        },
    })
}

fn sparse_candidates_from_solution(
    solution: &DietSolution,
    feed_index: &HashMap<i64, Feed>,
) -> Vec<SparseCandidate> {
    let mut candidates = solution
        .items
        .iter()
        .filter_map(|item| {
            let feed = feed_index.get(&item.feed_id)?.clone();
            Some(SparseCandidate {
                feed_id: item.feed_id,
                feed_name: item.feed_name.clone(),
                amount_kg: item.amount_kg,
                cost_per_day: item.cost_per_day,
                group: classify_feed(&feed),
                feed,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.amount_kg.total_cmp(&left.amount_kg));
    candidates
}

fn sparse_candidates_from_plan(plan: &AutoPopulatePlan) -> Vec<SparseCandidate> {
    let mut candidates = plan
        .items
        .iter()
        .filter_map(|item| {
            let feed_id = item.feed.id?;
            Some(SparseCandidate {
                feed_id,
                feed_name: item.feed.name_ru.clone(),
                amount_kg: item.amount_kg,
                cost_per_day: item.amount_kg * item.feed.price_per_kg(),
                group: item.group,
                feed: item.feed.clone(),
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.amount_kg.total_cmp(&left.amount_kg));
    candidates
}

fn select_sparse_candidates(candidates: &[SparseCandidate]) -> Vec<SparseCandidate> {
    let mut selected = Vec::new();
    let mut seen = HashSet::new();

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| matches!(candidate.group, FeedGroup::Roughage | FeedGroup::Succulent))
    {
        selected.push(candidate.clone());
        seen.insert(candidate.feed_id);
    }

    if let Some(candidate) = candidates.iter().find(|candidate| {
        !seen.contains(&candidate.feed_id)
            && matches!(
                candidate.group,
                FeedGroup::Concentrate
                    | FeedGroup::Protein
                    | FeedGroup::AnimalOrigin
                    | FeedGroup::Other
            )
    }) {
        selected.push(candidate.clone());
        seen.insert(candidate.feed_id);
    }

    for candidate in candidates {
        if selected.len() >= 2 {
            break;
        }
        if seen.contains(&candidate.feed_id) || is_supplement_group(candidate.group) {
            continue;
        }
        selected.push(candidate.clone());
        seen.insert(candidate.feed_id);
    }

    for candidate in candidates {
        if selected.len() >= 2 {
            break;
        }
        if seen.contains(&candidate.feed_id) {
            continue;
        }
        selected.push(candidate.clone());
        seen.insert(candidate.feed_id);
    }

    selected.sort_by(|left, right| right.amount_kg.total_cmp(&left.amount_kg));
    selected
}

fn merge_starter_items(ration: &mut RationFull, starter_plan: &AutoPopulatePlan) -> usize {
    let mut existing_feed_ids = ration.items.iter().map(|item| item.feed_id).collect::<HashSet<_>>();
    let mut added = 0usize;
    let mut starter_items = plan_to_ration_items(starter_plan, 1);
    let mut next_sort_order = ration.items.len() as i32;

    for mut starter_item in starter_items.drain(..) {
        if existing_feed_ids.contains(&starter_item.feed_id) {
            continue;
        }
        starter_item.sort_order = next_sort_order;
        next_sort_order += 1;
        existing_feed_ids.insert(starter_item.feed_id);
        ration.items.push(starter_item);
        added += 1;
    }

    added
}

fn assess_diet_solution(
    intent: &str,
    entry_state: &str,
    solution: &DietSolution,
    runtime_ms: f64,
    norms: &AnimalNorm,
    group_id: &str,
    species: &str,
    allowed_feeds: &[Feed],
    feed_index: &HashMap<i64, Feed>,
    price_info: Option<&HashMap<i64, BenchmarkPriceInfo>>,
) -> WorkflowAssessment {
    assess_solution_parts(
        intent,
        entry_state,
        solution.optimization_status.clone(),
        solution.applied_strategy.clone(),
        runtime_ms,
        solution.auto_populated,
        solution.best_achievable,
        &solution.items,
        &solution.nutrient_summary,
        &solution.workflow_notes,
        norms,
        group_id,
        species,
        allowed_feeds,
        feed_index,
        price_info,
    )
}

#[allow(clippy::too_many_arguments)]
fn assess_solution_parts(
    intent: &str,
    entry_state: &str,
    optimization_status: SolutionStatus,
    applied_strategy: String,
    runtime_ms: f64,
    auto_populated: bool,
    best_achievable: bool,
    items: &[OptimizedItem],
    nutrient_summary: &NutrientSummary,
    workflow_notes: &[String],
    norms: &AnimalNorm,
    group_id: &str,
    species: &str,
    allowed_feeds: &[Feed],
    feed_index: &HashMap<i64, Feed>,
    price_info: Option<&HashMap<i64, BenchmarkPriceInfo>>,
) -> WorkflowAssessment {
    let total_amount_kg = items.iter().map(|item| item.amount_kg).sum::<f64>().max(1e-9);
    let mut category_shares = BTreeMap::new();
    let mut appropriate = 0.0;
    let mut conditional = 0.0;
    let mut restricted = 0.0;
    let mut present_groups = Vec::new();
    let mut priced_mass = 0.0;
    let mut direct_cost = 0.0;
    let mut benchmark_cost = 0.0;
    let mut confidence_weighted_cost = 0.0;

    for item in items {
        if let Some(feed) = feed_index.get(&item.feed_id) {
            let group = classify_feed(feed);
            let group_key = group_label(group).to_string();
            *category_shares.entry(group_key).or_insert(0.0) +=
                item.amount_kg / total_amount_kg * 100.0;
            present_groups.push(group);

            if item.cost_per_day > 0.0 {
                priced_mass += item.amount_kg;
                match feed
                    .id
                    .and_then(|feed_id| price_info.and_then(|map| map.get(&feed_id)))
                {
                    Some(info) => {
                        if is_benchmark_price(info) {
                            benchmark_cost += item.cost_per_day;
                        } else {
                            direct_cost += item.cost_per_day;
                        }
                        confidence_weighted_cost +=
                            item.cost_per_day * price_confidence_weight(info);
                    }
                    None => {
                        direct_cost += item.cost_per_day;
                        confidence_weighted_cost += item.cost_per_day * 0.50;
                    }
                }
            }

            match assess_feed_suitability(feed, species, Some(group_id)).status {
                FeedSuitabilityStatus::Appropriate => appropriate += item.amount_kg,
                FeedSuitabilityStatus::Conditional => conditional += item.amount_kg,
                FeedSuitabilityStatus::Restricted => restricted += item.amount_kg,
            }
        }
    }

    let required_groups = required_groups_for_species(species, Some(group_id));
    let missing_groups = validate_group_coverage(&present_groups, &required_groups);
    let group_coverage_score = if required_groups.is_empty() {
        100.0
    } else {
        100.0 * (required_groups.len().saturating_sub(missing_groups.len()) as f64)
            / required_groups.len() as f64
    };

    let mut constraints = Vec::new();
    let mut unevaluable_constraints = Vec::new();
    let mut tier_passes = [0usize; 3];
    let mut tier_totals = [0usize; 3];
    let mut weighted_penalty = 0.0;
    let mut weighted_deficiency = 0.0;
    let mut weighted_excess = 0.0;
    let mut weighted_target = 0.0;
    let mut weight_sum = 0.0;

    for key in benchmark_metric_keys(norms) {
        let min = norms.nutrients_min.get(&key).copied();
        let max = norms.nutrients_max.get(&key).copied();
        let target = norms.nutrients_target.get(&key).copied();
        let actual = match actual_metric_value(norms, nutrient_summary, key.as_str()) {
            Some(value) => value,
            None => {
                unevaluable_constraints.push(key);
                continue;
            }
        };

        let deficiency = relative_deficit(min, actual);
        let excess = relative_excess(max, actual);
        let target_gap = relative_target_gap(target, actual);
        let hard_pass = deficiency == 0.0
            && excess == 0.0
            && (min.is_some() || max.is_some() || target_gap <= 0.05);
        let tier = constraint_tier_for_key(norms, key.as_str());
        let tier_index = tier_index(tier);
        tier_totals[tier_index] += 1;
        if hard_pass {
            tier_passes[tier_index] += 1;
        }

        let weight = tier_weight(tier);
        let penalty = deficiency + excess + TARGET_ALIGNMENT_WEIGHT * target_gap;
        weight_sum += weight;
        weighted_penalty += weight * penalty;
        weighted_deficiency += weight * deficiency;
        weighted_excess += weight * excess;
        weighted_target += weight * target_gap;

        constraints.push(ConstraintAssessment {
            key: key.clone(),
            tier: (tier_index + 1) as u8,
            actual,
            min,
            max,
            target,
            relative_deficit: deficiency,
            relative_excess: excess,
            relative_target_gap: target_gap,
            support_feed_count: allowed_feeds
                .iter()
                .filter(|feed| feed_has_metric_signal(feed, key.as_str()))
                .count(),
            hard_pass,
            score: 100.0 / (1.0 + penalty),
        });
    }

    constraints.sort_by(|left, right| {
        issue_severity(right)
            .partial_cmp(&issue_severity(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let constraint_violation_index = if weight_sum > 0.0 {
        weighted_penalty / weight_sum
    } else {
        0.0
    };
    let norm_coverage_index = 100.0 / (1.0 + constraint_violation_index);
    let suitability_score =
        100.0 * ((appropriate + 0.5 * conditional) / total_amount_kg).clamp(0.0, 1.0);
    let feasibility_factor = match optimization_status {
        SolutionStatus::Optimal | SolutionStatus::Feasible => 1.0,
        SolutionStatus::Infeasible => 0.75,
        SolutionStatus::Unbounded | SolutionStatus::Error => 0.50,
    };
    let auxiliary_composite_index =
        (0.70 * norm_coverage_index + 0.15 * suitability_score + 0.15 * group_coverage_score)
            * feasibility_factor;
    let cost_per_day_rub: f64 = items.iter().map(|item| item.cost_per_day).sum();
    let screening = assess_screening(items, norms, allowed_feeds, feed_index);

    WorkflowAssessment {
        intent: intent.to_string(),
        entry_state: entry_state.to_string(),
        optimization_status: solution_status_label(optimization_status).to_string(),
        applied_strategy,
        runtime_ms,
        auto_populated,
        best_achievable,
        feed_count: items.len(),
        total_amount_kg,
        category_richness: category_shares.len(),
        category_shares_pct: category_shares,
        suitability: SuitabilitySummary {
            appropriate_share_pct: 100.0 * appropriate / total_amount_kg,
            conditional_share_pct: 100.0 * conditional / total_amount_kg,
            restricted_share_pct: 100.0 * restricted / total_amount_kg,
            score: suitability_score,
        },
        required_group_coverage: RequiredGroupCoverage {
            covered_groups: present_groups
                .iter()
                .copied()
                .map(group_label)
                .map(str::to_string)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            missing_groups: missing_groups
                .into_iter()
                .map(group_label)
                .map(str::to_string)
                .collect(),
            score: group_coverage_score,
        },
        cost_per_day_rub,
        rub_per_kg_as_fed: cost_per_day_rub / total_amount_kg,
        price_coverage_pct: 100.0 * priced_mass / total_amount_kg,
        direct_price_share_pct: if cost_per_day_rub > 0.0 {
            100.0 * direct_cost / cost_per_day_rub
        } else {
            0.0
        },
        benchmark_price_share_pct: if cost_per_day_rub > 0.0 {
            100.0 * benchmark_cost / cost_per_day_rub
        } else {
            0.0
        },
        price_confidence_score: if cost_per_day_rub > 0.0 {
            100.0 * confidence_weighted_cost / cost_per_day_rub
        } else {
            0.0
        },
        evaluable_constraints: constraints.len(),
        unevaluable_constraints,
        hard_pass_rate: pass_rate(
            constraints
                .iter()
                .filter(|constraint| constraint.hard_pass)
                .count(),
            constraints.len(),
        ),
        tier1_pass_rate: pass_rate(tier_passes[0], tier_totals[0]),
        tier2_pass_rate: pass_rate(tier_passes[1], tier_totals[1]),
        tier3_pass_rate: pass_rate(tier_passes[2], tier_totals[2]),
        norm_coverage_index,
        deficiency_index: if weight_sum > 0.0 {
            weighted_deficiency / weight_sum
        } else {
            0.0
        },
        excess_index: if weight_sum > 0.0 {
            weighted_excess / weight_sum
        } else {
            0.0
        },
        target_gap_index: if weight_sum > 0.0 {
            weighted_target / weight_sum
        } else {
            0.0
        },
        constraint_violation_index,
        auxiliary_composite_index,
        post_screening: screening,
        top_issues: constraints.into_iter().take(8).collect(),
        feeds: items
            .iter()
            .map(|item| BenchFeedAmount {
                feed_id: item.feed_id,
                feed_name: item.feed_name.clone(),
                amount_kg: item.amount_kg,
                cost_per_day: item.cost_per_day,
                price_provenance_kind: price_info
                    .and_then(|map| map.get(&item.feed_id))
                    .map(price_kind_label)
                    .unwrap_or_else(|| {
                        if item.cost_per_day > 0.0 {
                            "direct_or_manual".to_string()
                        } else {
                            "unpriced".to_string()
                        }
                    }),
            })
            .collect(),
        workflow_notes: workflow_notes.to_vec(),
    }
}

fn assess_screening(
    items: &[OptimizedItem],
    norms: &AnimalNorm,
    allowed_feeds: &[Feed],
    feed_index: &HashMap<i64, Feed>,
) -> ScreeningAssessment {
    let ration_items = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let feed = feed_index.get(&item.feed_id)?.clone();
            Some(RationItem {
                id: None,
                ration_id: 1,
                feed_id: item.feed_id,
                feed: Some(feed),
                amount_kg: item.amount_kg,
                is_locked: false,
                sort_order: index as i32,
            })
        })
        .collect::<Vec<_>>();

    let report = screen_current_feed_set(&ration_items, allowed_feeds, norms);
    ScreeningAssessment {
        can_meet_reference: report.can_meet_reference,
        limiting_nutrients: report.limiting_nutrients,
        recommendations: report
            .recommendations
            .into_iter()
            .map(|recommendation| BenchRecommendation {
                feed_id: recommendation.feed_id,
                feed_name: recommendation.feed_name,
                reason: recommendation.reason,
                suggested_amount_kg: recommendation.suggested_amount_kg,
                category: recommendation.category,
                priority: recommendation.priority,
            })
            .collect(),
    }
}

fn empty_ration(group_id: &str) -> RationFull {
    RationFull {
        ration: Ration {
            id: Some(1),
            name: format!("Benchmark {group_id}"),
            animal_group_id: Some(group_id.to_string()),
            animal_count: 1,
            description: Some("Publication benchmark".to_string()),
            status: "draft".to_string(),
            created_at: None,
            updated_at: None,
        },
        items: Vec::new(),
    }
}

fn benchmark_metric_keys(norms: &AnimalNorm) -> Vec<String> {
    let mut keys = BTreeSet::new();
    if norms.feed_intake_min.is_some() || norms.feed_intake_max.is_some() {
        keys.insert(if norms.species == "cattle" {
            "dry_matter_intake".to_string()
        } else {
            "feed_intake".to_string()
        });
    }
    keys.extend(norms.nutrients_min.keys().cloned());
    keys.extend(norms.nutrients_max.keys().cloned());
    keys.extend(norms.nutrients_target.keys().cloned());
    keys.into_iter().collect()
}

fn feed_index(feeds: &[Feed]) -> HashMap<i64, Feed> {
    feeds
        .iter()
        .filter_map(|feed| feed.id.map(|id| (id, feed.clone())))
        .collect()
}

fn solution_status_label(status: SolutionStatus) -> &'static str {
    match status {
        SolutionStatus::Optimal => "optimal",
        SolutionStatus::Feasible => "feasible",
        SolutionStatus::Infeasible => "infeasible",
        SolutionStatus::Unbounded => "unbounded",
        SolutionStatus::Error => "error",
    }
}

fn relative_deficit(min: Option<f64>, actual: f64) -> f64 {
    match min {
        Some(minimum) if minimum.abs() > f64::EPSILON && actual < minimum => {
            (minimum - actual) / minimum.abs()
        }
        _ => 0.0,
    }
}

fn relative_excess(max: Option<f64>, actual: f64) -> f64 {
    match max {
        Some(maximum) if maximum.abs() > f64::EPSILON && actual > maximum => {
            (actual - maximum) / maximum.abs()
        }
        _ => 0.0,
    }
}

fn relative_target_gap(target: Option<f64>, actual: f64) -> f64 {
    match target {
        Some(target) if target.abs() > f64::EPSILON => (actual - target).abs() / target.abs(),
        Some(_) => actual.abs(),
        None => 0.0,
    }
}

fn tier_index(tier: ConstraintTier) -> usize {
    match tier {
        ConstraintTier::Tier1 => 0,
        ConstraintTier::Tier2 => 1,
        ConstraintTier::Tier3 => 2,
    }
}

fn tier_weight(tier: ConstraintTier) -> f64 {
    match tier {
        ConstraintTier::Tier1 => 3.0,
        ConstraintTier::Tier2 => 2.0,
        ConstraintTier::Tier3 => 1.0,
    }
}

fn pass_rate(passed: usize, total: usize) -> f64 {
    if total == 0 {
        100.0
    } else {
        100.0 * passed as f64 / total as f64
    }
}

fn issue_severity(issue: &ConstraintAssessment) -> f64 {
    issue.relative_deficit
        + issue.relative_excess
        + TARGET_ALIGNMENT_WEIGHT * issue.relative_target_gap
}

fn feed_has_any_price(feed: &Feed, price_info: Option<&HashMap<i64, BenchmarkPriceInfo>>) -> bool {
    match feed.id {
        Some(feed_id) => {
            price_info
                .and_then(|map| map.get(&feed_id))
                .is_some()
                || feed.price_per_ton.unwrap_or(0.0) > 0.0
        }
        None => feed.price_per_ton.unwrap_or(0.0) > 0.0,
    }
}

#[derive(Debug, Clone, Copy)]
enum PriceBucket {
    Direct,
    Benchmark,
    Unpriced,
}

fn feed_price_bucket(
    feed: &Feed,
    price_info: Option<&HashMap<i64, BenchmarkPriceInfo>>,
) -> PriceBucket {
    match feed.id.and_then(|feed_id| price_info.and_then(|map| map.get(&feed_id))) {
        Some(info) if is_benchmark_price(info) => PriceBucket::Benchmark,
        Some(_) => PriceBucket::Direct,
        None if feed.price_per_ton.unwrap_or(0.0) > 0.0 => PriceBucket::Direct,
        None => PriceBucket::Unpriced,
    }
}

fn is_benchmark_price(info: &BenchmarkPriceInfo) -> bool {
    info.kind == "benchmark"
}

fn price_kind_label(info: &BenchmarkPriceInfo) -> String {
    if info.kind == "benchmark" {
        match info.benchmark_level.as_deref() {
            Some(level) => format!("benchmark:{level}"),
            None => "benchmark".to_string(),
        }
    } else if info.kind == "direct" && info.is_precise_source {
        "direct:precise".to_string()
    } else {
        info.kind.clone()
    }
}

fn price_confidence_weight(info: &BenchmarkPriceInfo) -> f64 {
    match (info.kind.as_str(), info.benchmark_level.as_deref(), info.is_precise_source) {
        ("direct", _, true) => 1.00,
        ("direct", _, false) => 0.85,
        ("seed", _, _) | ("manual", _, _) => 0.80,
        ("benchmark", Some("subcategory"), _) => 0.65,
        ("benchmark", Some("category"), _) => 0.55,
        ("benchmark", Some("family"), _) => 0.40,
        ("benchmark", Some("global"), _) => 0.25,
        ("benchmark", _, _) => 0.35,
        _ => 0.50,
    }
}

fn is_supplement_group(group: FeedGroup) -> bool {
    matches!(
        group,
        FeedGroup::Mineral | FeedGroup::Premix | FeedGroup::Vitamin
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(id: i64, name: &str, category: &str) -> Feed {
        Feed {
            id: Some(id),
            source_id: Some(format!("seed:test:{id}")),
            name_ru: name.to_string(),
            category: category.to_string(),
            dry_matter: Some(88.0),
            price_per_ton: Some(10000.0),
            ..Default::default()
        }
    }

    #[test]
    fn sparse_seed_prefers_forage_and_energy_components() {
        let hay = feed(1, "Hay", "roughage");
        let barley = feed(2, "Barley", "grain");
        let chalk = feed(3, "Chalk", "mineral");

        let feed_index = HashMap::from([
            (1_i64, hay.clone()),
            (2_i64, barley.clone()),
            (3_i64, chalk.clone()),
        ]);
        let solution = DietSolution {
            items: vec![
                OptimizedItem {
                    feed_id: 1,
                    feed_name: "Hay".to_string(),
                    amount_kg: 8.0,
                    dm_kg: 7.0,
                    cost_per_day: 80.0,
                },
                OptimizedItem {
                    feed_id: 2,
                    feed_name: "Barley".to_string(),
                    amount_kg: 3.0,
                    dm_kg: 2.6,
                    cost_per_day: 30.0,
                },
                OptimizedItem {
                    feed_id: 3,
                    feed_name: "Chalk".to_string(),
                    amount_kg: 0.2,
                    dm_kg: 0.2,
                    cost_per_day: 2.0,
                },
            ],
            nutrient_summary: NutrientSummary::default(),
            cost_per_day: 112.0,
            optimization_status: SolutionStatus::Feasible,
            warnings: Vec::new(),
            recommendations: Vec::new(),
            applied_strategy: "test".to_string(),
            auto_populated: false,
            solve_intent: None,
            ration_state: None,
            workflow_notes: Vec::new(),
            best_achievable: false,
            relaxed_targets: Vec::new(),
            auto_added_feeds: Vec::new(),
            alternatives: Vec::new(),
        };

        let seed = derive_sparse_seed(
            "cattle_dairy_fresh",
            Some(&solution),
            &AnimalNorm::default(),
            &[hay, barley, chalk],
            &feed_index,
            None,
        )
        .unwrap();

        let selected_ids = seed
            .ration
            .items
            .iter()
            .map(|item| item.feed_id)
            .collect::<HashSet<_>>();
        assert_eq!(selected_ids.len(), 2);
        assert!(selected_ids.contains(&1));
        assert!(selected_ids.contains(&2));
        assert!(!selected_ids.contains(&3));
    }

    #[test]
    fn confidence_weights_decrease_for_weaker_provenance() {
        let direct = BenchmarkPriceInfo {
            kind: "direct".to_string(),
            is_precise_source: true,
            benchmark_level: None,
        };
        let subcategory = BenchmarkPriceInfo {
            kind: "benchmark".to_string(),
            is_precise_source: false,
            benchmark_level: Some("subcategory".to_string()),
        };
        let global = BenchmarkPriceInfo {
            kind: "benchmark".to_string(),
            is_precise_source: false,
            benchmark_level: Some("global".to_string()),
        };

        assert!(price_confidence_weight(&direct) > price_confidence_weight(&subcategory));
        assert!(price_confidence_weight(&subcategory) > price_confidence_weight(&global));
    }
}
