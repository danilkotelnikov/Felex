use std::collections::{BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::db::{
    feeds::Feed,
    rations::{RationFull, RationItem},
};
use crate::norms::{factorial::ConstraintTier, AnimalNorm};

use super::{
    actual_metric_value, all_objective_keys, constraint_tier_for_key, optimize_with_library,
    DietSolution, NutritionWarning, OptimizedItem, OptimizationMode, SolutionStatus,
};
use super::feed_groups::{
    classify_feed, feeds_by_group, is_feed_allowed_for_context, required_groups_for_species,
    score_feed_for_group, template_for_group, validate_group_coverage, FeedGroup,
};

const DEFAULT_MAX_SOLUTIONS: usize = 4;
const MIN_TOTAL_SOLUTIONS: usize = 3;
const MAX_TOTAL_SOLUTIONS: usize = 6;
const COST_TOLERANCE_FRACTION: f64 = 0.20;
const SCORE_TOLERANCE_POINTS: f64 = 15.0;
const MIN_ACCEPTABLE_SCORE: f64 = 55.0;
const NEAR_DUPLICATE_CONTAINMENT: f64 = 0.75;
const DISCOURAGED_REPEAT_MARKERS: [&str; 6] =
    ["паприн", "paprin", "лён", "лен", "linseed", "flax"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeRationSolution {
    pub id: String,
    pub label: String,
    pub feeds: Vec<OptimizedItem>,
    pub nutrients: super::nutrient_calc::NutrientSummary,
    pub adequacy_score: f64,
    pub cost: f64,
    pub tags: Vec<String>,
    pub optimization_status: SolutionStatus,
    pub applied_strategy: String,
    pub warnings: Vec<NutritionWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationComparison {
    pub cost_range: [f64; 2],
    pub score_range: [f64; 2],
    pub common_feeds: Vec<String>,
    pub differentiators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub primary: AlternativeRationSolution,
    pub alternatives: Vec<AlternativeRationSolution>,
    pub comparison: OptimizationComparison,
}

#[derive(Debug, Clone, Copy)]
struct AdequacyBreakdown {
    overall: f64,
    tier1: f64,
}

#[derive(Debug, Clone)]
struct CandidateSolution {
    id: String,
    label: String,
    tags: Vec<String>,
    solution: DietSolution,
    adequacy: AdequacyBreakdown,
    signature: String,
    feed_set_signature: String,
}

pub fn optimize_with_alternatives(
    ration: &RationFull,
    mode: OptimizationMode,
    norms_override: Option<&AnimalNorm>,
    available_feeds: Option<&[Feed]>,
    max_solutions: Option<usize>,
) -> anyhow::Result<OptimizationResult> {
    let target_count = max_solutions
        .unwrap_or(DEFAULT_MAX_SOLUTIONS)
        .clamp(MIN_TOTAL_SOLUTIONS, MAX_TOTAL_SOLUTIONS);
    let library = available_feeds.unwrap_or(&[]);

    // Phase 1: Generate the primary solution (unchanged).
    let primary_solution = optimize_with_library(ration, mode, norms_override, available_feeds)?;
    let primary = candidate_from_solution(
        "primary",
        "Optimal",
        &["requested_mode"],
        primary_solution,
        norms_override,
    );

    let primary_seed = solution_to_ration(&primary.solution, ration, library);
    let mut seen_signatures = HashSet::from([primary.signature.clone()]);
    let mut seen_feed_sets = HashSet::from([primary.feed_set_signature.clone()]);
    let mut accepted: Vec<CandidateSolution> = vec![primary.clone()];

    // Phase 2: Diversity-first iterative repeat escalation.
    //
    // For each alternative slot, try to generate a solution by capping feeds
    // from accepted solutions (setting them to zero in the seed). Start with
    // excluding ALL overlapping feeds (0 repeats allowed). If that fails,
    // escalate by allowing 1 repeat, then 2, etc. This produces nutritionally
    // equivalent but factually diverse rations.
    let alt_modes = [
        OptimizationMode::TieredBalance,
        OptimizationMode::SinglePassBalance,
        OptimizationMode::MinimizeCost,
        OptimizationMode::FixedRation,
        OptimizationMode::RepairWithAdditions,
    ];

    let alt_slots = target_count.saturating_sub(1);
    let primary_feed_count = primary.solution.items.len();
    let max_repeat_escalation = primary_feed_count.max(1);
    let species = species_from_group_id(ration.ration.animal_group_id.as_deref());
    let animal_group_id = ration.ration.animal_group_id.as_deref();

    for slot_index in 0..alt_slots {
        let mut found = false;

        // Try escalating repeat count: 0, 1, 2, ...
        'escalation: for allowed_repeats in 0..=max_repeat_escalation {
            let excluded = build_excluded_feed_set(&accepted);
            let allowed_repeat_ids = select_allowed_repeats(&accepted, allowed_repeats);

            for (mode_index, &try_mode) in alt_modes.iter().enumerate() {
                // Skip RepairWithAdditions when there is no library.
                if matches!(try_mode, OptimizationMode::RepairWithAdditions) && library.is_empty() {
                    continue;
                }

                // Build a seed ration that caps excluded feeds to zero,
                // except for feeds in the allowed-repeat set.
                let capped = cap_excluded_feeds_in_ration(
                    &primary_seed,
                    &excluded,
                    &allowed_repeat_ids,
                );

                // Inject library feeds for missing required groups so the LP
                // solver has new decision variables to work with.
                let seed_ration = build_diverse_seed(
                    &capped,
                    library,
                    &excluded,
                    species,
                    animal_group_id,
                );

                let id = format!("alt_{}_{}", slot_index + 1, mode_index);
                let label = format!("Alternative {}", slot_index + 1);

                let candidate = build_candidate(
                    &seed_ration,
                    try_mode,
                    norms_override,
                    available_feeds,
                    &id,
                    &label,
                    &["diversity", "auto_generated"],
                );

                if let Ok(Some(candidate)) = candidate {
                    if !seen_signatures.insert(candidate.signature.clone()) {
                        continue;
                    }
                    if !seen_feed_sets.insert(candidate.feed_set_signature.clone()) {
                        // Revert signature insertion -- this feed-set is a duplicate.
                        seen_signatures.remove(&candidate.signature);
                        continue;
                    }
                    if candidate_is_near_duplicate(&candidate, &accepted) {
                        seen_signatures.remove(&candidate.signature);
                        seen_feed_sets.remove(&candidate.feed_set_signature);
                        continue;
                    }
                    if !candidate_within_equivalence_band(&candidate, &primary) {
                        // Revert insertions.
                        seen_signatures.remove(&candidate.signature);
                        seen_feed_sets.remove(&candidate.feed_set_signature);
                        continue;
                    }
                    accepted.push(candidate);
                    found = true;
                    break 'escalation;
                }
            }
        }

        // Fallback: If escalation failed, try original-style strategies
        // (budget, stable, diverse via feed capping) without exclusion.
        if !found {
            let fallback_candidates = generate_fallback_candidates(
                ration,
                &primary,
                &primary_seed,
                norms_override,
                available_feeds,
                library,
                slot_index,
            );
            // First pass: prefer candidates within equivalence band.
            for candidate in &fallback_candidates {
                if seen_signatures.contains(&candidate.signature) {
                    continue;
                }
                if seen_feed_sets.contains(&candidate.feed_set_signature) {
                    continue;
                }
                if candidate_is_near_duplicate(candidate, &accepted) {
                    continue;
                }
                if !candidate_within_equivalence_band(candidate, &primary) {
                    continue;
                }
                seen_signatures.insert(candidate.signature.clone());
                seen_feed_sets.insert(candidate.feed_set_signature.clone());
                accepted.push(candidate.clone());
                found = true;
                break;
            }
            // Second pass: accept any unique candidate regardless of band.
            if !found {
                for candidate in &fallback_candidates {
                    if seen_signatures.contains(&candidate.signature) {
                        continue;
                    }
                    if seen_feed_sets.contains(&candidate.feed_set_signature) {
                        continue;
                    }
                    seen_signatures.insert(candidate.signature.clone());
                    seen_feed_sets.insert(candidate.feed_set_signature.clone());
                    accepted.push(candidate.clone());
                    found = true;
                    break;
                }
            }
        }

        if !found {
            // No more solutions can be produced; stop trying.
            break;
        }
    }

    finalize_candidate_tags(&mut accepted);

    let primary_solution = to_api_solution(accepted.first().expect("primary candidate"));
    let alternative_solutions = accepted
        .iter()
        .skip(1)
        .map(to_api_solution)
        .collect::<Vec<_>>();
    let comparison = build_comparison(&primary_solution, &alternative_solutions);

    Ok(OptimizationResult {
        primary: primary_solution,
        alternatives: alternative_solutions,
        comparison,
    })
}

/// Collects feed IDs from all accepted solutions into an excluded set.
fn build_excluded_feed_set(accepted: &[CandidateSolution]) -> HashSet<i64> {
    let mut excluded = HashSet::new();
    for candidate in accepted {
        for item in &candidate.solution.items {
            excluded.insert(item.feed_id);
        }
    }
    excluded
}

/// Builds a ration with excluded feeds capped to zero (not removed),
/// except those in `allowed_repeats` which retain their original amounts.
/// Keeping feeds at zero rather than removing them preserves the ration
/// structure so FixedRation and other modes can still operate.
fn cap_excluded_feeds_in_ration(
    ration: &RationFull,
    excluded: &HashSet<i64>,
    allowed_repeats: &HashSet<i64>,
) -> RationFull {
    RationFull {
        ration: ration.ration.clone(),
        items: ration
            .items
            .iter()
            .map(|item| {
                let mut next = item.clone();
                if excluded.contains(&item.feed_id) && !allowed_repeats.contains(&item.feed_id) {
                    next.amount_kg = 0.0;
                    next.is_locked = false;
                }
                next
            })
            .collect(),
    }
}

/// Injects library feeds into a seed ration to cover missing required groups.
///
/// After `cap_excluded_feeds_in_ration()` zeroes excluded feeds, the seed may
/// lack active feeds in some required groups. This function identifies those
/// missing groups and inserts 1-2 high-scoring candidate feeds from the library,
/// giving the LP solver new decision variables to work with.
fn build_diverse_seed(
    seed: &RationFull,
    library: &[Feed],
    excluded: &HashSet<i64>,
    species: &str,
    animal_group_id: Option<&str>,
) -> RationFull {
    if library.is_empty() {
        return seed.clone();
    }

    let existing_feed_ids: HashSet<i64> = seed.items.iter().map(|item| item.feed_id).collect();

    // Determine which groups are active (have feeds with amount > 0).
    let active_groups: Vec<FeedGroup> = seed
        .items
        .iter()
        .filter(|item| item.amount_kg > 0.0)
        .filter_map(|item| item.feed.as_ref().map(|f| classify_feed(f)))
        .collect();

    let required = required_groups_for_species(species, animal_group_id);
    let missing = validate_group_coverage(&active_groups, &required);

    if missing.is_empty() {
        return seed.clone();
    }

    // Estimate total ration weight for share-based initial amounts.
    let total_weight: f64 = seed
        .items
        .iter()
        .map(|item| item.amount_kg)
        .sum::<f64>()
        .max(10.0);

    let template = template_for_group(animal_group_id, species);
    let grouped = feeds_by_group(library);

    let mut result = seed.clone();
    let next_sort = result.items.len() as i32;

    for (group_idx, group) in missing.iter().enumerate() {
        let share = template
            .iter()
            .find(|ts| ts.group == *group)
            .map(|ts| ts.share)
            .unwrap_or(0.05);

        let Some(feeds_in_group) = grouped.get(group) else {
            continue;
        };

        // Score and filter candidates: not excluded, not already in seed, species-appropriate.
        let mut candidates: Vec<(&Feed, f64)> = feeds_in_group
            .iter()
            .filter(|f| {
                let fid = f.id.unwrap_or(0);
                !excluded.contains(&fid)
                    && !existing_feed_ids.contains(&fid)
                    && is_feed_allowed_for_context(f, species, animal_group_id)
            })
            .map(|f| (*f, score_feed_for_group(f, *group, species)))
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Insert up to 2 best candidates per missing group.
        for (i, (feed, _score)) in candidates.iter().take(2).enumerate() {
            let feed_id = feed.id.unwrap_or(0);
            let initial_amount = (total_weight * share) / (candidates.len().min(2) as f64).max(1.0);

            result.items.push(RationItem {
                id: None,
                ration_id: seed.ration.id.unwrap_or_default(),
                feed_id,
                feed: Some((*feed).clone()),
                amount_kg: initial_amount.max(0.1),
                is_locked: false,
                sort_order: next_sort + (group_idx * 2 + i) as i32,
            });
        }
    }

    result
}

/// Extracts the species string from an animal group ID.
fn species_from_group_id(group_id: Option<&str>) -> &str {
    group_id
        .map(|g| {
            if g.contains("cattle") {
                "cattle"
            } else if g.contains("swine") {
                "swine"
            } else if g.contains("poultry") {
                "poultry"
            } else {
                "cattle"
            }
        })
        .unwrap_or("cattle")
}

/// Runs cost optimization and repairs missing feed groups.
/// If MinimizeCost drops a required feed group, inserts the cheapest
/// available feed from that group at a minimal quantity.
pub fn cost_optimize_preserving_groups(
    ration: &RationFull,
    norms_override: Option<&AnimalNorm>,
    available_feeds: Option<&[Feed]>,
) -> anyhow::Result<DietSolution> {
    let mut solution = optimize_with_library(
        ration,
        OptimizationMode::MinimizeCost,
        norms_override,
        available_feeds,
    )?;

    let library = available_feeds.unwrap_or(&[]);
    if library.is_empty() {
        return Ok(solution);
    }

    let species = species_from_group_id(ration.ration.animal_group_id.as_deref());
    let required = required_groups_for_species(species, ration.ration.animal_group_id.as_deref());

    let present_groups: Vec<FeedGroup> = solution
        .items
        .iter()
        .filter_map(|item| {
            library
                .iter()
                .find(|f| f.id == Some(item.feed_id))
                .map(|f| classify_feed(f))
        })
        .collect();

    let missing = validate_group_coverage(&present_groups, &required);
    if missing.is_empty() {
        return Ok(solution);
    }

    let grouped = feeds_by_group(library);
    for group in missing {
        if let Some(feeds_in_group) = grouped.get(&group) {
            let cheapest = feeds_in_group
                .iter()
                .filter(|f| f.price_per_ton.unwrap_or(f64::INFINITY) > 0.0)
                .min_by(|a, b| {
                    a.price_per_ton
                        .unwrap_or(f64::INFINITY)
                        .partial_cmp(&b.price_per_ton.unwrap_or(f64::INFINITY))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            if let Some(feed) = cheapest {
                let feed_id = feed.id.unwrap_or(0);
                if !solution.items.iter().any(|item| item.feed_id == feed_id) {
                    let amount = match group {
                        FeedGroup::Mineral | FeedGroup::Premix | FeedGroup::Vitamin => 0.1,
                        _ => 1.0,
                    };
                    let price_per_kg = feed.price_per_ton.unwrap_or(0.0) / 1000.0;
                    solution.items.push(OptimizedItem {
                        feed_id,
                        feed_name: feed.name_ru.clone(),
                        amount_kg: amount,
                        dm_kg: amount * feed.dry_matter.unwrap_or(88.0) / 100.0,
                        cost_per_day: amount * price_per_kg,
                    });
                }
            }
        }
    }

    // Recalculate cost after additions
    let total_cost: f64 = solution
        .items
        .iter()
        .map(|item| {
            let price_per_kg = library
                .iter()
                .find(|f| f.id == Some(item.feed_id))
                .and_then(|f| f.price_per_ton)
                .map(|p| p / 1000.0)
                .unwrap_or(0.0);
            item.amount_kg * price_per_kg
        })
        .sum();
    solution.cost_per_day = total_cost;

    Ok(solution)
}

/// Picks N least-repeated feeds across accepted solutions to allow as repeats first.
/// Frequently repeated and specifically discouraged feeds are reintroduced last.
fn select_allowed_repeats(accepted: &[CandidateSolution], count: usize) -> HashSet<i64> {
    if count == 0 {
        return HashSet::new();
    }

    let mut frequency: HashMap<i64, usize> = HashMap::new();
    let mut total_amount: HashMap<i64, f64> = HashMap::new();
    let mut discouraged: HashMap<i64, bool> = HashMap::new();
    for candidate in accepted {
        for item in &candidate.solution.items {
            *frequency.entry(item.feed_id).or_insert(0) += 1;
            *total_amount.entry(item.feed_id).or_insert(0.0) += item.amount_kg;
            discouraged
                .entry(item.feed_id)
                .and_modify(|value| *value |= is_discouraged_repeat_feed_name(&item.feed_name))
                .or_insert_with(|| is_discouraged_repeat_feed_name(&item.feed_name));
        }
    }

    let mut freq_vec: Vec<(i64, usize, f64, bool)> = frequency
        .into_iter()
        .map(|(feed_id, feed_frequency)| {
            (
                feed_id,
                feed_frequency,
                total_amount.get(&feed_id).copied().unwrap_or_default(),
                discouraged.get(&feed_id).copied().unwrap_or(false),
            )
        })
        .collect();
    freq_vec.sort_by(|a, b| {
        a.3.cmp(&b.3)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| {
                a.2.partial_cmp(&b.2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.0.cmp(&b.0))
    });

    freq_vec
        .into_iter()
        .take(count)
        .map(|(id, _, _, _)| id)
        .collect()
}

/// Generates fallback candidates using the original-style strategies
/// when diversity-first escalation fails.
fn generate_fallback_candidates(
    ration: &RationFull,
    primary: &CandidateSolution,
    primary_seed: &RationFull,
    norms_override: Option<&AnimalNorm>,
    available_feeds: Option<&[Feed]>,
    library: &[Feed],
    slot_index: usize,
) -> Vec<CandidateSolution> {
    let mut candidates = Vec::new();

    // Budget alternative
    if let Ok(Some(c)) = build_candidate(
        ration,
        OptimizationMode::MinimizeCost,
        norms_override,
        available_feeds,
        &format!("fallback_budget_{}", slot_index + 1),
        &format!("Budget {}", slot_index + 1),
        &["fallback", "budget"],
    ) {
        if candidate_within_equivalence_band(&c, primary) {
            candidates.push(c);
        }
    }

    // Stable (FixedRation from primary seed)
    if let Ok(Some(c)) = build_candidate(
        primary_seed,
        OptimizationMode::FixedRation,
        norms_override,
        available_feeds,
        &format!("fallback_stable_{}", slot_index + 1),
        &format!("Stable {}", slot_index + 1),
        &["fallback", "stable"],
    ) {
        if candidate_within_equivalence_band(&c, primary) {
            candidates.push(c);
        }
    }

    // Diverse via dominant feed exclusion
    for (idx, feed_id) in dominant_and_discouraged_feed_ids(&primary.solution, 4)
        .into_iter()
        .enumerate()
    {
        let capped_seed = cap_feed_in_ration(primary_seed, feed_id, 0.0);
        if let Ok(Some(c)) = build_candidate(
            &capped_seed,
            OptimizationMode::FixedRation,
            norms_override,
            available_feeds,
            &format!("fallback_diverse_{}_{}", slot_index + 1, idx + 1),
            &format!("Diverse {}", slot_index + 1),
            &["fallback", "feed_exclusion"],
        ) {
            if candidate_within_equivalence_band(&c, primary) {
                candidates.push(c);
            }
        }

        if !library.is_empty() {
            if let Ok(Some(c)) = build_candidate(
                &capped_seed,
                OptimizationMode::RepairWithAdditions,
                norms_override,
                available_feeds,
                &format!("fallback_repair_{}_{}", slot_index + 1, idx + 1),
                &format!("Repair {}", slot_index + 1),
                &["fallback", "feed_exclusion"],
            ) {
                if candidate_within_equivalence_band(&c, primary) {
                    candidates.push(c);
                }
            }
        }
    }

    candidates
}

fn build_candidate(
    ration: &RationFull,
    mode: OptimizationMode,
    norms_override: Option<&AnimalNorm>,
    available_feeds: Option<&[Feed]>,
    id: &str,
    label: &str,
    tags: &[&str],
) -> anyhow::Result<Option<CandidateSolution>> {
    let solution = optimize_with_library(ration, mode, norms_override, available_feeds)?;
    if !matches!(
        solution.optimization_status,
        SolutionStatus::Optimal | SolutionStatus::Feasible
    ) || solution.items.is_empty()
    {
        return Ok(None);
    }

    Ok(Some(candidate_from_solution(
        id,
        label,
        tags,
        solution,
        norms_override,
    )))
}

fn candidate_from_solution(
    id: &str,
    label: &str,
    tags: &[&str],
    solution: DietSolution,
    norms_override: Option<&AnimalNorm>,
) -> CandidateSolution {
    let adequacy = adequacy_for_solution(&solution, norms_override);

    CandidateSolution {
        id: id.to_string(),
        label: label.to_string(),
        tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
        signature: solution_signature(&solution.items),
        feed_set_signature: feed_set_signature(&solution.items),
        solution,
        adequacy,
    }
}

fn dominant_feed_ids(solution: &DietSolution, limit: usize) -> Vec<i64> {
    let mut items = solution.items.clone();
    items.sort_by(|left, right| {
        right
            .amount_kg
            .partial_cmp(&left.amount_kg)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items.into_iter().take(limit).map(|item| item.feed_id).collect()
}

fn dominant_and_discouraged_feed_ids(solution: &DietSolution, limit: usize) -> Vec<i64> {
    let mut ranked = dominant_feed_ids(solution, limit);

    for item in &solution.items {
        if is_discouraged_repeat_feed_name(&item.feed_name) && !ranked.contains(&item.feed_id) {
            ranked.push(item.feed_id);
        }
    }

    ranked
}

fn cap_feed_in_ration(ration: &RationFull, feed_id: i64, factor: f64) -> RationFull {
    RationFull {
        ration: ration.ration.clone(),
        items: ration
            .items
            .iter()
            .map(|item| {
                let mut next = item.clone();
                if next.feed_id == feed_id {
                    next.amount_kg = (next.amount_kg * factor).max(0.0);
                    next.is_locked = false;
                }
                next
            })
            .collect(),
    }
}

fn solution_to_ration(solution: &DietSolution, ration: &RationFull, available_feeds: &[Feed]) -> RationFull {
    let mut feed_lookup = HashMap::<i64, Feed>::new();
    for item in &ration.items {
        if let Some(feed) = item.feed.as_ref() {
            feed_lookup.entry(item.feed_id).or_insert_with(|| feed.clone());
        }
    }
    for feed in available_feeds {
        if let Some(feed_id) = feed.id {
            feed_lookup.entry(feed_id).or_insert_with(|| feed.clone());
        }
    }

    let items = solution
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let feed = feed_lookup.get(&item.feed_id)?.clone();
            Some(RationItem {
                id: None,
                ration_id: ration.ration.id.unwrap_or_default(),
                feed_id: item.feed_id,
                feed: Some(feed),
                amount_kg: item.amount_kg,
                is_locked: false,
                sort_order: index as i32,
            })
        })
        .collect();

    RationFull {
        ration: ration.ration.clone(),
        items,
    }
}

fn adequacy_for_solution(solution: &DietSolution, norms: Option<&AnimalNorm>) -> AdequacyBreakdown {
    let Some(norms) = norms else {
        return AdequacyBreakdown {
            overall: 100.0,
            tier1: 100.0,
        };
    };

    let mut tier1_scores = Vec::new();
    let mut tier2_scores = Vec::new();
    let mut tier3_scores = Vec::new();
    let summary = &solution.nutrient_summary;

    if norms.feed_intake_min.is_some() || norms.feed_intake_max.is_some() {
        let intake_key = if norms.species == "cattle" {
            "dry_matter_intake"
        } else {
            "feed_intake"
        };
        if let Some(actual) = actual_metric_value(norms, summary, intake_key) {
            tier1_scores.push(score_metric(
                actual,
                norms.feed_intake_min,
                norms.feed_intake_max,
                None,
            ));
        }
    }

    for key in all_objective_keys(norms) {
        let Some(actual) = actual_metric_value(norms, summary, key) else {
            continue;
        };
        let min = norms.nutrients_min.get(key).copied();
        let max = norms.nutrients_max.get(key).copied();
        let target = norms.nutrients_target.get(key).copied();
        if min.is_none() && max.is_none() && target.is_none() {
            continue;
        }

        let score = score_metric(actual, min, max, target);
        match constraint_tier_for_key(norms, key) {
            ConstraintTier::Tier1 => tier1_scores.push(score),
            ConstraintTier::Tier2 => tier2_scores.push(score),
            ConstraintTier::Tier3 => tier3_scores.push(score),
        }
    }

    let overall = average_scores(&tier1_scores, &tier2_scores, &tier3_scores);

    AdequacyBreakdown {
        overall,
        tier1: average(&tier1_scores),
    }
}

fn score_metric(actual: f64, min: Option<f64>, max: Option<f64>, target: Option<f64>) -> f64 {
    match (min, max, target) {
        (Some(min), Some(max), _) => {
            if actual >= min && actual <= max {
                100.0
            } else if actual < min && min > 0.0 {
                ((actual / min) * 100.0).clamp(0.0, 100.0)
            } else if actual > max && max > 0.0 {
                (100.0 - ((actual - max) / max * 50.0)).clamp(0.0, 100.0)
            } else {
                0.0
            }
        }
        (Some(min), None, _) => {
            if actual >= min {
                100.0
            } else if min > 0.0 {
                ((actual / min) * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            }
        }
        (None, Some(max), _) => {
            if actual <= max {
                100.0
            } else if max > 0.0 {
                (100.0 - ((actual - max) / max * 50.0)).clamp(0.0, 100.0)
            } else {
                0.0
            }
        }
        (None, None, Some(target)) => {
            if target.abs() < f64::EPSILON {
                if actual.abs() < 1e-6 {
                    100.0
                } else {
                    0.0
                }
            } else {
                (100.0 - ((actual - target).abs() / target.abs() * 100.0)).clamp(0.0, 100.0)
            }
        }
        (None, None, None) => 100.0,
    }
}

fn average_scores(tier1: &[f64], tier2: &[f64], tier3: &[f64]) -> f64 {
    let mut combined = Vec::with_capacity(tier1.len() + tier2.len() + tier3.len());
    combined.extend_from_slice(tier1);
    combined.extend_from_slice(tier2);
    combined.extend_from_slice(tier3);
    average(&combined)
}

fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        100.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn candidate_within_equivalence_band(candidate: &CandidateSolution, primary: &CandidateSolution) -> bool {
    let max_cost = if primary.solution.cost_per_day > 0.0 {
        primary.solution.cost_per_day * (1.0 + COST_TOLERANCE_FRACTION)
    } else {
        f64::INFINITY
    };
    // When the primary itself scores below MIN_ACCEPTABLE_SCORE, use a
    // relative floor so that alternatives with similar (low) scores are
    // still accepted rather than being gated by an unreachable absolute.
    let min_score = if primary.adequacy.overall >= MIN_ACCEPTABLE_SCORE {
        (primary.adequacy.overall - SCORE_TOLERANCE_POINTS).max(MIN_ACCEPTABLE_SCORE)
    } else {
        (primary.adequacy.overall - SCORE_TOLERANCE_POINTS).max(0.0)
    };

    candidate.solution.cost_per_day <= max_cost + 1e-6
        && candidate.adequacy.overall + 1e-6 >= min_score
        && candidate.adequacy.tier1 + 1e-6 >= (primary.adequacy.tier1 - SCORE_TOLERANCE_POINTS)
}

fn solution_signature(items: &[OptimizedItem]) -> String {
    let mut entries = items
        .iter()
        .map(|item| format!("{}:{:.3}", item.feed_id, item.amount_kg))
        .collect::<Vec<_>>();
    entries.sort();
    entries.join("|")
}

fn feed_set_signature(items: &[OptimizedItem]) -> String {
    let mut entries = items
        .iter()
        .map(|item| item.feed_id.to_string())
        .collect::<Vec<_>>();
    entries.sort();
    entries.dedup();
    entries.join("|")
}

fn candidate_is_near_duplicate(
    candidate: &CandidateSolution,
    accepted: &[CandidateSolution],
) -> bool {
    accepted.iter().any(|existing| {
        feed_set_containment_ratio(&candidate.solution.items, &existing.solution.items)
            >= NEAR_DUPLICATE_CONTAINMENT
    })
}

fn feed_set_containment_ratio(left: &[OptimizedItem], right: &[OptimizedItem]) -> f64 {
    let left_ids = left.iter().map(|item| item.feed_id).collect::<HashSet<_>>();
    let right_ids = right.iter().map(|item| item.feed_id).collect::<HashSet<_>>();
    let min_size = left_ids.len().min(right_ids.len());

    if min_size == 0 {
        return 0.0;
    }

    let intersection = left_ids.intersection(&right_ids).count();
    intersection as f64 / min_size as f64
}

fn is_discouraged_repeat_feed_name(feed_name: &str) -> bool {
    let normalized = feed_name.to_lowercase();
    DISCOURAGED_REPEAT_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn finalize_candidate_tags(candidates: &mut [CandidateSolution]) {
    if candidates.is_empty() {
        return;
    }

    let lowest_cost = candidates
        .iter()
        .map(|candidate| candidate.solution.cost_per_day)
        .fold(f64::INFINITY, f64::min);
    let widest_mix = candidates
        .iter()
        .map(|candidate| candidate.solution.items.len())
        .max()
        .unwrap_or(0);

    for candidate in candidates {
        if (candidate.solution.cost_per_day - lowest_cost).abs() < 1e-6
            && !candidate.tags.iter().any(|tag| tag == "lowest_cost")
        {
            candidate.tags.push("lowest_cost".to_string());
        }
        if candidate.solution.items.len() == widest_mix
            && widest_mix > 0
            && !candidate.tags.iter().any(|tag| tag == "widest_mix")
        {
            candidate.tags.push("widest_mix".to_string());
        }
        candidate.tags.sort();
        candidate.tags.dedup();
    }
}

fn to_api_solution(candidate: &CandidateSolution) -> AlternativeRationSolution {
    AlternativeRationSolution {
        id: candidate.id.clone(),
        label: candidate.label.clone(),
        feeds: candidate.solution.items.clone(),
        nutrients: candidate.solution.nutrient_summary.clone(),
        adequacy_score: candidate.adequacy.overall,
        cost: candidate.solution.cost_per_day,
        tags: candidate.tags.clone(),
        optimization_status: candidate.solution.optimization_status.clone(),
        applied_strategy: candidate.solution.applied_strategy.clone(),
        warnings: candidate.solution.warnings.clone(),
    }
}

fn build_comparison(
    primary: &AlternativeRationSolution,
    alternatives: &[AlternativeRationSolution],
) -> OptimizationComparison {
    let mut all_solutions = Vec::with_capacity(alternatives.len() + 1);
    all_solutions.push(primary);
    all_solutions.extend(alternatives.iter());

    let mut cost_range = [f64::INFINITY, 0.0];
    let mut score_range = [f64::INFINITY, 0.0];
    let mut feed_sets = Vec::new();

    for solution in &all_solutions {
        cost_range[0] = cost_range[0].min(solution.cost);
        cost_range[1] = cost_range[1].max(solution.cost);
        score_range[0] = score_range[0].min(solution.adequacy_score);
        score_range[1] = score_range[1].max(solution.adequacy_score);

        let feed_set = solution
            .feeds
            .iter()
            .map(|feed| feed.feed_name.clone())
            .collect::<BTreeSet<_>>();
        feed_sets.push(feed_set);
    }

    if !cost_range[0].is_finite() {
        cost_range = [0.0, 0.0];
    }
    if !score_range[0].is_finite() {
        score_range = [0.0, 0.0];
    }

    let common_feeds = feed_sets
        .iter()
        .skip(1)
        .fold(feed_sets.first().cloned().unwrap_or_default(), |acc, feeds| {
            acc.intersection(feeds).cloned().collect()
        })
        .into_iter()
        .collect::<Vec<_>>();

    let differentiators = feed_sets
        .iter()
        .fold(BTreeSet::new(), |mut acc, feeds| {
            acc.extend(feeds.iter().cloned());
            acc
        })
        .difference(&common_feeds.iter().cloned().collect())
        .cloned()
        .collect::<Vec<_>>();

    OptimizationComparison {
        cost_range,
        score_range,
        common_feeds,
        differentiators,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        feeds::Feed,
        rations::{Ration, RationFull, RationItem},
    };
    use std::collections::HashSet;

    fn make_solution(items: Vec<OptimizedItem>) -> DietSolution {
        DietSolution {
            items,
            nutrient_summary: super::super::nutrient_calc::NutrientSummary::default(),
            cost_per_day: 0.0,
            optimization_status: SolutionStatus::Optimal,
            warnings: Vec::new(),
            recommendations: Vec::new(),
            applied_strategy: "priority_tiered_balance".to_string(),
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

    fn make_candidate(id: &str, items: Vec<OptimizedItem>) -> CandidateSolution {
        CandidateSolution {
            id: id.to_string(),
            label: id.to_string(),
            tags: Vec::new(),
            signature: solution_signature(&items),
            feed_set_signature: feed_set_signature(&items),
            solution: make_solution(items),
            adequacy: AdequacyBreakdown {
                overall: 100.0,
                tier1: 100.0,
            },
        }
    }

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

    fn cattle_norms() -> AnimalNorm {
        AnimalNorm {
            id: "cattle_dairy_alternatives".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("energy_eke".to_string(), 10.0),
                ("crude_protein".to_string(), 1650.0),
                ("calcium".to_string(), 60.0),
                ("phosphorus".to_string(), 25.0),
                ("crude_fiber".to_string(), 3000.0),
            ]),
            nutrients_max: HashMap::from([("crude_fiber".to_string(), 6500.0)]),
            feed_intake_min: Some(14.0),
            feed_intake_max: Some(22.0),
            ..Default::default()
        }
    }

    fn sample_library() -> Vec<Feed> {
        vec![
            Feed {
                id: Some(1),
                name_ru: "Hay".to_string(),
                category: "roughage".to_string(),
                dry_matter: Some(88.0),
                energy_oe_cattle: Some(9.0),
                crude_protein: Some(145.0),
                crude_fiber: Some(540.0),
                calcium: Some(8.5),
                phosphorus: Some(2.5),
                price_per_ton: Some(12000.0),
                ..Default::default()
            },
            Feed {
                id: Some(2),
                name_ru: "Corn silage".to_string(),
                category: "silage".to_string(),
                dry_matter: Some(35.0),
                energy_oe_cattle: Some(10.9),
                crude_protein: Some(80.0),
                crude_fiber: Some(430.0),
                calcium: Some(2.3),
                phosphorus: Some(2.0),
                price_per_ton: Some(6000.0),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Barley".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(86.0),
                energy_oe_cattle: Some(12.7),
                crude_protein: Some(115.0),
                crude_fiber: Some(185.0),
                calcium: Some(0.7),
                phosphorus: Some(3.6),
                price_per_ton: Some(16500.0),
                ..Default::default()
            },
            Feed {
                id: Some(4),
                name_ru: "Corn grain".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(86.0),
                energy_oe_cattle: Some(13.4),
                crude_protein: Some(90.0),
                crude_fiber: Some(120.0),
                calcium: Some(0.3),
                phosphorus: Some(2.7),
                price_per_ton: Some(15400.0),
                ..Default::default()
            },
            Feed {
                id: Some(5),
                name_ru: "Soybean meal".to_string(),
                category: "oilseed_meal".to_string(),
                dry_matter: Some(89.0),
                crude_protein: Some(430.0),
                lysine: Some(28.0),
                calcium: Some(3.0),
                phosphorus: Some(6.2),
                price_per_ton: Some(25800.0),
                ..Default::default()
            },
            Feed {
                id: Some(6),
                name_ru: "Sunflower meal".to_string(),
                category: "oilseed_meal".to_string(),
                dry_matter: Some(90.0),
                crude_protein: Some(360.0),
                lysine: Some(12.0),
                calcium: Some(4.0),
                phosphorus: Some(9.0),
                price_per_ton: Some(21200.0),
                ..Default::default()
            },
            Feed {
                id: Some(7),
                name_ru: "Feed chalk".to_string(),
                category: "mineral".to_string(),
                calcium: Some(360.0),
                phosphorus: Some(0.2),
                price_per_ton: Some(5200.0),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn generates_three_or_more_distinct_alternative_solutions() {
        let library = sample_library();
        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Alternatives".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(1, library[0].clone(), 8.0),
                make_item(2, library[1].clone(), 14.0),
                make_item(3, library[2].clone(), 4.5),
                make_item(5, library[4].clone(), 1.6),
                make_item(7, library[6].clone(), 0.12),
            ],
        };

        let result = optimize_with_alternatives(
            &ration,
            OptimizationMode::TieredBalance,
            Some(&cattle_norms()),
            Some(&library),
            Some(4),
        )
        .expect("alternatives");

        let total = 1 + result.alternatives.len();
        assert!((3..=4).contains(&total));
        assert!(result.comparison.cost_range[0] <= result.comparison.cost_range[1]);
        assert!(result.comparison.score_range[0] <= result.comparison.score_range[1]);
        assert!(!result.comparison.common_feeds.is_empty());

        let mut signatures = HashSet::new();
        let mut feed_sets = HashSet::new();
        let all = std::iter::once(&result.primary).chain(result.alternatives.iter());
        for solution in all {
            let signature = solution
                .feeds
                .iter()
                .map(|item| format!("{}:{:.3}", item.feed_id, item.amount_kg))
                .collect::<Vec<_>>()
                .join("|");
            assert!(signatures.insert(signature));

            let mut feed_set = solution
                .feeds
                .iter()
                .map(|item| item.feed_id.to_string())
                .collect::<Vec<_>>();
            feed_set.sort();
            feed_set.dedup();
            assert!(feed_sets.insert(feed_set.join("|")));
        }
        assert!(result
            .alternatives
            .iter()
            .any(|solution| solution.tags.iter().any(|tag| tag == "lowest_cost")));
    }

    #[test]
    fn select_allowed_repeats_delays_discouraged_recurring_feeds() {
        let accepted = vec![
            make_candidate(
                "first",
                vec![
                    OptimizedItem {
                        feed_id: 1,
                        feed_name: "Hay".to_string(),
                        amount_kg: 8.0,
                        dm_kg: 7.0,
                        cost_per_day: 12.0,
                    },
                    OptimizedItem {
                        feed_id: 2,
                        feed_name: "Паприн".to_string(),
                        amount_kg: 0.8,
                        dm_kg: 0.7,
                        cost_per_day: 4.0,
                    },
                ],
            ),
            make_candidate(
                "second",
                vec![
                    OptimizedItem {
                        feed_id: 3,
                        feed_name: "Barley".to_string(),
                        amount_kg: 4.0,
                        dm_kg: 3.4,
                        cost_per_day: 8.0,
                    },
                    OptimizedItem {
                        feed_id: 2,
                        feed_name: "Paprin concentrate".to_string(),
                        amount_kg: 0.6,
                        dm_kg: 0.5,
                        cost_per_day: 3.0,
                    },
                ],
            ),
        ];

        let allowed = select_allowed_repeats(&accepted, 2);

        assert!(allowed.contains(&1) || allowed.contains(&3));
        assert!(!allowed.contains(&2));
    }

    #[test]
    fn rejects_near_duplicate_feed_sets_before_fallback_relaxation() {
        let accepted = vec![make_candidate(
            "accepted",
            vec![
                OptimizedItem {
                    feed_id: 1,
                    feed_name: "Hay".to_string(),
                    amount_kg: 7.0,
                    dm_kg: 6.1,
                    cost_per_day: 10.0,
                },
                OptimizedItem {
                    feed_id: 2,
                    feed_name: "Corn silage".to_string(),
                    amount_kg: 12.0,
                    dm_kg: 4.2,
                    cost_per_day: 8.0,
                },
                OptimizedItem {
                    feed_id: 3,
                    feed_name: "Barley".to_string(),
                    amount_kg: 3.0,
                    dm_kg: 2.5,
                    cost_per_day: 5.0,
                },
                OptimizedItem {
                    feed_id: 4,
                    feed_name: "Soybean meal".to_string(),
                    amount_kg: 1.8,
                    dm_kg: 1.6,
                    cost_per_day: 6.0,
                },
            ],
        )];

        let candidate = make_candidate(
            "candidate",
            vec![
                OptimizedItem {
                    feed_id: 1,
                    feed_name: "Hay".to_string(),
                    amount_kg: 6.8,
                    dm_kg: 6.0,
                    cost_per_day: 9.5,
                },
                OptimizedItem {
                    feed_id: 2,
                    feed_name: "Corn silage".to_string(),
                    amount_kg: 11.5,
                    dm_kg: 4.0,
                    cost_per_day: 7.8,
                },
                OptimizedItem {
                    feed_id: 3,
                    feed_name: "Barley".to_string(),
                    amount_kg: 3.4,
                    dm_kg: 2.8,
                    cost_per_day: 5.4,
                },
                OptimizedItem {
                    feed_id: 4,
                    feed_name: "Soybean meal".to_string(),
                    amount_kg: 1.6,
                    dm_kg: 1.4,
                    cost_per_day: 5.8,
                },
                OptimizedItem {
                    feed_id: 5,
                    feed_name: "Feed chalk".to_string(),
                    amount_kg: 0.1,
                    dm_kg: 0.1,
                    cost_per_day: 0.3,
                },
            ],
        );

        assert!(candidate_is_near_duplicate(&candidate, &accepted));
    }
}
