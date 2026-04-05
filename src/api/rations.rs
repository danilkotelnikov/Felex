//! Rations API handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{ApiError, ApiResponse};
use crate::api::norm_resolution as shared_norm_resolution;
use crate::db::rations::{self, Ration, RationFull};
use crate::diet_engine::{
    AutoPopulatePlan, DietSolution, EconomicAnalysis, NutrientSummary, OptimizationMode,
    RationState, ScreeningReport, SolutionStatus, SolveIntent,
};
use crate::presets::MatchedPresetCategory;
use crate::AppState;

const NOTE_SELECTED_ONLY_USES_BALANCE: &str = "optimize.noteSelectedOnlyUsesBalance";
const NOTE_LIBRARY_STARTS_WITH_REPAIR: &str = "optimize.noteLibraryStartsWithRepair";
const NOTE_STARTER_BUILT: &str = "optimize.noteStarterBuilt";
const NOTE_SELECTED_ONLY_NEEDS_LIBRARY: &str = "optimize.noteSelectedOnlyNeedsLibrary";
const NOTE_LIBRARY_STILL_INSUFFICIENT: &str = "optimize.noteLibraryStillInsufficient";
const NOTE_BEST_SELECTED_ONLY: &str = "optimize.noteBestSelectedOnly";
const NOTE_ALLOW_LIBRARY_COMPLETION: &str = "optimize.noteAllowLibraryCompletion";
const NOTE_BEST_COMPLETE_LIBRARY: &str = "optimize.noteBestCompleteLibrary";
const NOTE_BEST_BUILD_LIBRARY: &str = "optimize.noteBestBuildLibrary";
const NOTE_BALANCED_SELECTED_ONLY: &str = "optimize.noteBalancedSelectedOnly";
const NOTE_BALANCED_COMPLETE_LIBRARY: &str = "optimize.noteBalancedCompleteLibrary";
const NOTE_BALANCED_BUILD_LIBRARY: &str = "optimize.noteBalancedBuildLibrary";
const NOTE_SELECTED_REFERENCE_USED: &str = "optimize.noteSelectedReferenceUsed";

#[derive(Debug, Serialize)]
pub struct PresetCatalogResponse {
    pub categories: Vec<MatchedPresetCategory>,
}

pub async fn list_presets(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<PresetCatalogResponse>>, ApiError> {
    let categories = state.db.with_conn(|conn| {
        let feeds = crate::db::feeds::list_feeds(conn, None, None, Some(5000), None)?;
        Ok(crate::presets::matched_preset_categories(&feeds))
    })?;

    Ok(ApiResponse::new(PresetCatalogResponse { categories }))
}

/// List all rations
pub async fn list_rations(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<Ration>>>, ApiError> {
    let rations = state.db.with_conn(|conn| rations::list_rations(conn))?;

    Ok(ApiResponse::new(rations))
}

/// Get a single ration with items
pub async fn get_ration(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<RationFull>>, (StatusCode, Json<ApiError>)> {
    let ration = state
        .db
        .with_conn(|conn| rations::get_ration(conn, id))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::from(e))))?;

    match ration {
        Some(r) => Ok(ApiResponse::new(r)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "not_found".to_string(),
                message: format!("Ration {} not found", id),
            }),
        )),
    }
}

/// Create ration request
#[derive(Debug, Deserialize)]
pub struct CreateRationRequest {
    pub name: String,
    pub animal_group_id: Option<String>,
    pub animal_count: Option<i32>,
    pub description: Option<String>,
}

/// Create a new ration
pub async fn create_ration(
    State(state): State<AppState>,
    Json(req): Json<CreateRationRequest>,
) -> Result<(StatusCode, Json<ApiResponse<i64>>), ApiError> {
    let ration = Ration {
        name: req.name,
        animal_group_id: req.animal_group_id,
        animal_count: req.animal_count.unwrap_or(1),
        description: req.description,
        ..Default::default()
    };

    let id = state
        .db
        .with_conn(|conn| rations::create_ration(conn, &ration))?;

    Ok((StatusCode::CREATED, ApiResponse::new(id)))
}

/// Update ration request
#[derive(Debug, Deserialize)]
pub struct UpdateRationRequest {
    pub name: Option<String>,
    pub animal_group_id: Option<String>,
    pub animal_count: Option<i32>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub items: Option<Vec<RationItemUpdate>>,
}

#[derive(Debug, Deserialize)]
pub struct RationItemUpdate {
    pub id: Option<i64>,
    pub feed_id: i64,
    pub amount_kg: f64,
    pub is_locked: Option<bool>,
}

/// Update a ration
pub async fn update_ration(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateRationRequest>,
) -> Result<StatusCode, ApiError> {
    state.db.with_conn(|conn| {
        // Get existing ration
        let existing =
            rations::get_ration(conn, id)?.ok_or_else(|| anyhow::anyhow!("Ration not found"))?;

        let updated = Ration {
            id: Some(id),
            name: req.name.unwrap_or(existing.ration.name),
            animal_group_id: req.animal_group_id.or(existing.ration.animal_group_id),
            animal_count: req.animal_count.unwrap_or(existing.ration.animal_count),
            description: req.description.or(existing.ration.description),
            status: req.status.unwrap_or(existing.ration.status),
            created_at: existing.ration.created_at,
            updated_at: None,
        };

        rations::update_ration(conn, id, &updated)?;

        // Update items if provided
        if let Some(items) = req.items {
            // Remove all existing items
            for item in &existing.items {
                if let Some(item_id) = item.id {
                    rations::remove_ration_item(conn, item_id)?;
                }
            }

            // Add new items
            for item in items {
                rations::add_ration_item(
                    conn,
                    id,
                    item.feed_id,
                    item.amount_kg,
                    item.is_locked.unwrap_or(false),
                )?;
            }
        }

        Ok(())
    })?;

    Ok(StatusCode::OK)
}

/// Optimize request
#[derive(Debug, Default, Deserialize)]
pub struct OptimizeRequest {
    pub mode: Option<String>,
    pub intent: Option<String>,
    pub norms: Option<HashMap<String, OptimizeNormRange>>,
    pub norm_preset_id: Option<String>,
    pub animal_properties: Option<OptimizeAnimalProperties>,
    /// When set, restricts the optimizer to only use feeds with these IDs from the library.
    pub available_feed_ids: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OptimizeNormRange {
    pub min: Option<f64>,
    pub target: Option<f64>,
    pub max: Option<f64>,
}

pub type OptimizeAnimalProperties = shared_norm_resolution::ResolveAnimalProperties;

#[derive(Debug, Clone)]
struct BuiltOverrideNorm {
    norm: crate::norms::AnimalNorm,
    #[cfg(test)]
    ignored_keys: Vec<String>,
}

fn infer_species(group_id: &str) -> String {
    if group_id.starts_with("swine") {
        "swine".to_string()
    } else if group_id.starts_with("poultry") {
        "poultry".to_string()
    } else {
        "cattle".to_string()
    }
}

fn infer_production_type(group_id: &str) -> Option<String> {
    if group_id.contains("dairy") {
        Some("dairy".to_string())
    } else if group_id.contains("beef") {
        Some("beef".to_string())
    } else if group_id.contains("finisher") {
        Some("fattening".to_string())
    } else if group_id.contains("sow") {
        Some("breeding".to_string())
    } else if group_id.contains("broiler") {
        Some("broiler".to_string())
    } else if group_id.contains("layer") {
        Some("layer".to_string())
    } else {
        None
    }
}

fn norm_resolve_request(req: &OptimizeRequest) -> shared_norm_resolution::NormResolveRequest {
    shared_norm_resolution::NormResolveRequest {
        norm_preset_id: req.norm_preset_id.clone(),
        animal_properties: req.animal_properties.clone(),
    }
}

fn build_override_norm(
    ration: &RationFull,
    ranges: &HashMap<String, OptimizeNormRange>,
    resolved_group_id: Option<&str>,
    base_norm: Option<&crate::norms::AnimalNorm>,
) -> Option<BuiltOverrideNorm> {
    if ranges.is_empty() {
        return None;
    }

    let group_id = resolved_group_id
        .map(ToOwned::to_owned)
        .or_else(|| ration.ration.animal_group_id.clone())
        .unwrap_or_else(|| "custom".to_string());

    let mut norm = base_norm.cloned().unwrap_or_else(|| {
        crate::norms::get_norms_for_group(&group_id).unwrap_or_else(|| crate::norms::AnimalNorm {
            id: group_id.clone(),
            species: infer_species(&group_id),
            production_type: infer_production_type(&group_id),
            ..Default::default()
        })
    });

    // Preserve the base norm ID so species-specific unit semantics remain intact.
    // For example, swine finisher norms interpret several keys per kg feed.
    if norm.id.is_empty() {
        norm.id = group_id.clone();
    }
    norm.species = if norm.species.is_empty() {
        infer_species(&group_id)
    } else {
        norm.species
    };
    norm.production_type = norm
        .production_type
        .or_else(|| infer_production_type(&group_id));
    norm.source = norm
        .source
        .clone()
        .map(|source| format!("{source}; Felex optimize dialog"))
        .or_else(|| Some("Felex optimize dialog".to_string()));
    norm.notes = norm
        .notes
        .clone()
        .map(|note| format!("{note}; norms selected in UI for optimization"))
        .or_else(|| Some("Norms selected in UI for optimization".to_string()));
    norm.nutrients_min.clear();
    norm.nutrients_target.clear();
    norm.nutrients_max.clear();
    #[cfg(test)]
    let mut ignored_keys = Vec::new();

    for (key, range) in ranges {
        let Some(canonical_key) =
            canonical_optimize_norm_key(norm.species.as_str(), group_id.as_str(), key)
        else {
            #[cfg(test)]
            ignored_keys.push(key.clone());
            continue;
        };

        if let Some(value) = range.min {
            norm.nutrients_min.insert(canonical_key.clone(), value);
        }
        if let Some(value) = range.target {
            norm.nutrients_target.insert(canonical_key.clone(), value);
        }
        if let Some(value) = range.max {
            norm.nutrients_max.insert(canonical_key, value);
        }
    }

    if norm.nutrients_min.is_empty()
        && norm.nutrients_target.is_empty()
        && norm.nutrients_max.is_empty()
    {
        None
    } else {
        #[cfg(test)]
        ignored_keys.sort();
        #[cfg(test)]
        ignored_keys.dedup();
        Some(BuiltOverrideNorm {
            norm,
            #[cfg(test)]
            ignored_keys,
        })
    }
}

fn resolved_norm_group_id(ration: &RationFull, req: &OptimizeRequest) -> String {
    shared_norm_resolution::resolved_norm_group_id(
        ration.ration.animal_group_id.as_deref(),
        &norm_resolve_request(req),
    )
}

fn resolved_default_norm(
    ration: &RationFull,
    req: &OptimizeRequest,
    resolved_group_id: &str,
) -> Option<crate::norms::AnimalNorm> {
    shared_norm_resolution::resolved_default_norm(
        ration.ration.animal_group_id.as_deref(),
        &norm_resolve_request(req),
        Some(resolved_group_id),
    )
}

fn canonical_optimize_norm_key(species: &str, group_id: &str, key: &str) -> Option<String> {
    let normalized = key.trim();

    let canonical = match species {
        "cattle" => normalized,
        "swine" => match normalized {
            "dry_matter_intake" => "feed_intake",
            "lysine" => "lysine_sid",
            "methionine_cystine" => "methionine_cystine_sid",
            // Keep local grower / nursery protein percent overrides aligned with backend semantics.
            "crude_protein_pct" if group_id == "swine_finisher" => "crude_protein",
            _ => normalized,
        },
        "poultry" => match normalized {
            "dry_matter_intake" => "feed_intake",
            "crude_protein" => "crude_protein_pct",
            "lysine" => "lysine_tid_pct",
            "methionine_cystine" => "methionine_cystine_tid_pct",
            "calcium" => "calcium_pct",
            _ => normalized,
        },
        _ => return None,
    };

    let supported = match species {
        "cattle" => matches!(
            canonical,
            "dry_matter_intake"
                | "feed_intake"
                | "energy_eke"
                | "energy_oe_cattle"
                | "crude_protein"
                | "crude_protein_pct"
                | "dig_protein_cattle"
                | "dig_protein_cattle_pct_cp"
                | "lysine"
                | "methionine_cystine"
                | "crude_fiber"
                | "starch"
                | "starch_pct_dm"
                | "sugar"
                | "crude_fat"
                | "calcium"
                | "calcium_pct"
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
                | "vit_e"
                | "ca_p_ratio"
                | "methionine_cystine_lys_ratio"
        ),
        "swine" => matches!(
            canonical,
            "feed_intake"
                | "energy_oe_pig"
                | "crude_protein"
                | "crude_protein_pct"
                | "dig_protein_pig"
                | "lysine"
                | "lysine_sid"
                | "lysine_sid_pct"
                | "methionine_cystine"
                | "methionine_cystine_sid"
                | "methionine_cystine_tid_pct"
                | "crude_fiber"
                | "starch"
                | "sugar"
                | "crude_fat"
                | "calcium"
                | "calcium_pct"
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
                | "vit_e"
                | "ca_p_ratio"
                | "methionine_cystine_lys_ratio"
        ),
        "poultry" => matches!(
            canonical,
            "feed_intake"
                | "energy_oe_poultry"
                | "crude_protein"
                | "crude_protein_pct"
                | "dig_protein_poultry"
                | "lysine"
                | "lysine_tid_pct"
                | "methionine_cystine"
                | "methionine_cystine_tid_pct"
                | "methionine_cystine_lys_ratio"
                | "crude_fiber"
                | "starch"
                | "sugar"
                | "crude_fat"
                | "calcium"
                | "calcium_pct"
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
                | "vit_e"
                | "ca_p_ratio"
        ),
        _ => false,
    };

    supported.then(|| canonical.to_string())
}

fn parse_optimization_mode(mode: Option<&str>) -> OptimizationMode {
    match mode {
        Some("minimize_cost") => OptimizationMode::MinimizeCost,
        Some("single_pass") | Some("quick") => OptimizationMode::SinglePassBalance,
        Some("tiered") => OptimizationMode::TieredBalance,
        Some("repair") | Some("repair_additions") => OptimizationMode::RepairWithAdditions,
        Some("balance") => OptimizationMode::BalanceNutrients,
        Some("fixed") => OptimizationMode::FixedRation,
        _ => OptimizationMode::TieredBalance,
    }
}

fn parse_solve_intent(intent: Option<&str>) -> Option<SolveIntent> {
    match intent {
        Some("selected_only") => Some(SolveIntent::SelectedOnly),
        Some("complete_from_library") => Some(SolveIntent::CompleteFromLibrary),
        Some("build_from_library") => Some(SolveIntent::BuildFromLibrary),
        _ => None,
    }
}

fn classify_ration_state(ration: &RationFull) -> RationState {
    match ration.items.len() {
        0 => RationState::Empty,
        1..=2 => RationState::Sparse,
        _ => RationState::Structured,
    }
}

fn default_solve_intent(state: RationState, mode: OptimizationMode) -> SolveIntent {
    match state {
        RationState::Empty => SolveIntent::BuildFromLibrary,
        RationState::Sparse
            if !matches!(
                mode,
                OptimizationMode::FixedRation | OptimizationMode::MinimizeCost
            ) =>
        {
            SolveIntent::CompleteFromLibrary
        }
        _ if matches!(mode, OptimizationMode::RepairWithAdditions) => {
            SolveIntent::CompleteFromLibrary
        }
        _ => SolveIntent::SelectedOnly,
    }
}

fn effective_mode_for_intent(
    requested_mode: OptimizationMode,
    intent: SolveIntent,
    workflow_notes: &mut Vec<String>,
) -> OptimizationMode {
    match intent {
        SolveIntent::SelectedOnly => {
            if matches!(requested_mode, OptimizationMode::RepairWithAdditions) {
                workflow_notes.push(NOTE_SELECTED_ONLY_USES_BALANCE.to_string());
                OptimizationMode::TieredBalance
            } else {
                requested_mode
            }
        }
        SolveIntent::CompleteFromLibrary | SolveIntent::BuildFromLibrary => {
            if matches!(
                requested_mode,
                OptimizationMode::FixedRation | OptimizationMode::MinimizeCost
            ) {
                workflow_notes.push(NOTE_LIBRARY_STARTS_WITH_REPAIR.to_string());
            }
            OptimizationMode::RepairWithAdditions
        }
    }
}

fn build_working_ration(
    ration: &RationFull,
    intent: SolveIntent,
    state: RationState,
    starter_plan: Option<&AutoPopulatePlan>,
) -> (RationFull, bool, Vec<String>) {
    let mut workflow_notes = Vec::new();
    let Some(plan) = starter_plan else {
        return (ration.clone(), false, workflow_notes);
    };

    if !plan.notes.is_empty() {
        workflow_notes.extend(plan.notes.iter().cloned());
    }

    let starter_items = crate::diet_engine::auto_populate::plan_to_ration_items(
        plan,
        ration.ration.id.unwrap_or_default(),
    );
    if starter_items.is_empty() {
        return (ration.clone(), false, workflow_notes);
    }

    match intent {
        SolveIntent::BuildFromLibrary => {
            workflow_notes.push(NOTE_STARTER_BUILT.to_string());
            (
                RationFull {
                    ration: ration.ration.clone(),
                    items: starter_items,
                },
                true,
                workflow_notes,
            )
        }
        SolveIntent::CompleteFromLibrary
            if matches!(state, RationState::Empty | RationState::Sparse) =>
        {
            let mut merged_items = ration.items.clone();
            let existing_feed_ids: std::collections::HashSet<i64> =
                merged_items.iter().map(|item| item.feed_id).collect();
            let mut added = 0usize;
            for starter_item in starter_items {
                if !existing_feed_ids.contains(&starter_item.feed_id) {
                    merged_items.push(starter_item);
                    added += 1;
                }
            }

            if added > 0 {
                workflow_notes.push(format!(
                    "Completed the current ration with {} starter feed{} from the library.",
                    added,
                    if added == 1 { "" } else { "s" }
                ));
                (
                    RationFull {
                        ration: ration.ration.clone(),
                        items: merged_items,
                    },
                    true,
                    workflow_notes,
                )
            } else {
                (ration.clone(), false, workflow_notes)
            }
        }
        _ => (ration.clone(), false, workflow_notes),
    }
}

/// Optimize a ration
pub async fn optimize_ration(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<OptimizeRequest>,
) -> Result<Json<ApiResponse<DietSolution>>, ApiError> {
    let mode = parse_optimization_mode(req.mode.as_deref());
    let requested_intent = parse_solve_intent(req.intent.as_deref());

    let available_feed_ids = req.available_feed_ids.clone();
    let solution = state.db.with_conn(|conn| {
        let ration =
            rations::get_ration(conn, id)?.ok_or_else(|| anyhow::anyhow!("Ration not found"))?;
        let all_feeds = crate::db::feeds::list_feeds(conn, None, None, Some(5000), None)?;
        let feeds = if let Some(ref allowed) = available_feed_ids {
            let allowed_set: std::collections::HashSet<i64> = allowed.iter().copied().collect();
            // Always include feeds already in the ration
            let ration_feed_ids: std::collections::HashSet<i64> = ration.items.iter().map(|i| i.feed_id).collect();
            all_feeds.into_iter().filter(|f| f.id.map_or(false, |fid| allowed_set.contains(&fid) || ration_feed_ids.contains(&fid))).collect()
        } else {
            all_feeds
        };
        let resolved_group_id = resolved_norm_group_id(&ration, &req);
        let default_norm = resolved_default_norm(&ration, &req, &resolved_group_id);

        let override_norm = req.norms.as_ref().and_then(|ranges| {
            build_override_norm(
                &ration,
                ranges,
                Some(&resolved_group_id),
                default_norm.as_ref(),
            )
        });
        let effective_norm = override_norm
            .as_ref()
            .map(|built| &built.norm)
            .or(default_norm.as_ref());
        let ration_state = classify_ration_state(&ration);
        let effective_intent =
            requested_intent.unwrap_or_else(|| default_solve_intent(ration_state, mode));

        if matches!(effective_intent, SolveIntent::BuildFromLibrary) {
            if let Some(norm) = effective_norm {
                if let Some(mut constructed) =
                    crate::diet_engine::construct_ration_from_library(&ration, norm, &feeds)?
                {
                    constructed.solve_intent = Some(effective_intent);
                    constructed.ration_state = Some(ration_state);
                    if let Some(selected) = starter_plan_note_for_norm_source(req.norms.as_ref()) {
                        constructed.workflow_notes.push(selected);
                    }
                    if matches!(
                        constructed.optimization_status,
                        SolutionStatus::Optimal | SolutionStatus::Feasible
                    ) {
                        let constructed_ration =
                            ration_from_solution(&ration, &constructed, &feeds);
                        if let Ok(alt_result) =
                            crate::diet_engine::optimizer::optimize_with_alternatives(
                                &constructed_ration,
                                OptimizationMode::FixedRation,
                                Some(norm),
                                Some(&feeds),
                                Some(4),
                            )
                        {
                            constructed.alternatives = alt_result.alternatives;
                        }
                    }
                    return Ok(constructed);
                }
            }
        }

        let starter_plan = if matches!(effective_intent, SolveIntent::SelectedOnly) {
            None
        } else {
            Some(crate::diet_engine::auto_populate::build_auto_populate_plan(
                ration.ration.animal_group_id.as_deref(),
                effective_norm,
                &feeds,
            ))
        };
        let (working_ration, auto_populated, mut workflow_notes) = build_working_ration(
            &ration,
            effective_intent,
            ration_state,
            starter_plan.as_ref(),
        );
        let effective_mode = effective_mode_for_intent(mode, effective_intent, &mut workflow_notes);

        let mut solution = crate::diet_engine::optimize_ration_with_library(
            &working_ration,
            effective_mode,
            override_norm.as_ref().map(|built| &built.norm),
            Some(&feeds),
        )?;

        if let Some(norm) = effective_norm {
            if matches!(
                solution.optimization_status,
                SolutionStatus::Infeasible | SolutionStatus::Unbounded | SolutionStatus::Error
            ) {
                let screening = crate::diet_engine::screening::screen_current_feed_set(
                    &working_ration.items,
                    &feeds,
                    norm,
                );
                solution.recommendations = screening.recommendations;
                match effective_intent {
                    SolveIntent::SelectedOnly => {
                        workflow_notes.push(NOTE_SELECTED_ONLY_NEEDS_LIBRARY.to_string())
                    }
                    SolveIntent::CompleteFromLibrary | SolveIntent::BuildFromLibrary => {
                        workflow_notes.push(NOTE_LIBRARY_STILL_INSUFFICIENT.to_string())
                    }
                }
            } else if solution.best_achievable {
                match effective_intent {
                    SolveIntent::SelectedOnly => {
                        let screening = crate::diet_engine::screening::screen_current_feed_set(
                            &working_ration.items,
                            &feeds,
                            norm,
                        );
                        solution.recommendations = screening.recommendations;
                        workflow_notes.push(NOTE_BEST_SELECTED_ONLY.to_string());
                        workflow_notes.push(NOTE_ALLOW_LIBRARY_COMPLETION.to_string());
                    }
                    SolveIntent::CompleteFromLibrary => {
                        solution.recommendations.clear();
                        workflow_notes.push(NOTE_BEST_COMPLETE_LIBRARY.to_string());
                    }
                    SolveIntent::BuildFromLibrary => {
                        solution.recommendations.clear();
                        workflow_notes.push(NOTE_BEST_BUILD_LIBRARY.to_string());
                    }
                }
            } else {
                solution.recommendations.clear();
                match effective_intent {
                    SolveIntent::SelectedOnly => {
                        workflow_notes.push(NOTE_BALANCED_SELECTED_ONLY.to_string())
                    }
                    SolveIntent::CompleteFromLibrary => {
                        workflow_notes.push(NOTE_BALANCED_COMPLETE_LIBRARY.to_string())
                    }
                    SolveIntent::BuildFromLibrary => {
                        workflow_notes.push(NOTE_BALANCED_BUILD_LIBRARY.to_string())
                    }
                }
            }
            if let Some(note) = auto_added_feed_summary_note(&solution) {
                workflow_notes.push(note);
            }
        }
        solution.auto_populated = auto_populated;
        if auto_populated {
            solution.applied_strategy = format!("{}+starter_plan", solution.applied_strategy);
        }
        solution.solve_intent = Some(effective_intent);
        solution.ration_state = Some(ration_state);
        solution.workflow_notes = workflow_notes;

        if matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ) {
            if let Ok(alt_result) = crate::diet_engine::optimizer::optimize_with_alternatives(
                &working_ration,
                effective_mode,
                effective_norm,
                Some(&feeds),
                Some(4),
            ) {
                solution.alternatives = alt_result.alternatives;
            }
        }

        Ok(solution)
    })?;

    Ok(ApiResponse::new(solution))
}

fn ration_from_solution(
    template: &RationFull,
    solution: &DietSolution,
    feeds: &[crate::db::feeds::Feed],
) -> RationFull {
    let mut feed_lookup = std::collections::HashMap::<i64, crate::db::feeds::Feed>::new();
    for item in &template.items {
        if let Some(feed) = item.feed.as_ref() {
            feed_lookup
                .entry(item.feed_id)
                .or_insert_with(|| feed.clone());
        }
    }
    for feed in feeds {
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
            Some(crate::db::rations::RationItem {
                id: None,
                ration_id: template.ration.id.unwrap_or_default(),
                feed_id: item.feed_id,
                feed: Some(feed),
                amount_kg: item.amount_kg,
                is_locked: false,
                sort_order: index as i32,
            })
        })
        .collect();

    RationFull {
        ration: template.ration.clone(),
        items,
    }
}

/// Generate alternative ration solutions for comparison.
pub async fn optimize_ration_alternatives(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<OptimizeRequest>,
) -> Result<Json<ApiResponse<crate::diet_engine::optimizer::OptimizationResult>>, ApiError> {
    let mode = parse_optimization_mode(req.mode.as_deref());
    let requested_intent = parse_solve_intent(req.intent.as_deref());

    let available_feed_ids = req.available_feed_ids.clone();
    let result = state.db.with_conn(|conn| {
        let ration =
            rations::get_ration(conn, id)?.ok_or_else(|| anyhow::anyhow!("Ration not found"))?;
        let all_feeds = crate::db::feeds::list_feeds(conn, None, None, Some(5000), None)?;
        let feeds = if let Some(ref allowed) = available_feed_ids {
            let allowed_set: std::collections::HashSet<i64> = allowed.iter().copied().collect();
            let ration_feed_ids: std::collections::HashSet<i64> = ration.items.iter().map(|i| i.feed_id).collect();
            all_feeds.into_iter().filter(|f| f.id.map_or(false, |fid| allowed_set.contains(&fid) || ration_feed_ids.contains(&fid))).collect()
        } else {
            all_feeds
        };
        let resolved_group_id = resolved_norm_group_id(&ration, &req);
        let default_norm = resolved_default_norm(&ration, &req, &resolved_group_id);
        let override_norm = req.norms.as_ref().and_then(|ranges| {
            build_override_norm(
                &ration,
                ranges,
                Some(&resolved_group_id),
                default_norm.as_ref(),
            )
        });
        let effective_norm = override_norm
            .as_ref()
            .map(|built| &built.norm)
            .or(default_norm.as_ref());
        let ration_state = classify_ration_state(&ration);
        let effective_intent =
            requested_intent.unwrap_or_else(|| default_solve_intent(ration_state, mode));

        if matches!(effective_intent, SolveIntent::BuildFromLibrary) {
            if let Some(norm) = effective_norm {
                if let Some(constructed) =
                    crate::diet_engine::construct_ration_from_library(&ration, norm, &feeds)?
                {
                    let constructed_ration = ration_from_solution(&ration, &constructed, &feeds);
                    return crate::diet_engine::optimizer::optimize_with_alternatives(
                        &constructed_ration,
                        OptimizationMode::FixedRation,
                        Some(norm),
                        Some(&feeds),
                        Some(5),
                    );
                }
            }
        }

        let starter_plan = if matches!(effective_intent, SolveIntent::SelectedOnly) {
            None
        } else {
            Some(crate::diet_engine::auto_populate::build_auto_populate_plan(
                ration.ration.animal_group_id.as_deref(),
                effective_norm,
                &feeds,
            ))
        };
        let (working_ration, _, mut workflow_notes) = build_working_ration(
            &ration,
            effective_intent,
            ration_state,
            starter_plan.as_ref(),
        );
        let effective_mode = effective_mode_for_intent(mode, effective_intent, &mut workflow_notes);

        crate::diet_engine::optimizer::optimize_with_alternatives(
            &working_ration,
            effective_mode,
            effective_norm,
            Some(&feeds),
            Some(5),
        )
    })?;

    Ok(ApiResponse::new(result))
}

fn starter_plan_note_for_norm_source(
    selected_norms: Option<&HashMap<String, OptimizeNormRange>>,
) -> Option<String> {
    if selected_norms.is_some() {
        Some(NOTE_SELECTED_REFERENCE_USED.to_string())
    } else {
        None
    }
}

fn auto_added_feed_summary_note(solution: &DietSolution) -> Option<String> {
    let count = solution.auto_added_feeds.len();
    if count == 0 {
        return None;
    }

    Some(format!(
        "Auto-added {count} feed{} from the library. Review the added-feed notes below for the repair roles.",
        if count == 1 { "" } else { "s" }
    ))
}

/// Build a starter ration from the library for an empty or incomplete ration.
pub async fn auto_populate_ration(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    req: Option<Json<OptimizeRequest>>,
) -> Result<Json<ApiResponse<AutoPopulatePlan>>, ApiError> {
    let req = req.map(|Json(req)| req).unwrap_or_default();

    let plan = state.db.with_conn(|conn| {
        let ration =
            rations::get_ration(conn, id)?.ok_or_else(|| anyhow::anyhow!("Ration not found"))?;
        let feeds = crate::db::feeds::list_feeds(conn, None, None, Some(5000), None)?;
        let resolved_group_id = resolved_norm_group_id(&ration, &req);
        let norms = resolved_default_norm(&ration, &req, &resolved_group_id);

        Ok(crate::diet_engine::auto_populate::build_auto_populate_plan(
            Some(resolved_group_id.as_str()),
            norms.as_ref(),
            &feeds,
        ))
    })?;

    Ok(ApiResponse::new(plan))
}

/// Screen the current feed set and recommend additions for unmet norms.
pub async fn screen_ration(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<OptimizeRequest>,
) -> Result<Json<ApiResponse<ScreeningReport>>, ApiError> {
    let report = state.db.with_conn(|conn| {
        let ration =
            rations::get_ration(conn, id)?.ok_or_else(|| anyhow::anyhow!("Ration not found"))?;
        let feeds = crate::db::feeds::list_feeds(conn, None, None, Some(5000), None)?;
        let resolved_group_id = resolved_norm_group_id(&ration, &req);
        let default_norm = resolved_default_norm(&ration, &req, &resolved_group_id);
        let override_norm = req.norms.as_ref().and_then(|ranges| {
            build_override_norm(
                &ration,
                ranges,
                Some(&resolved_group_id),
                default_norm.as_ref(),
            )
        });
        let effective_norm = override_norm
            .as_ref()
            .map(|built| &built.norm)
            .or(default_norm.as_ref())
            .ok_or_else(|| anyhow::anyhow!("No norms available for screening"))?;

        Ok(crate::diet_engine::screening::screen_current_feed_set(
            &ration.items,
            &feeds,
            effective_norm,
        ))
    })?;

    Ok(ApiResponse::new(report))
}

#[cfg(test)]
mod tests {
    use super::{
        build_override_norm, build_working_ration, canonical_optimize_norm_key,
        default_solve_intent, effective_mode_for_intent, parse_optimization_mode,
        parse_solve_intent, resolved_default_norm, resolved_norm_group_id,
        OptimizeAnimalProperties, OptimizeNormRange, OptimizeRequest,
        NOTE_SELECTED_ONLY_USES_BALANCE, NOTE_STARTER_BUILT,
    };
    use crate::{
        api::norm_resolution::backend_group_for_norm_preset,
        db::{
            feeds::Feed,
            feeds::insert_feed,
            Database,
            rations::{Ration, RationFull, RationItem},
        },
        diet_engine::{
            self, feed_groups::FeedGroup, AutoPopulateItem, AutoPopulatePlan, OptimizationMode,
            RationState, SolutionStatus, SolveIntent,
        },
        AppConfig, AppState,
        norms::AnimalNorm,
    };
    use axum::Json;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use tokio::sync::RwLock;

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

    fn cattle_test_norms() -> AnimalNorm {
        AnimalNorm {
            id: "cattle_dairy_test".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([
                ("energy_eke".to_string(), 9.5),
                ("crude_protein".to_string(), 1600.0),
                ("calcium".to_string(), 60.0),
                ("phosphorus".to_string(), 24.0),
                ("crude_fiber_pct".to_string(), 28.0),
            ]),
            nutrients_max: HashMap::from([("crude_fiber_pct".to_string(), 45.0)]),
            feed_intake_min: Some(12.0),
            feed_intake_max: Some(18.0),
            ..Default::default()
        }
    }

    fn test_state() -> AppState {
        let db = Database::new(":memory:").unwrap();
        db.run_migrations().unwrap();

        let config = AppConfig {
            database_path: "test.db".to_string(),
            ..AppConfig::default()
        };

        AppState {
            db: Arc::new(db),
            config: Arc::new(RwLock::new(config)),
        }
    }

    #[test]
    fn parses_legacy_balance_alias() {
        assert!(matches!(
            parse_optimization_mode(Some("balance")),
            OptimizationMode::BalanceNutrients
        ));
        assert!(matches!(
            parse_optimization_mode(Some("tiered")),
            OptimizationMode::TieredBalance
        ));
    }

    #[test]
    fn defaults_unknown_modes_to_tiered() {
        assert!(matches!(
            parse_optimization_mode(None),
            OptimizationMode::TieredBalance
        ));
        assert!(matches!(
            parse_optimization_mode(Some("unknown")),
            OptimizationMode::TieredBalance
        ));
    }

    #[test]
    fn parses_single_pass_mode() {
        assert!(matches!(
            parse_optimization_mode(Some("single_pass")),
            OptimizationMode::SinglePassBalance
        ));
    }

    #[test]
    fn parses_repair_mode() {
        assert!(matches!(
            parse_optimization_mode(Some("repair")),
            OptimizationMode::RepairWithAdditions
        ));
    }

    #[test]
    fn parses_solve_intent() {
        assert_eq!(
            parse_solve_intent(Some("build_from_library")),
            Some(SolveIntent::BuildFromLibrary)
        );
        assert_eq!(
            parse_solve_intent(Some("complete_from_library")),
            Some(SolveIntent::CompleteFromLibrary)
        );
        assert_eq!(parse_solve_intent(Some("unknown")), None);
    }

    #[test]
    fn defaults_sparse_rations_to_library_completion() {
        assert_eq!(
            default_solve_intent(RationState::Sparse, OptimizationMode::TieredBalance),
            SolveIntent::CompleteFromLibrary
        );
        assert_eq!(
            default_solve_intent(RationState::Empty, OptimizationMode::TieredBalance),
            SolveIntent::BuildFromLibrary
        );
        assert_eq!(
            default_solve_intent(RationState::Structured, OptimizationMode::TieredBalance),
            SolveIntent::SelectedOnly
        );
    }

    #[test]
    fn selected_only_requested_repair_keeps_current_feed_set() {
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
            ..Default::default()
        };
        let barley = Feed {
            id: Some(2),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(12.8),
            crude_protein: Some(115.0),
            crude_fiber: Some(180.0),
            phosphorus: Some(3.6),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(3),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(89.0),
            crude_protein: Some(430.0),
            lysine: Some(28.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(4),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            calcium: Some(360.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Selected only".to_string(),
                ..Default::default()
            },
            items: vec![
                make_item(1, hay.clone(), 9.0),
                make_item(2, barley.clone(), 4.0),
            ],
        };
        let starter_plan = AutoPopulatePlan {
            items: vec![
                AutoPopulateItem {
                    feed: soy.clone(),
                    amount_kg: 1.0,
                    group: FeedGroup::Protein,
                    reason: "starter protein".to_string(),
                },
                AutoPopulateItem {
                    feed: chalk.clone(),
                    amount_kg: 0.1,
                    group: FeedGroup::Mineral,
                    reason: "starter mineral".to_string(),
                },
            ],
            notes: vec!["starter note".to_string()],
        };

        let (working_ration, auto_populated, mut workflow_notes) = build_working_ration(
            &ration,
            SolveIntent::SelectedOnly,
            RationState::Sparse,
            Some(&starter_plan),
        );
        assert!(!auto_populated);
        assert_eq!(working_ration.items.len(), ration.items.len());
        assert!(working_ration
            .items
            .iter()
            .all(|item| item.feed_id == 1 || item.feed_id == 2));

        let effective_mode = effective_mode_for_intent(
            OptimizationMode::RepairWithAdditions,
            SolveIntent::SelectedOnly,
            &mut workflow_notes,
        );
        assert!(matches!(effective_mode, OptimizationMode::TieredBalance));
        assert!(workflow_notes
            .iter()
            .any(|note| note == NOTE_SELECTED_ONLY_USES_BALANCE));

        let solution = diet_engine::optimize_ration_with_library(
            &working_ration,
            effective_mode,
            Some(&cattle_test_norms()),
            Some(&[hay, barley, soy, chalk]),
        )
        .unwrap();

        let original_ids = HashSet::from([1_i64, 2_i64]);
        assert!(solution.auto_added_feeds.is_empty());
        assert!(solution
            .items
            .iter()
            .all(|item| original_ids.contains(&item.feed_id)));
    }

    #[test]
    fn complete_from_library_merges_starter_items_for_single_feed_ration() {
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
            ..Default::default()
        };
        let barley = Feed {
            id: Some(2),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_cattle: Some(12.8),
            crude_protein: Some(115.0),
            crude_fiber: Some(180.0),
            calcium: Some(0.6),
            phosphorus: Some(3.6),
            ..Default::default()
        };
        let soy = Feed {
            id: Some(3),
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            dry_matter: Some(89.0),
            crude_protein: Some(430.0),
            lysine: Some(28.0),
            calcium: Some(3.0),
            phosphorus: Some(6.5),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(4),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            calcium: Some(360.0),
            ..Default::default()
        };

        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Single feed".to_string(),
                ..Default::default()
            },
            items: vec![make_item(1, hay.clone(), 9.0)],
        };
        let starter_plan = AutoPopulatePlan {
            items: vec![
                AutoPopulateItem {
                    feed: hay.clone(),
                    amount_kg: 8.0,
                    group: FeedGroup::Roughage,
                    reason: "starter roughage".to_string(),
                },
                AutoPopulateItem {
                    feed: barley.clone(),
                    amount_kg: 4.0,
                    group: FeedGroup::Concentrate,
                    reason: "starter concentrate".to_string(),
                },
                AutoPopulateItem {
                    feed: soy.clone(),
                    amount_kg: 1.0,
                    group: FeedGroup::Protein,
                    reason: "starter protein".to_string(),
                },
                AutoPopulateItem {
                    feed: chalk.clone(),
                    amount_kg: 0.1,
                    group: FeedGroup::Mineral,
                    reason: "starter mineral".to_string(),
                },
            ],
            notes: vec!["starter note".to_string()],
        };

        let (working_ration, auto_populated, mut workflow_notes) = build_working_ration(
            &ration,
            SolveIntent::CompleteFromLibrary,
            RationState::Sparse,
            Some(&starter_plan),
        );
        assert!(auto_populated);
        assert_eq!(working_ration.items.len(), 4);
        assert!(working_ration.items.iter().any(|item| item.feed_id == 2));
        assert!(working_ration.items.iter().any(|item| item.feed_id == 3));
        assert!(working_ration.items.iter().any(|item| item.feed_id == 4));
        assert!(workflow_notes
            .iter()
            .any(|note| note.contains("Completed the current ration")));

        let effective_mode = effective_mode_for_intent(
            OptimizationMode::TieredBalance,
            SolveIntent::CompleteFromLibrary,
            &mut workflow_notes,
        );
        assert!(matches!(
            effective_mode,
            OptimizationMode::RepairWithAdditions
        ));

        let solution = diet_engine::optimize_ration_with_library(
            &working_ration,
            effective_mode,
            Some(&cattle_test_norms()),
            Some(&[hay, barley, soy, chalk]),
        )
        .unwrap();

        assert!(matches!(
            solution.optimization_status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ));
        assert!(solution.items.len() >= 3);
        assert!(solution.items.iter().any(|item| item.feed_id != 1));
    }

    #[test]
    fn build_from_library_uses_starter_items_for_empty_ration_fallback() {
        let hay = Feed {
            id: Some(1),
            name_ru: "Hay".to_string(),
            category: "roughage".to_string(),
            ..Default::default()
        };
        let barley = Feed {
            id: Some(2),
            name_ru: "Barley".to_string(),
            category: "grain".to_string(),
            ..Default::default()
        };
        let chalk = Feed {
            id: Some(3),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            ..Default::default()
        };

        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Blank ration".to_string(),
                ..Default::default()
            },
            items: vec![],
        };
        let starter_plan = AutoPopulatePlan {
            items: vec![
                AutoPopulateItem {
                    feed: hay,
                    amount_kg: 8.0,
                    group: FeedGroup::Roughage,
                    reason: "starter roughage".to_string(),
                },
                AutoPopulateItem {
                    feed: barley,
                    amount_kg: 4.0,
                    group: FeedGroup::Concentrate,
                    reason: "starter concentrate".to_string(),
                },
                AutoPopulateItem {
                    feed: chalk,
                    amount_kg: 0.1,
                    group: FeedGroup::Mineral,
                    reason: "starter mineral".to_string(),
                },
            ],
            notes: vec![],
        };

        let (working_ration, auto_populated, workflow_notes) = build_working_ration(
            &ration,
            SolveIntent::BuildFromLibrary,
            RationState::Empty,
            Some(&starter_plan),
        );

        assert!(auto_populated);
        assert_eq!(working_ration.items.len(), 3);
        assert!(workflow_notes.iter().any(|note| note == NOTE_STARTER_BUILT));
    }

    #[test]
    fn canonicalizes_cattle_reference_keys_and_ignores_unmodeled_ones() {
        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Cattle override".to_string(),
                ..Default::default()
            },
            items: Vec::new(),
        };
        let ranges = HashMap::from([
            (
                "crude_fiber".to_string(),
                OptimizeNormRange {
                    min: Some(28.0),
                    target: None,
                    max: Some(35.0),
                },
            ),
            (
                "crude_fat".to_string(),
                OptimizeNormRange {
                    min: Some(130.0),
                    target: None,
                    max: None,
                },
            ),
            (
                "dig_protein_cattle".to_string(),
                OptimizeNormRange {
                    min: Some(1500.0),
                    target: Some(1650.0),
                    max: None,
                },
            ),
        ]);

        let built = build_override_norm(&ration, &ranges, Some("cattle_dairy_early_lact"), None)
            .expect("override norm should exist");

        assert_eq!(built.norm.id, "cattle_dairy_early_lact");
        assert_eq!(built.norm.nutrients_min.get("crude_fiber"), Some(&28.0));
        assert_eq!(built.norm.nutrients_max.get("crude_fiber"), Some(&35.0));
        assert_eq!(
            built.norm.nutrients_target.get("dig_protein_cattle"),
            Some(&1650.0)
        );
        assert_eq!(
            built.norm.nutrients_min.get("crude_fat"),
            Some(&130.0)
        );
        assert!(built.ignored_keys.is_empty());
    }

    #[test]
    fn canonicalizes_swine_and_poultry_override_keys() {
        assert_eq!(
            canonical_optimize_norm_key("cattle", "cattle_dairy_early_lact", "crude_fiber"),
            Some("crude_fiber".to_string())
        );
        assert_eq!(
            canonical_optimize_norm_key("swine", "swine_finisher", "phosphorus"),
            Some("phosphorus".to_string())
        );
        assert_eq!(
            canonical_optimize_norm_key("swine", "swine_finisher", "lysine"),
            Some("lysine_sid".to_string())
        );
        assert_eq!(
            canonical_optimize_norm_key("swine", "swine_finisher", "magnesium"),
            Some("magnesium".to_string())
        );
        assert_eq!(
            canonical_optimize_norm_key("poultry", "poultry_broiler", "crude_protein"),
            Some("crude_protein_pct".to_string())
        );
        assert_eq!(
            canonical_optimize_norm_key("poultry", "poultry_broiler", "calcium"),
            Some("calcium_pct".to_string())
        );
        assert_eq!(
            canonical_optimize_norm_key("poultry", "poultry_broiler", "phosphorus"),
            Some("phosphorus".to_string())
        );
        assert_eq!(
            canonical_optimize_norm_key("poultry", "poultry_broiler", "vit_b1"),
            None
        );
    }

    #[test]
    fn swine_override_keeps_finisher_semantic_id() {
        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("swine_finisher".to_string()),
                animal_count: 1,
                name: "Swine override".to_string(),
                ..Default::default()
            },
            items: Vec::new(),
        };
        let ranges = HashMap::from([
            (
                "crude_protein".to_string(),
                OptimizeNormRange {
                    min: Some(250.0),
                    target: Some(280.0),
                    max: None,
                },
            ),
            (
                "lysine".to_string(),
                OptimizeNormRange {
                    min: Some(14.0),
                    target: Some(16.0),
                    max: None,
                },
            ),
        ]);

        let built = build_override_norm(&ration, &ranges, Some("swine_finisher"), None)
            .expect("override norm should exist");

        assert_eq!(built.norm.id, "swine_finisher");
        assert_eq!(built.norm.nutrients_min.get("crude_protein"), Some(&250.0));
        assert_eq!(built.norm.nutrients_target.get("lysine_sid"), Some(&16.0));
    }

    #[test]
    fn maps_known_frontend_presets_to_backend_groups() {
        assert_eq!(
            backend_group_for_norm_preset("cattle_dairy_35"),
            Some("cattle_dairy_fresh")
        );
        assert_eq!(
            backend_group_for_norm_preset("swine_starter"),
            Some("swine_piglet_nursery")
        );
        assert_eq!(
            backend_group_for_norm_preset("poultry_broiler_starter"),
            Some("poultry_broiler_starter")
        );
    }

    #[test]
    fn resolved_norm_group_uses_preset_for_high_yield_dairy() {
        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Dairy".to_string(),
                ..Default::default()
            },
            items: Vec::new(),
        };
        let req = OptimizeRequest {
            mode: None,
            intent: None,
            norms: None,
            norm_preset_id: Some("cattle_dairy_35".to_string()),
            animal_properties: Some(OptimizeAnimalProperties {
                species: Some("cattle".to_string()),
                production_type: Some("dairy".to_string()),
                breed: Some("Голштинская".to_string()),
                sex: Some("female".to_string()),
                live_weight_kg: Some(640.0),
                age_from_days: None,
                age_to_days: None,
                milk_yield_kg: Some(35.0),
                milk_fat_pct: Some(3.7),
                daily_gain_g: None,
                egg_production_per_year: None,
                litter_size: None,
                reproductive_stage: None,
            }),
            ..Default::default()
        };

        assert_eq!(resolved_norm_group_id(&ration, &req), "cattle_dairy_fresh");
    }

    #[test]
    fn resolved_norm_group_uses_animal_context_for_broiler_age() {
        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("poultry_broiler".to_string()),
                animal_count: 1,
                name: "Broiler".to_string(),
                ..Default::default()
            },
            items: Vec::new(),
        };
        let req = OptimizeRequest {
            mode: None,
            intent: None,
            norms: None,
            norm_preset_id: None,
            animal_properties: Some(OptimizeAnimalProperties {
                species: Some("poultry".to_string()),
                production_type: Some("broiler".to_string()),
                breed: Some("Кобб 500".to_string()),
                sex: Some("mixed".to_string()),
                live_weight_kg: Some(0.25),
                age_from_days: Some(0),
                age_to_days: Some(10),
                milk_yield_kg: None,
                milk_fat_pct: None,
                daily_gain_g: Some(30.0),
                egg_production_per_year: None,
                litter_size: None,
                reproductive_stage: None,
            }),
            ..Default::default()
        };

        assert_eq!(
            resolved_norm_group_id(&ration, &req),
            "poultry_broiler_starter"
        );
    }

    #[test]
    fn resolved_norm_group_uses_animal_context_for_beef_weight() {
        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("cattle_beef".to_string()),
                animal_count: 1,
                name: "Beef".to_string(),
                ..Default::default()
            },
            items: Vec::new(),
        };
        let req = OptimizeRequest {
            mode: None,
            intent: None,
            norms: None,
            norm_preset_id: None,
            animal_properties: Some(OptimizeAnimalProperties {
                species: Some("cattle".to_string()),
                production_type: Some("beef".to_string()),
                breed: Some("Абердин-ангусская".to_string()),
                sex: Some("male".to_string()),
                live_weight_kg: Some(250.0),
                age_from_days: Some(180),
                age_to_days: Some(300),
                milk_yield_kg: None,
                milk_fat_pct: None,
                daily_gain_g: Some(850.0),
                egg_production_per_year: None,
                litter_size: None,
                reproductive_stage: None,
            }),
            ..Default::default()
        };

        assert_eq!(resolved_norm_group_id(&ration, &req), "cattle_beef_stocker");
    }

    #[test]
    fn override_norm_preserves_dynamic_feed_intake_window() {
        let ration = RationFull {
            ration: Ration {
                id: Some(1),
                animal_group_id: Some("cattle_dairy".to_string()),
                animal_count: 1,
                name: "Dynamic dairy".to_string(),
                ..Default::default()
            },
            items: Vec::new(),
        };
        let req = OptimizeRequest {
            mode: None,
            intent: None,
            norms: None,
            norm_preset_id: Some("cattle_dairy_35".to_string()),
            animal_properties: Some(OptimizeAnimalProperties {
                species: Some("cattle".to_string()),
                production_type: Some("dairy".to_string()),
                breed: Some("Голштинская".to_string()),
                sex: Some("female".to_string()),
                live_weight_kg: Some(640.0),
                age_from_days: None,
                age_to_days: None,
                milk_yield_kg: Some(35.0),
                milk_fat_pct: Some(3.8),
                daily_gain_g: None,
                egg_production_per_year: None,
                litter_size: None,
                reproductive_stage: None,
            }),
            ..Default::default()
        };
        let resolved_group_id = resolved_norm_group_id(&ration, &req);
        let default_norm =
            resolved_default_norm(&ration, &req, &resolved_group_id).expect("default norm");
        let static_norm =
            crate::norms::get_norms_for_group(&resolved_group_id).expect("static norm");
        let ranges = HashMap::from([(
            "energy_eke".to_string(),
            OptimizeNormRange {
                min: Some(14.5),
                target: Some(15.2),
                max: None,
            },
        )]);

        let built = build_override_norm(
            &ration,
            &ranges,
            Some(&resolved_group_id),
            Some(&default_norm),
        )
        .expect("override norm");

        assert!(default_norm.feed_intake_max.unwrap() > static_norm.feed_intake_max.unwrap());
        assert_eq!(built.norm.feed_intake_min, default_norm.feed_intake_min);
        assert_eq!(built.norm.feed_intake_max, default_norm.feed_intake_max);
        assert_eq!(built.norm.nutrients_target.get("energy_eke"), Some(&15.2));
    }

    #[tokio::test]
    async fn alternatives_endpoint_returns_multiple_solutions() {
        let state = test_state();

        let ration_id = state
            .db
            .with_conn(|conn| {
                let hay_id = insert_feed(
                    conn,
                    &Feed {
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
                )?;
                let silage_id = insert_feed(
                    conn,
                    &Feed {
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
                )?;
                let barley_id = insert_feed(
                    conn,
                    &Feed {
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
                )?;
                let corn_id = insert_feed(
                    conn,
                    &Feed {
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
                )?;
                let soy_id = insert_feed(
                    conn,
                    &Feed {
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
                )?;
                let sunflower_id = insert_feed(
                    conn,
                    &Feed {
                        name_ru: "Sunflower meal".to_string(),
                        category: "oilseed_meal".to_string(),
                        dry_matter: Some(90.0),
                        crude_protein: Some(360.0),
                        calcium: Some(4.0),
                        phosphorus: Some(9.0),
                        price_per_ton: Some(21200.0),
                        ..Default::default()
                    },
                )?;
                let chalk_id = insert_feed(
                    conn,
                    &Feed {
                        name_ru: "Feed chalk".to_string(),
                        category: "mineral".to_string(),
                        calcium: Some(360.0),
                        phosphorus: Some(0.2),
                        price_per_ton: Some(5200.0),
                        ..Default::default()
                    },
                )?;

                let ration_id = crate::db::rations::create_ration(
                    conn,
                    &Ration {
                        name: "Alternative API".to_string(),
                        animal_group_id: Some("cattle_dairy".to_string()),
                        animal_count: 1,
                        ..Default::default()
                    },
                )?;
                crate::db::rations::add_ration_item(conn, ration_id, hay_id, 8.0, false)?;
                crate::db::rations::add_ration_item(conn, ration_id, silage_id, 14.0, false)?;
                crate::db::rations::add_ration_item(conn, ration_id, barley_id, 4.5, false)?;
                crate::db::rations::add_ration_item(conn, ration_id, soy_id, 1.6, false)?;
                crate::db::rations::add_ration_item(conn, ration_id, chalk_id, 0.12, false)?;

                let _ = corn_id;
                let _ = sunflower_id;

                Ok::<i64, anyhow::Error>(ration_id)
            })
            .unwrap();

        let Json(response) = super::optimize_ration_alternatives(
            axum::extract::State(state),
            axum::extract::Path(ration_id),
            Json(OptimizeRequest {
                mode: Some("tiered".to_string()),
                intent: Some("complete_from_library".to_string()),
                ..Default::default()
            }),
        )
        .await
        .unwrap();

        assert!(response.data.alternatives.len() >= 2);
        assert!(response.data.alternatives.len() <= 4);
        assert!(response.data.comparison.cost_range[0] <= response.data.comparison.cost_range[1]);
        assert!(!response.data.comparison.differentiators.is_empty());
    }

    #[tokio::test]
    async fn presets_endpoint_returns_research_backed_catalog() {
        let state = test_state();

        state
            .db
            .with_conn(|conn| {
                insert_feed(
                    conn,
                    &Feed {
                        name_ru: "Силос кукурузный".to_string(),
                        category: "silage".to_string(),
                        dry_matter: Some(34.0),
                        crude_protein: Some(85.0),
                        verified: true,
                        ..Default::default()
                    },
                )?;
                insert_feed(
                    conn,
                    &Feed {
                        name_ru: "Сено люцерновое".to_string(),
                        category: "roughage".to_string(),
                        dry_matter: Some(86.0),
                        crude_protein: Some(180.0),
                        verified: true,
                        ..Default::default()
                    },
                )?;
                insert_feed(
                    conn,
                    &Feed {
                        name_ru: "Шрот соевый".to_string(),
                        category: "oilseed_meal".to_string(),
                        crude_protein: Some(430.0),
                        verified: true,
                        ..Default::default()
                    },
                )?;
                insert_feed(
                    conn,
                    &Feed {
                        name_ru: "Кукуруза зерно".to_string(),
                        category: "grain".to_string(),
                        energy_oe_pig: Some(13.1),
                        verified: true,
                        ..Default::default()
                    },
                )?;
                Ok::<(), anyhow::Error>(())
            })
            .unwrap();

        let Json(response) = super::list_presets(axum::extract::State(state))
            .await
            .unwrap();

        let total_subcategories: usize = response
            .data
            .categories
            .iter()
            .map(|category| category.subcategories.len())
            .sum();
        assert_eq!(total_subcategories, 25);

        let dairy = response
            .data
            .categories
            .iter()
            .find(|category| category.species == "cattle" && category.production_type == "dairy")
            .expect("dairy preset category");
        let high_yield = dairy
            .subcategories
            .iter()
            .find(|preset| preset.id == "dairy_high_yield")
            .expect("high yield dairy preset");

        assert!(
            high_yield
                .recommendations
                .iter()
                .any(|recommendation| !recommendation.matches.is_empty())
        );
    }
}

/// Get nutrients for a ration
pub async fn get_nutrients(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<NutrientSummary>>, ApiError> {
    let summary = state.db.with_conn(|conn| {
        let ration =
            rations::get_ration(conn, id)?.ok_or_else(|| anyhow::anyhow!("Ration not found"))?;

        Ok(crate::diet_engine::calculate_nutrients(&ration))
    })?;

    Ok(ApiResponse::new(summary))
}

/// Get economics for a ration
pub async fn get_economics(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<EconomicAnalysis>>, ApiError> {
    let analysis = state.db.with_conn(|conn| {
        let ration =
            rations::get_ration(conn, id)?.ok_or_else(|| anyhow::anyhow!("Ration not found"))?;

        Ok(crate::diet_engine::calculate_economics(&ration))
    })?;

    Ok(ApiResponse::new(analysis))
}
