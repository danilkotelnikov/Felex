use super::{
    get_norms_for_group, AnimalContext, AnimalNorm, NormMethodology, NormMethodologyFactor,
    NormMethodologyMetric,
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct AdjustmentSet {
    energy: f64,
    protein: f64,
    mineral: f64,
    vitamin: f64,
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

fn round_value(value: f64) -> f64 {
    if value.abs() >= 100.0 {
        value.round()
    } else if value.abs() >= 10.0 {
        (value * 10.0).round() / 10.0
    } else {
        (value * 100.0).round() / 100.0
    }
}

fn midpoint(min: Option<f64>, max: Option<f64>) -> Option<f64> {
    match (min, max) {
        (Some(min), Some(max)) => Some((min + max) / 2.0),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn context_age_days(context: &AnimalContext) -> Option<f64> {
    match (context.age_from_days, context.age_to_days) {
        (Some(min), Some(max)) => Some((f64::from(min) + f64::from(max)) / 2.0),
        (Some(value), None) | (None, Some(value)) => Some(f64::from(value)),
        (None, None) => None,
    }
}

fn base_age_days(norm: &AnimalNorm) -> Option<f64> {
    match (norm.age_from_days, norm.age_to_days) {
        (Some(min), Some(max)) => Some((f64::from(min) + f64::from(max)) / 2.0),
        (Some(value), None) | (None, Some(value)) => Some(f64::from(value)),
        (None, None) => None,
    }
}

fn group_family(group_id: &str) -> &'static str {
    if group_id.starts_with("cattle_dairy") {
        "cattle_dairy"
    } else if group_id.starts_with("cattle_beef") {
        "cattle_beef"
    } else if group_id.starts_with("swine_sow") {
        "swine_sow"
    } else if group_id.starts_with("swine") {
        "swine_finisher"
    } else if group_id.starts_with("poultry_layer") {
        "poultry_layer"
    } else if group_id.starts_with("poultry") {
        "poultry_broiler"
    } else {
        "other"
    }
}

fn reference_egg_production(group_id: &str) -> Option<f64> {
    if group_id.starts_with("poultry_layer") {
        Some(320.0)
    } else {
        None
    }
}

fn stage_boost(family: &str, context: &AnimalContext) -> AdjustmentSet {
    if family == "swine_sow" && context.reproductive_stage.as_deref() == Some("lactation") {
        AdjustmentSet {
            energy: 0.18,
            protein: 0.22,
            mineral: 0.12,
            vitamin: 0.08,
        }
    } else {
        AdjustmentSet {
            energy: 0.0,
            protein: 0.0,
            mineral: 0.0,
            vitamin: 0.0,
        }
    }
}

fn zero_adjustment() -> AdjustmentSet {
    AdjustmentSet {
        energy: 0.0,
        protein: 0.0,
        mineral: 0.0,
        vitamin: 0.0,
    }
}

fn methodology_metric(
    key: &str,
    unit: &str,
    reference_value: Option<f64>,
    current_value: Option<f64>,
) -> NormMethodologyMetric {
    NormMethodologyMetric {
        key: key.to_string(),
        unit: unit.to_string(),
        reference_value,
        current_value,
    }
}

fn methodology_factor(key: &str, value: f64) -> NormMethodologyFactor {
    NormMethodologyFactor {
        key: key.to_string(),
        value,
    }
}

fn collect_source_refs(base: &AnimalNorm, derived_ref: Option<&str>) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(source) = base.source.as_ref() {
        refs.push(source.clone());
    }
    if let Some(reference) = derived_ref {
        if !refs.iter().any(|existing| existing == reference) {
            refs.push(reference.to_string());
        }
    }
    refs
}

fn midpoint_or_none(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    midpoint(left, right)
}

fn context_has_dynamic_inputs(context: &AnimalContext) -> bool {
    context.live_weight_kg.is_some()
        || context.age_from_days.is_some()
        || context.age_to_days.is_some()
        || context.milk_yield_kg.is_some()
        || context.milk_fat_pct.is_some()
        || context.daily_gain_g.is_some()
        || context.egg_production_per_year.is_some()
        || context
            .breed
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || context
            .sex
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || context
            .reproductive_stage
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
}

#[derive(Clone)]
struct DairyLactationModel {
    reference_weight: f64,
    current_weight: f64,
    reference_fat_pct: f64,
    reference_milk: f64,
    current_milk: f64,
    current_fat_pct: f64,
    reference_milk_equivalent: f64,
    current_milk_equivalent: f64,
    anchor_dmi: f64,
    anchor_nel: f64,
    modeled_dmi: f64,
    modeled_nel: f64,
    dmi_factor: f64,
    energy_factor: f64,
    protein_factor: f64,
    mineral_factor: f64,
    vitamin_factor: f64,
    intake_min: f64,
    intake_max: f64,
}

#[derive(Clone)]
struct SwineFinisherModel {
    reference_weight: f64,
    current_weight: f64,
    reference_gain: f64,
    current_gain: f64,
    intake_factor: f64,
    energy_factor: f64,
    amino_factor: f64,
    modeled_intake: f64,
    modeled_energy: f64,
    modeled_cp: f64,
    modeled_lys: f64,
    modeled_met_cys: f64,
    modeled_calcium: f64,
    modeled_phosphorus: f64,
    intake_min: f64,
    intake_max: f64,
}

#[derive(Clone)]
struct GenericContextScalingModel {
    reference_weight: Option<f64>,
    current_weight: f64,
    reference_gain: Option<f64>,
    current_gain: Option<f64>,
    reference_milk: Option<f64>,
    current_milk: Option<f64>,
    reference_fat_pct: f64,
    current_fat_pct: Option<f64>,
    reference_egg_production: Option<f64>,
    current_egg_production: Option<f64>,
    reference_age_days: Option<f64>,
    current_age_days: Option<f64>,
    energy_factor: f64,
    protein_factor: f64,
    mineral_factor: f64,
    vitamin_factor: f64,
    fiber_factor: f64,
    intake_factor: f64,
}

fn breed_adjustment(family: &str, breed: &str) -> AdjustmentSet {
    let normalized = breed.to_lowercase();

    match family {
        "cattle_dairy" => {
            if normalized.contains("голш") || normalized.contains("holstein") {
                AdjustmentSet {
                    energy: 0.05,
                    protein: 0.05,
                    mineral: 0.03,
                    vitamin: 0.03,
                }
            } else if normalized.contains("джерс") || normalized.contains("jersey") {
                AdjustmentSet {
                    energy: -0.02,
                    protein: 0.01,
                    mineral: 0.05,
                    vitamin: 0.02,
                }
            } else if normalized.contains("айрш") || normalized.contains("ayrshire") {
                AdjustmentSet {
                    energy: 0.01,
                    protein: 0.02,
                    mineral: 0.02,
                    vitamin: 0.02,
                }
            } else if normalized.contains("симмент") || normalized.contains("simmental") {
                AdjustmentSet {
                    energy: 0.02,
                    protein: 0.02,
                    mineral: 0.02,
                    vitamin: 0.01,
                }
            } else {
                zero_adjustment()
            }
        }
        "cattle_beef" => {
            if normalized.contains("ангус") || normalized.contains("angus") {
                AdjustmentSet {
                    energy: 0.03,
                    protein: 0.03,
                    mineral: 0.01,
                    vitamin: 0.01,
                }
            } else if normalized.contains("шароле") || normalized.contains("charolais") {
                AdjustmentSet {
                    energy: 0.04,
                    protein: 0.04,
                    mineral: 0.02,
                    vitamin: 0.02,
                }
            } else if normalized.contains("лимуз") || normalized.contains("limousin") {
                AdjustmentSet {
                    energy: 0.03,
                    protein: 0.03,
                    mineral: 0.02,
                    vitamin: 0.01,
                }
            } else {
                zero_adjustment()
            }
        }
        "swine_finisher" | "swine_sow" => {
            if normalized.contains("дюрок") || normalized.contains("duroc") {
                AdjustmentSet {
                    energy: 0.03,
                    protein: 0.03,
                    mineral: 0.01,
                    vitamin: 0.01,
                }
            } else if normalized.contains("ландрас") || normalized.contains("landrace") {
                AdjustmentSet {
                    energy: 0.02,
                    protein: 0.03,
                    mineral: 0.01,
                    vitamin: 0.01,
                }
            } else if normalized.contains("пьетрен") || normalized.contains("pietrain") {
                AdjustmentSet {
                    energy: 0.01,
                    protein: 0.04,
                    mineral: 0.01,
                    vitamin: 0.01,
                }
            } else {
                zero_adjustment()
            }
        }
        "poultry_broiler" => {
            if normalized.contains("кобб") || normalized.contains("cobb") {
                AdjustmentSet {
                    energy: 0.02,
                    protein: 0.03,
                    mineral: 0.01,
                    vitamin: 0.01,
                }
            } else if normalized.contains("росс") || normalized.contains("ross") {
                AdjustmentSet {
                    energy: 0.03,
                    protein: 0.03,
                    mineral: 0.01,
                    vitamin: 0.01,
                }
            } else {
                zero_adjustment()
            }
        }
        "poultry_layer" => {
            if normalized.contains("ломанн")
                || normalized.contains("loman")
                || normalized.contains("lohmann")
            {
                AdjustmentSet {
                    energy: 0.01,
                    protein: 0.02,
                    mineral: 0.03,
                    vitamin: 0.02,
                }
            } else if normalized.contains("хайсекс") || normalized.contains("hysex") {
                AdjustmentSet {
                    energy: 0.01,
                    protein: 0.02,
                    mineral: 0.03,
                    vitamin: 0.02,
                }
            } else {
                zero_adjustment()
            }
        }
        _ => zero_adjustment(),
    }
}

fn sex_adjustment(family: &str, sex: &str) -> AdjustmentSet {
    if matches!(family, "cattle_dairy" | "poultry_layer" | "swine_sow") {
        return zero_adjustment();
    }

    match sex {
        "male" => AdjustmentSet {
            energy: 0.03,
            protein: 0.03,
            mineral: 0.01,
            vitamin: 0.0,
        },
        "female" => AdjustmentSet {
            energy: -0.01,
            protein: -0.01,
            mineral: 0.0,
            vitamin: 0.0,
        },
        _ => zero_adjustment(),
    }
}

fn category_for_key(key: &str) -> &'static str {
    if matches!(key, "ca_p_ratio" | "dig_protein_cattle_pct_cp") || key.ends_with("_ratio")
    {
        "ratio"
    } else if key.starts_with("energy_") || matches!(key, "energy_eke") {
        "energy"
    } else if key.contains("protein")
        || matches!(
            key,
            "lysine"
                | "lysine_sid"
                | "lysine_sid_pct"
                | "lysine_tid_pct"
                | "methionine_cystine"
                | "methionine_cystine_sid"
                | "methionine_cystine_tid_pct"
        )
    {
        "protein"
    } else if key.contains("fiber") || key.contains("starch") || key == "sugar" {
        "fiber"
    } else if key.starts_with("vit_") || matches!(key, "carotene") {
        "vitamin"
    } else {
        "mineral"
    }
}

fn scale_value(value: f64, factor: f64, category: &str) -> f64 {
    if category == "ratio" {
        return value;
    }
    round_value(value * factor)
}

fn scale_map(
    source: &HashMap<String, f64>,
    energy_factor: f64,
    protein_factor: f64,
    mineral_factor: f64,
    vitamin_factor: f64,
    fiber_factor: f64,
) -> HashMap<String, f64> {
    source
        .iter()
        .map(|(key, value)| {
            let category = category_for_key(key);
            let factor = match category {
                "energy" => energy_factor,
                "protein" => protein_factor,
                "vitamin" => vitamin_factor,
                "fiber" => fiber_factor,
                "ratio" => 1.0,
                _ => mineral_factor,
            };
            (key.clone(), scale_value(*value, factor, category))
        })
        .collect()
}

const SWINE_FINISHER_WEIGHT_POINTS: [f64; 3] = [55.0, 80.0, 110.0];
const SWINE_FINISHER_GAIN_POINTS: [f64; 3] = [700.0, 900.0, 1100.0];
const SWINE_FINISHER_INTAKE_TABLE: [[f64; 3]; 3] =
    [[2.15, 2.25, 2.35], [2.60, 2.75, 2.90], [2.90, 3.05, 3.20]];
const SWINE_FINISHER_ENERGY_TABLE: [[f64; 3]; 3] =
    [[25.5, 27.5, 29.5], [30.5, 33.0, 35.5], [34.0, 36.5, 39.0]];
const SWINE_FINISHER_CP_TABLE: [[f64; 3]; 3] = [
    [150.0, 155.0, 160.0],
    [135.0, 140.0, 145.0],
    [125.0, 130.0, 135.0],
];
const SWINE_FINISHER_LYS_TABLE: [[f64; 3]; 3] = [[8.1, 8.6, 9.1], [7.1, 7.5, 8.0], [6.3, 6.7, 7.1]];
const SWINE_FINISHER_MET_CYS_TABLE: [[f64; 3]; 3] =
    [[4.86, 5.16, 5.46], [4.26, 4.50, 4.80], [3.78, 4.02, 4.26]];
const SWINE_FINISHER_CA_TABLE: [[f64; 3]; 3] = [[7.0, 7.0, 7.0], [6.6, 6.5, 6.5], [6.2, 6.2, 6.1]];
const SWINE_FINISHER_AVAIL_P_TABLE: [[f64; 3]; 3] =
    [[2.8, 2.8, 2.8], [2.6, 2.5, 2.5], [2.4, 2.4, 2.3]];

fn interpolation_bounds(points: &[f64], value: f64) -> (usize, usize, f64) {
    if points.len() <= 1 {
        return (0, 0, 0.0);
    }

    for index in 0..(points.len() - 1) {
        let left = points[index];
        let right = points[index + 1];
        if value >= left && value <= right {
            let span = (right - left).max(f64::EPSILON);
            return (index, index + 1, (value - left) / span);
        }
    }

    if value < points[0] {
        let span = (points[1] - points[0]).max(f64::EPSILON);
        return (0, 1, (value - points[0]) / span);
    }

    let last = points.len() - 1;
    let span = (points[last] - points[last - 1]).max(f64::EPSILON);
    (last - 1, last, (value - points[last - 1]) / span)
}

fn bilinear_interpolate(
    x_points: &[f64],
    y_points: &[f64],
    table: &[[f64; 3]; 3],
    x: f64,
    y: f64,
) -> f64 {
    let (x0, x1, tx) = interpolation_bounds(x_points, x);
    let (y0, y1, ty) = interpolation_bounds(y_points, y);

    let q11 = table[x0][y0];
    let q12 = table[x0][y1];
    let q21 = table[x1][y0];
    let q22 = table[x1][y1];

    let top = q11 + (q12 - q11) * ty;
    let bottom = q21 + (q22 - q21) * ty;
    top + (bottom - top) * tx
}

fn dairy_fcm_4_pct(milk_yield_kg: f64, milk_fat_pct: f64) -> f64 {
    if milk_yield_kg <= 0.0 {
        0.0
    } else {
        // 4% Fat Corrected Milk (FCM)
        0.4 * milk_yield_kg + 15.0 * (milk_yield_kg * (milk_fat_pct / 100.0))
    }
}

fn dairy_modeled_dmi(weight_kg: f64, fcm_kg: f64) -> f64 {
    // Simplified NASEM/NRC 2001 DMI formula (excluding Days In Milk curve for generic application)
    0.372 * fcm_kg + 0.0968 * weight_kg.powf(0.75)
}

fn dairy_modeled_nel(weight_kg: f64, milk_yield_kg: f64, milk_fat_pct: f64) -> f64 {
    // NASEM/NRC 2001 Net Energy for Lactation
    let nel_maint = 0.08 * weight_kg.powf(0.75);
    let nel_milk = milk_yield_kg * (0.3512 + 0.0962 * milk_fat_pct);
    nel_maint + nel_milk
}

fn is_dairy_lactating_group(group_id: &str, base: &AnimalNorm, context: &AnimalContext) -> bool {
    group_id.starts_with("cattle_dairy")
        && !group_id.contains("dry")
        && !group_id.contains("heifer")
        && (base.milk_yield_kg.is_some() || context.milk_yield_kg.is_some())
}

fn is_swine_finisher_group(group_id: &str, base: &AnimalNorm) -> bool {
    group_id == "swine_finisher" || base.id == "swine_finisher"
}

fn is_dairy_energy_key(key: &str) -> bool {
    matches!(key, "energy_eke" | "energy_oe_cattle")
}

fn is_dairy_daily_amount_key(key: &str) -> bool {
    matches!(
        key,
        "crude_protein"
            | "dig_protein_cattle"
            | "calcium"
            | "phosphorus"
            | "vit_d3"
            | "vit_e"
    )
}

fn scale_dairy_map(
    source: &HashMap<String, f64>,
    energy_factor: f64,
    protein_factor: f64,
    mineral_factor: f64,
    vitamin_factor: f64,
) -> HashMap<String, f64> {
    source
        .iter()
        .map(|(key, value)| {
            let scaled = if is_dairy_energy_key(key) {
                round_value(*value * energy_factor)
            } else if matches!(key.as_str(), "crude_protein" | "dig_protein_cattle") {
                round_value(*value * protein_factor)
            } else if matches!(key.as_str(), "calcium" | "phosphorus") {
                round_value(*value * mineral_factor)
            } else if matches!(key.as_str(), "vit_d3" | "vit_e") {
                round_value(*value * vitamin_factor)
            } else if is_dairy_daily_amount_key(key) {
                round_value(*value * protein_factor)
            } else {
                *value
            };

            (key.clone(), scaled)
        })
        .collect()
}

fn build_dairy_lactation_model(base: &AnimalNorm, context: &AnimalContext) -> DairyLactationModel {
    let reference_weight = midpoint(base.weight_from_kg, base.weight_to_kg).unwrap_or(620.0);
    let current_weight = context.live_weight_kg.unwrap_or(reference_weight);
    let reference_fat_pct = base.milk_fat_pct.unwrap_or(3.8);
    let reference_milk = base.milk_yield_kg.unwrap_or(30.0);
    let current_milk = context.milk_yield_kg.unwrap_or(reference_milk);
    let current_fat_pct = context.milk_fat_pct.unwrap_or(reference_fat_pct);

    let reference_milk_equivalent = dairy_fcm_4_pct(reference_milk, reference_fat_pct);
    let current_milk_equivalent = dairy_fcm_4_pct(current_milk, current_fat_pct);

    let anchor_dmi = dairy_modeled_dmi(reference_weight, reference_milk_equivalent).max(1.0);
    let anchor_nel = dairy_modeled_nel(reference_weight, reference_milk, reference_fat_pct).max(1.0);

    let breed = context
        .breed
        .as_deref()
        .map(|breed| breed_adjustment("cattle_dairy", breed))
        .unwrap_or_else(zero_adjustment);

    let intake_adjustment = clamp(1.0 + breed.energy * 0.5, 0.96, 1.05);
    let energy_adjustment = clamp(1.0 + breed.energy * 0.35, 0.97, 1.06);
    let protein_adjustment = clamp(1.0 + breed.protein * 0.35, 0.97, 1.06);
    let mineral_adjustment = clamp(1.0 + breed.mineral * 0.35, 0.97, 1.07);
    let vitamin_adjustment = clamp(1.0 + breed.vitamin * 0.35, 0.97, 1.07);

    let modeled_dmi =
        dairy_modeled_dmi(current_weight, current_milk_equivalent).max(1.0) * intake_adjustment;
    let modeled_nel =
        dairy_modeled_nel(current_weight, current_milk, current_fat_pct).max(1.0) * energy_adjustment;

    let dmi_factor = modeled_dmi / anchor_dmi;
    let energy_factor = modeled_nel / anchor_nel;
    let protein_factor = dmi_factor * protein_adjustment;
    let mineral_factor = dmi_factor * mineral_adjustment;
    let vitamin_factor = dmi_factor * vitamin_adjustment;

    let intake_min = round_value((modeled_dmi * 0.90).max(1.0));
    let intake_max = round_value((modeled_dmi * 1.12).max(intake_min + 1.0));

    DairyLactationModel {
        reference_weight,
        current_weight,
        reference_fat_pct,
        reference_milk,
        current_milk,
        current_fat_pct,
        reference_milk_equivalent,
        current_milk_equivalent,
        anchor_dmi,
        anchor_nel,
        modeled_dmi,
        modeled_nel,
        dmi_factor,
        energy_factor,
        protein_factor,
        mineral_factor,
        vitamin_factor,
        intake_min,
        intake_max,
    }
}

fn derive_dairy_lactation_norms(base: &AnimalNorm, context: &AnimalContext) -> AnimalNorm {
    let model = build_dairy_lactation_model(base, context);
    let source = base
        .source
        .clone()
        .map(|source| {
            format!(
                "{source}; Felex dairy intake and energy interpolation from NASEM dairy table anchors"
            )
        })
        .or_else(|| {
            Some(
                "Felex dairy intake and energy interpolation from NASEM dairy table anchors"
                    .to_string(),
            )
        });
    let notes = base
        .notes
        .clone()
        .map(|note| {
            format!(
                "{note}; lactating dairy requirements derived from modeled intake and energy context"
            )
        })
        .or_else(|| {
            Some(
                "Lactating dairy requirements derived from modeled intake and energy context"
                    .to_string(),
            )
        });

    AnimalNorm {
        id: base.id.clone(),
        species: base.species.clone(),
        production_type: base
            .production_type
            .clone()
            .or(context.production_type.clone()),
        breed_group: base.breed_group.clone(),
        sex: base.sex.clone(),
        age_from_days: context.age_from_days.or(base.age_from_days),
        age_to_days: context.age_to_days.or(base.age_to_days),
        weight_from_kg: Some(model.current_weight),
        weight_to_kg: Some(model.current_weight),
        milk_yield_kg: Some(model.current_milk),
        milk_fat_pct: Some(model.current_fat_pct),
        milk_protein_pct: base.milk_protein_pct,
        daily_gain_g: context.daily_gain_g.or(base.daily_gain_g),
        nutrients_min: scale_dairy_map(
            &base.nutrients_min,
            model.energy_factor,
            model.protein_factor,
            model.mineral_factor,
            model.vitamin_factor,
        ),
        nutrients_max: scale_dairy_map(
            &base.nutrients_max,
            model.energy_factor,
            model.protein_factor,
            model.mineral_factor,
            model.vitamin_factor,
        ),
        nutrients_target: scale_dairy_map(
            &base.nutrients_target,
            model.energy_factor,
            model.protein_factor,
            model.mineral_factor,
            model.vitamin_factor,
        ),
        feed_intake_min: Some(model.intake_min),
        feed_intake_max: Some(model.intake_max),
        notes,
        source,
    }
}

fn swine_finisher_sex_adjustments(sex: Option<&str>) -> (f64, f64, f64) {
    let normalized = sex.unwrap_or_default().to_lowercase();
    match normalized.as_str() {
        "female" => (0.98, 1.0, 1.02),
        "male" => (1.01, 1.01, 0.98),
        _ => (1.0, 1.0, 1.0),
    }
}

fn build_swine_finisher_model(base: &AnimalNorm, context: &AnimalContext) -> SwineFinisherModel {
    let reference_weight = midpoint(base.weight_from_kg, base.weight_to_kg).unwrap_or(80.0);
    let current_weight = context.live_weight_kg.unwrap_or(reference_weight);
    let reference_gain = base.daily_gain_g.map(f64::from).unwrap_or(900.0);
    let current_gain = context
        .daily_gain_g
        .map(f64::from)
        .unwrap_or(reference_gain);
    let (intake_factor, energy_factor, amino_factor) =
        swine_finisher_sex_adjustments(context.sex.as_deref());

    let modeled_intake = bilinear_interpolate(
        &SWINE_FINISHER_WEIGHT_POINTS,
        &SWINE_FINISHER_GAIN_POINTS,
        &SWINE_FINISHER_INTAKE_TABLE,
        current_weight,
        current_gain,
    ) * intake_factor;
    let modeled_energy = bilinear_interpolate(
        &SWINE_FINISHER_WEIGHT_POINTS,
        &SWINE_FINISHER_GAIN_POINTS,
        &SWINE_FINISHER_ENERGY_TABLE,
        current_weight,
        current_gain,
    ) * energy_factor;
    let modeled_cp = bilinear_interpolate(
        &SWINE_FINISHER_WEIGHT_POINTS,
        &SWINE_FINISHER_GAIN_POINTS,
        &SWINE_FINISHER_CP_TABLE,
        current_weight,
        current_gain,
    );
    let modeled_lys = bilinear_interpolate(
        &SWINE_FINISHER_WEIGHT_POINTS,
        &SWINE_FINISHER_GAIN_POINTS,
        &SWINE_FINISHER_LYS_TABLE,
        current_weight,
        current_gain,
    ) * amino_factor;
    let modeled_met_cys = bilinear_interpolate(
        &SWINE_FINISHER_WEIGHT_POINTS,
        &SWINE_FINISHER_GAIN_POINTS,
        &SWINE_FINISHER_MET_CYS_TABLE,
        current_weight,
        current_gain,
    ) * amino_factor;
    let modeled_calcium = bilinear_interpolate(
        &SWINE_FINISHER_WEIGHT_POINTS,
        &SWINE_FINISHER_GAIN_POINTS,
        &SWINE_FINISHER_CA_TABLE,
        current_weight,
        current_gain,
    );
    let modeled_phosphorus = bilinear_interpolate(
        &SWINE_FINISHER_WEIGHT_POINTS,
        &SWINE_FINISHER_GAIN_POINTS,
        &SWINE_FINISHER_AVAIL_P_TABLE,
        current_weight,
        current_gain,
    );

    let intake_min = round_value((modeled_intake * 0.92).max(1.5));
    let intake_max = round_value((modeled_intake * 1.10).max(intake_min + 0.35));

    SwineFinisherModel {
        reference_weight,
        current_weight,
        reference_gain,
        current_gain,
        intake_factor,
        energy_factor,
        amino_factor,
        modeled_intake,
        modeled_energy,
        modeled_cp,
        modeled_lys,
        modeled_met_cys,
        modeled_calcium,
        modeled_phosphorus,
        intake_min,
        intake_max,
    }
}

fn derive_swine_finisher_norms(base: &AnimalNorm, context: &AnimalContext) -> AnimalNorm {
    let model = build_swine_finisher_model(base, context);
    let energy_target = round_value(model.modeled_energy);
    let energy_min = round_value(model.modeled_energy * 0.94);
    let crude_protein_min = round_value(model.modeled_cp);
    let lysine_target = round_value(model.modeled_lys);
    let lysine_min = round_value(model.modeled_lys);
    let met_cys_target = round_value(model.modeled_met_cys);
    let met_cys_min = round_value(model.modeled_met_cys);
    let calcium_min = round_value(model.modeled_calcium);
    let phosphorus_min = round_value(model.modeled_phosphorus);

    let mut nutrients_min = base.nutrients_min.clone();
    nutrients_min.insert("energy_oe_pig".to_string(), energy_min);
    nutrients_min.insert("crude_protein".to_string(), crude_protein_min);
    nutrients_min.insert("lysine_sid".to_string(), lysine_min);
    nutrients_min.insert("methionine_cystine_sid".to_string(), met_cys_min);
    nutrients_min.insert("calcium".to_string(), calcium_min);
    nutrients_min.insert("phosphorus".to_string(), phosphorus_min);

    let mut nutrients_target = base.nutrients_target.clone();
    nutrients_target.insert("energy_oe_pig".to_string(), energy_target);
    nutrients_target.insert("lysine_sid".to_string(), lysine_target);
    nutrients_target.insert("methionine_cystine_sid".to_string(), met_cys_target);

    let source = base
        .source
        .clone()
        .map(|source| {
            format!(
                "{source}; Felex swine finisher interpolation from NASEM swine weight-gain anchors"
            )
        })
        .or_else(|| {
            Some(
                "Felex swine finisher interpolation from NASEM swine weight-gain anchors"
                    .to_string(),
            )
        });
    let notes = base
        .notes
        .clone()
        .map(|note| {
            format!(
                "{note}; growing-finishing swine requirements derived from modeled weight, gain, and SID amino-acid context"
            )
        })
        .or_else(|| {
            Some(
                "Growing-finishing swine requirements derived from modeled weight, gain, and SID amino-acid context"
                    .to_string(),
            )
        });

    AnimalNorm {
        id: base.id.clone(),
        species: base.species.clone(),
        production_type: base
            .production_type
            .clone()
            .or(context.production_type.clone()),
        breed_group: base.breed_group.clone(),
        sex: base.sex.clone(),
        age_from_days: context.age_from_days.or(base.age_from_days),
        age_to_days: context.age_to_days.or(base.age_to_days),
        weight_from_kg: Some(model.current_weight),
        weight_to_kg: Some(model.current_weight),
        milk_yield_kg: base.milk_yield_kg,
        milk_fat_pct: base.milk_fat_pct,
        milk_protein_pct: base.milk_protein_pct,
        daily_gain_g: Some(model.current_gain.round() as i32),
        nutrients_min,
        nutrients_max: base.nutrients_max.clone(),
        nutrients_target,
        feed_intake_min: Some(model.intake_min),
        feed_intake_max: Some(model.intake_max),
        notes,
        source,
    }
}

fn build_generic_context_scaling_model(
    group_id: &str,
    base: &AnimalNorm,
    context: &AnimalContext,
) -> GenericContextScalingModel {
    let family = group_family(group_id);
    let reference_weight = midpoint(base.weight_from_kg, base.weight_to_kg);
    let current_weight = context.live_weight_kg.or(reference_weight).unwrap_or(1.0);
    let weight_delta_pct = reference_weight
        .filter(|value| *value > 0.0)
        .map(|reference| (current_weight - reference) / reference)
        .unwrap_or(0.0);

    let reference_gain = base.daily_gain_g.map(f64::from);
    let gain_delta_pct = match (reference_gain, context.daily_gain_g) {
        (Some(reference), Some(current)) if reference > 0.0 => {
            (f64::from(current) - reference) / reference
        }
        _ => 0.0,
    };

    let reference_milk = base.milk_yield_kg;
    let milk_delta = match (reference_milk, context.milk_yield_kg) {
        (Some(reference), Some(current)) => current - reference,
        _ => 0.0,
    };

    let reference_fat_pct = base.milk_fat_pct.or(context.milk_fat_pct).unwrap_or(3.7);
    let fat_delta = context.milk_fat_pct.unwrap_or(reference_fat_pct) - reference_fat_pct;

    let reference_egg_production = reference_egg_production(group_id);
    let egg_delta_pct = match (reference_egg_production, context.egg_production_per_year) {
        (Some(reference), Some(current)) if reference > 0.0 => (current - reference) / reference,
        _ => 0.0,
    };

    let reference_age_days = base_age_days(base);
    let current_age_days = context_age_days(context);
    let age_delta_pct = match (reference_age_days, current_age_days) {
        (Some(reference), Some(current)) if reference > 0.0 => (current - reference) / reference,
        _ => 0.0,
    };

    let stage = stage_boost(family, context);
    let breed = context
        .breed
        .as_deref()
        .map(|breed| breed_adjustment(family, breed))
        .unwrap_or_else(zero_adjustment);
    let sex = context
        .sex
        .as_deref()
        .map(|sex| sex_adjustment(family, sex))
        .unwrap_or_else(zero_adjustment);

    let energy_factor = clamp(
        1.0 + weight_delta_pct * if family == "cattle_dairy" { 0.18 } else { 0.35 }
            + gain_delta_pct * 0.45
            + milk_delta * 0.018
            + fat_delta * 0.03
            + egg_delta_pct * 0.18
            + breed.energy
            + sex.energy
            + stage.energy,
        0.8,
        1.35,
    );

    let protein_factor = clamp(
        1.0 + weight_delta_pct * if family == "cattle_dairy" { 0.12 } else { 0.24 }
            + gain_delta_pct * 0.55
            + milk_delta * 0.022
            + fat_delta * 0.025
            + egg_delta_pct * 0.22
            + breed.protein
            + sex.protein
            + stage.protein,
        0.82,
        1.45,
    );

    let mineral_factor = clamp(
        1.0 + weight_delta_pct * 0.16
            + milk_delta * 0.012
            + fat_delta * 0.04
            + egg_delta_pct * 0.28
            + breed.mineral
            + sex.mineral
            + stage.mineral,
        0.85,
        1.45,
    );

    let vitamin_factor = clamp(
        1.0 + weight_delta_pct * 0.08
            + milk_delta * 0.01
            + egg_delta_pct * 0.12
            + breed.vitamin
            + sex.vitamin
            + stage.vitamin,
        0.85,
        1.3,
    );

    let fiber_factor = clamp(
        1.0 + weight_delta_pct * 0.05 - milk_delta * 0.004 + egg_delta_pct * 0.04
            - age_delta_pct * 0.03,
        0.9,
        1.12,
    );

    let intake_factor = clamp(
        1.0 + weight_delta_pct * if family == "cattle_dairy" { 0.14 } else { 0.18 }
            + gain_delta_pct * 0.28
            + milk_delta * 0.010
            + fat_delta * 0.015
            + egg_delta_pct * 0.08
            + stage.energy * 0.5,
        0.75,
        1.35,
    );

    GenericContextScalingModel {
        reference_weight,
        current_weight,
        reference_gain,
        current_gain: context.daily_gain_g.map(f64::from),
        reference_milk,
        current_milk: context.milk_yield_kg,
        reference_fat_pct,
        current_fat_pct: context.milk_fat_pct,
        reference_egg_production,
        current_egg_production: context.egg_production_per_year,
        reference_age_days,
        current_age_days,
        energy_factor,
        protein_factor,
        mineral_factor,
        vitamin_factor,
        fiber_factor,
        intake_factor,
    }
}

pub fn derive_norms_for_context(
    group_id: &str,
    context: Option<&AnimalContext>,
) -> Option<AnimalNorm> {
    let base = get_norms_for_group(group_id)?;
    let Some(context) = context else {
        return Some(base);
    };

    if is_dairy_lactating_group(group_id, &base, context) {
        return Some(derive_dairy_lactation_norms(&base, context));
    }

    if is_swine_finisher_group(group_id, &base) {
        return Some(derive_swine_finisher_norms(&base, context));
    }

    let model = build_generic_context_scaling_model(group_id, &base, context);

    let source = base
        .source
        .clone()
        .map(|source| format!("{source}; Felex dynamic context"))
        .or_else(|| Some("Felex dynamic context".to_string()));
    let notes = base
        .notes
        .clone()
        .map(|note| format!("{note}; adjusted for current animal context"))
        .or_else(|| Some("Adjusted for current animal context".to_string()));

    Some(AnimalNorm {
        id: base.id.clone(),
        species: base.species.clone(),
        production_type: base
            .production_type
            .clone()
            .or(context.production_type.clone()),
        breed_group: base.breed_group.clone(),
        sex: base.sex.clone(),
        age_from_days: context.age_from_days.or(base.age_from_days),
        age_to_days: context.age_to_days.or(base.age_to_days),
        weight_from_kg: context.live_weight_kg.or(base.weight_from_kg),
        weight_to_kg: context.live_weight_kg.or(base.weight_to_kg),
        milk_yield_kg: context.milk_yield_kg.or(base.milk_yield_kg),
        milk_fat_pct: context.milk_fat_pct.or(base.milk_fat_pct),
        milk_protein_pct: base.milk_protein_pct,
        daily_gain_g: context.daily_gain_g.or(base.daily_gain_g),
        nutrients_min: scale_map(
            &base.nutrients_min,
            model.energy_factor,
            model.protein_factor,
            model.mineral_factor,
            model.vitamin_factor,
            model.fiber_factor,
        ),
        nutrients_max: scale_map(
            &base.nutrients_max,
            model.energy_factor,
            model.protein_factor,
            model.mineral_factor,
            model.vitamin_factor,
            model.fiber_factor,
        ),
        nutrients_target: scale_map(
            &base.nutrients_target,
            model.energy_factor,
            model.protein_factor,
            model.mineral_factor,
            model.vitamin_factor,
            model.fiber_factor,
        ),
        feed_intake_min: base
            .feed_intake_min
            .map(|value| round_value(value * model.intake_factor)),
        feed_intake_max: base
            .feed_intake_max
            .map(|value| round_value(value * model.intake_factor)),
        notes,
        source,
    })
}

fn reference_table_methodology(base: &AnimalNorm) -> NormMethodology {
    let reference_weight = midpoint_or_none(base.weight_from_kg, base.weight_to_kg);
    let reference_age_days = base_age_days(base);
    let intake_key = if base.species == "cattle" {
        "dry_matter_intake"
    } else {
        "feed_intake"
    };

    let mut driver_metrics = Vec::new();
    if reference_weight.is_some() {
        driver_metrics.push(methodology_metric(
            "live_weight_kg",
            "kg",
            reference_weight,
            reference_weight,
        ));
    }
    if base.milk_yield_kg.is_some() {
        driver_metrics.push(methodology_metric(
            "milk_yield_kg",
            "kg_day",
            base.milk_yield_kg,
            base.milk_yield_kg,
        ));
    }
    if base.milk_fat_pct.is_some() {
        driver_metrics.push(methodology_metric(
            "milk_fat_pct",
            "pct",
            base.milk_fat_pct,
            base.milk_fat_pct,
        ));
    }
    if base.daily_gain_g.is_some() {
        driver_metrics.push(methodology_metric(
            "daily_gain_g",
            "g_day",
            base.daily_gain_g.map(f64::from),
            base.daily_gain_g.map(f64::from),
        ));
    }
    if let Some(reference_value) = reference_egg_production(&base.id) {
        driver_metrics.push(methodology_metric(
            "egg_production_per_year",
            "eggs_year",
            Some(reference_value),
            Some(reference_value),
        ));
    }
    if reference_age_days.is_some() {
        driver_metrics.push(methodology_metric(
            "age_days",
            "days",
            reference_age_days,
            reference_age_days,
        ));
    }

    let mut derived_metrics = Vec::new();
    if base.feed_intake_min.is_some() || base.feed_intake_max.is_some() {
        derived_metrics.push(methodology_metric(
            &format!("{intake_key}_min"),
            "kg_day",
            base.feed_intake_min,
            base.feed_intake_min,
        ));
        derived_metrics.push(methodology_metric(
            &format!("{intake_key}_max"),
            "kg_day",
            base.feed_intake_max,
            base.feed_intake_max,
        ));
    }

    NormMethodology {
        key: "reference_table".to_string(),
        reference_group_id: base.id.clone(),
        dynamic: false,
        source_refs: collect_source_refs(base, None),
        driver_metrics,
        derived_metrics,
        scaling_factors: Vec::new(),
    }
}

fn dairy_lactation_methodology(base: &AnimalNorm, context: &AnimalContext) -> NormMethodology {
    let model = build_dairy_lactation_model(base, context);
    NormMethodology {
        key: "dairy_lactation_interpolation".to_string(),
        reference_group_id: base.id.clone(),
        dynamic: true,
        source_refs: collect_source_refs(
            base,
            Some("NASEM Dairy 2021 intake and energy anchor tables"),
        ),
        driver_metrics: vec![
            methodology_metric(
                "live_weight_kg",
                "kg",
                Some(model.reference_weight),
                Some(model.current_weight),
            ),
            methodology_metric(
                "milk_yield_kg",
                "kg_day",
                Some(model.reference_milk),
                Some(model.current_milk),
            ),
            methodology_metric(
                "milk_fat_pct",
                "pct",
                Some(model.reference_fat_pct),
                Some(model.current_fat_pct),
            ),
            methodology_metric(
                "fat_corrected_milk_kg",
                "kg_day",
                Some(model.reference_milk_equivalent),
                Some(model.current_milk_equivalent),
            ),
        ],
        derived_metrics: vec![
            methodology_metric(
                "dry_matter_intake_modeled",
                "kg_day",
                Some(model.anchor_dmi),
                Some(model.modeled_dmi),
            ),
            methodology_metric(
                "energy_oe_cattle_modeled",
                "mj_day",
                Some(model.anchor_nel),
                Some(model.modeled_nel),
            ),
            methodology_metric(
                "dry_matter_intake_min",
                "kg_day",
                None,
                Some(model.intake_min),
            ),
            methodology_metric(
                "dry_matter_intake_max",
                "kg_day",
                None,
                Some(model.intake_max),
            ),
        ],
        scaling_factors: vec![
            methodology_factor("intake_factor", model.dmi_factor),
            methodology_factor("energy_factor", model.energy_factor),
            methodology_factor("protein_factor", model.protein_factor),
            methodology_factor("mineral_factor", model.mineral_factor),
            methodology_factor("vitamin_factor", model.vitamin_factor),
        ],
    }
}

fn swine_finisher_methodology(base: &AnimalNorm, context: &AnimalContext) -> NormMethodology {
    let model = build_swine_finisher_model(base, context);
    let reference_intake = midpoint_or_none(base.feed_intake_min, base.feed_intake_max);
    let reference_energy = base.nutrients_target.get("energy_oe_pig").copied();
    let reference_lys = base.nutrients_target.get("lysine_sid").copied();
    let reference_met_cys = base.nutrients_target.get("methionine_cystine_sid").copied();

    NormMethodology {
        key: "swine_finisher_interpolation".to_string(),
        reference_group_id: base.id.clone(),
        dynamic: true,
        source_refs: collect_source_refs(
            base,
            Some("NASEM Swine 2012 weight and gain anchor tables"),
        ),
        driver_metrics: vec![
            methodology_metric(
                "live_weight_kg",
                "kg",
                Some(model.reference_weight),
                Some(model.current_weight),
            ),
            methodology_metric(
                "daily_gain_g",
                "g_day",
                Some(model.reference_gain),
                Some(model.current_gain),
            ),
        ],
        derived_metrics: vec![
            methodology_metric(
                "feed_intake_modeled",
                "kg_day",
                reference_intake,
                Some(model.modeled_intake),
            ),
            methodology_metric(
                "energy_oe_pig_modeled",
                "mj_day",
                reference_energy,
                Some(round_value(model.modeled_energy)),
            ),
            methodology_metric(
                "crude_protein_modeled",
                "g_kg_feed",
                base.nutrients_min.get("crude_protein").copied(),
                Some(round_value(model.modeled_cp)),
            ),
            methodology_metric(
                "lysine_sid_modeled",
                "g_kg_feed",
                reference_lys,
                Some(round_value(model.modeled_lys)),
            ),
            methodology_metric(
                "methionine_cystine_sid_modeled",
                "g_kg_feed",
                reference_met_cys,
                Some(round_value(model.modeled_met_cys)),
            ),
            methodology_metric(
                "phosphorus_modeled",
                "g_kg_feed",
                base.nutrients_min.get("phosphorus").copied(),
                Some(round_value(model.modeled_phosphorus)),
            ),
        ],
        scaling_factors: vec![
            methodology_factor("intake_factor", model.intake_factor),
            methodology_factor("energy_factor", model.energy_factor),
            methodology_factor("amino_factor", model.amino_factor),
        ],
    }
}

fn generic_context_methodology(
    group_id: &str,
    base: &AnimalNorm,
    context: &AnimalContext,
) -> NormMethodology {
    let model = build_generic_context_scaling_model(group_id, base, context);
    let intake_key = if base.species == "cattle" {
        "dry_matter_intake"
    } else {
        "feed_intake"
    };
    let mut driver_metrics = Vec::new();
    if model.reference_weight.is_some() || context.live_weight_kg.is_some() {
        driver_metrics.push(methodology_metric(
            "live_weight_kg",
            "kg",
            model.reference_weight,
            Some(model.current_weight),
        ));
    }
    if model.reference_gain.is_some() || model.current_gain.is_some() {
        driver_metrics.push(methodology_metric(
            "daily_gain_g",
            "g_day",
            model.reference_gain,
            model.current_gain,
        ));
    }
    if model.reference_milk.is_some() || model.current_milk.is_some() {
        driver_metrics.push(methodology_metric(
            "milk_yield_kg",
            "kg_day",
            model.reference_milk,
            model.current_milk,
        ));
    }
    if model.current_fat_pct.is_some() || base.milk_fat_pct.is_some() {
        driver_metrics.push(methodology_metric(
            "milk_fat_pct",
            "pct",
            Some(model.reference_fat_pct),
            model.current_fat_pct.or(Some(model.reference_fat_pct)),
        ));
    }
    if model.reference_egg_production.is_some() || model.current_egg_production.is_some() {
        driver_metrics.push(methodology_metric(
            "egg_production_per_year",
            "eggs_year",
            model.reference_egg_production,
            model.current_egg_production,
        ));
    }
    if model.reference_age_days.is_some() || model.current_age_days.is_some() {
        driver_metrics.push(methodology_metric(
            "age_days",
            "days",
            model.reference_age_days,
            model.current_age_days,
        ));
    }

    let mut derived_metrics = Vec::new();
    if let Some(value) = base.feed_intake_min {
        derived_metrics.push(methodology_metric(
            &format!("{intake_key}_min"),
            "kg_day",
            Some(value),
            Some(round_value(value * model.intake_factor)),
        ));
    }
    if let Some(value) = base.feed_intake_max {
        derived_metrics.push(methodology_metric(
            &format!("{intake_key}_max"),
            "kg_day",
            Some(value),
            Some(round_value(value * model.intake_factor)),
        ));
    }

    NormMethodology {
        key: "context_scaling".to_string(),
        reference_group_id: base.id.clone(),
        dynamic: true,
        source_refs: collect_source_refs(base, Some("Felex context scaling model")),
        driver_metrics,
        derived_metrics,
        scaling_factors: vec![
            methodology_factor("energy_factor", model.energy_factor),
            methodology_factor("protein_factor", model.protein_factor),
            methodology_factor("mineral_factor", model.mineral_factor),
            methodology_factor("vitamin_factor", model.vitamin_factor),
            methodology_factor("fiber_factor", model.fiber_factor),
            methodology_factor("intake_factor", model.intake_factor),
        ],
    }
}

pub fn describe_norm_methodology(
    group_id: &str,
    context: Option<&AnimalContext>,
) -> Option<NormMethodology> {
    let base = get_norms_for_group(group_id)?;
    let Some(context) = context else {
        return Some(reference_table_methodology(&base));
    };

    if !context_has_dynamic_inputs(context) {
        return Some(reference_table_methodology(&base));
    }

    if is_dairy_lactating_group(group_id, &base, context) {
        return Some(dairy_lactation_methodology(&base, context));
    }

    if is_swine_finisher_group(group_id, &base) {
        return Some(swine_finisher_methodology(&base, context));
    }

    Some(generic_context_methodology(group_id, &base, context))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dairy_context_increases_intake_and_energy() {
        let base_context = AnimalContext {
            live_weight_kg: Some(600.0),
            milk_yield_kg: Some(30.0),
            milk_fat_pct: Some(3.8),
            ..Default::default()
        };
        let base = derive_norms_for_context("cattle_dairy_early_lact", Some(&base_context)).unwrap();

        let context = AnimalContext {
            live_weight_kg: Some(650.0),
            milk_yield_kg: Some(35.0),
            milk_fat_pct: Some(3.8),
            ..Default::default()
        };

        let derived = derive_norms_for_context("cattle_dairy_early_lact", Some(&context)).unwrap();

        assert!(derived.feed_intake_max.unwrap() > base.feed_intake_max.unwrap());
        assert!(
            derived.nutrients_target.get("energy_eke").unwrap()
                > base.nutrients_target.get("energy_eke").unwrap()
        );
        assert!(
            derived.nutrients_target.get("dig_protein_cattle").unwrap()
                > base.nutrients_target.get("dig_protein_cattle").unwrap()
        );
    }

    #[test]
    fn dairy_context_reduces_intake_and_energy_for_lower_milk_yield() {
        let base_context = AnimalContext {
            live_weight_kg: Some(600.0),
            milk_yield_kg: Some(30.0),
            milk_fat_pct: Some(3.8),
            ..Default::default()
        };
        let base = derive_norms_for_context("cattle_dairy_early_lact", Some(&base_context)).unwrap();

        let context = AnimalContext {
            live_weight_kg: Some(600.0),
            milk_yield_kg: Some(25.0),
            milk_fat_pct: Some(3.8),
            ..Default::default()
        };

        let derived = derive_norms_for_context("cattle_dairy_early_lact", Some(&context)).unwrap();

        assert!(derived.feed_intake_min.unwrap() < base.feed_intake_min.unwrap());
        assert!(
            derived.nutrients_target.get("energy_eke").unwrap()
                < base.nutrients_target.get("energy_eke").unwrap()
        );
        assert!(
            derived.nutrients_target.get("crude_protein").unwrap()
                < base.nutrients_target.get("crude_protein").unwrap()
        );
    }

    #[test]
    fn dairy_context_source_records_table_interpolation() {
        let context = AnimalContext {
            live_weight_kg: Some(640.0),
            milk_yield_kg: Some(33.0),
            milk_fat_pct: Some(4.1),
            breed: Some("Holstein".to_string()),
            ..Default::default()
        };

        let derived = derive_norms_for_context("cattle_dairy_fresh", Some(&context)).unwrap();

        assert!(derived
            .source
            .as_deref()
            .is_some_and(|source| source.contains("NASEM dairy table anchors")));
        assert!(derived.feed_intake_max.unwrap() > derived.feed_intake_min.unwrap());
    }

    #[test]
    fn swine_finisher_anchor_context_uses_weight_gain_interpolation() {
        let context = AnimalContext {
            live_weight_kg: Some(80.0),
            daily_gain_g: Some(900),
            sex: Some("mixed".to_string()),
            breed: Some("Duroc".to_string()),
            ..Default::default()
        };

        let derived = derive_norms_for_context("swine_finisher", Some(&context)).unwrap();

        assert_eq!(derived.nutrients_min.get("crude_protein"), Some(&140.0));
        assert_eq!(derived.nutrients_min.get("lysine_sid"), Some(&7.5));
        assert_eq!(derived.nutrients_target.get("lysine_sid"), Some(&7.5));
        assert_eq!(
            derived.nutrients_min.get("methionine_cystine_sid"),
            Some(&4.5)
        );
        assert_eq!(derived.nutrients_target.get("energy_oe_pig"), Some(&33.0));
        assert_eq!(derived.feed_intake_min, Some(2.53));
        assert_eq!(derived.feed_intake_max, Some(3.03));
        assert!(derived
            .source
            .as_deref()
            .is_some_and(|source| source.contains("NASEM swine weight-gain anchors")));
    }

    #[test]
    fn swine_finisher_context_raises_sid_density_for_lighter_faster_pigs() {
        let base = derive_norms_for_context(
            "swine_finisher",
            Some(&AnimalContext {
                live_weight_kg: Some(80.0),
                daily_gain_g: Some(900),
                sex: Some("mixed".to_string()),
                ..Default::default()
            }),
        )
        .unwrap();
        let derived = derive_norms_for_context(
            "swine_finisher",
            Some(&AnimalContext {
                live_weight_kg: Some(60.0),
                daily_gain_g: Some(1100),
                sex: Some("mixed".to_string()),
                ..Default::default()
            }),
        )
        .unwrap();

        assert!(
            derived.nutrients_target.get("lysine_sid").unwrap()
                > base.nutrients_target.get("lysine_sid").unwrap()
        );
        assert!(
            derived.nutrients_min.get("crude_protein").unwrap()
                > base.nutrients_min.get("crude_protein").unwrap()
        );
        assert!(derived.feed_intake_max.unwrap() <= base.feed_intake_max.unwrap());
    }

    #[test]
    fn swine_finisher_context_reduces_sid_density_for_heavier_slower_pigs() {
        let base = derive_norms_for_context(
            "swine_finisher",
            Some(&AnimalContext {
                live_weight_kg: Some(80.0),
                daily_gain_g: Some(900),
                sex: Some("mixed".to_string()),
                ..Default::default()
            }),
        )
        .unwrap();
        let derived = derive_norms_for_context(
            "swine_finisher",
            Some(&AnimalContext {
                live_weight_kg: Some(105.0),
                daily_gain_g: Some(700),
                sex: Some("mixed".to_string()),
                ..Default::default()
            }),
        )
        .unwrap();

        assert!(
            derived.nutrients_target.get("lysine_sid").unwrap()
                < base.nutrients_target.get("lysine_sid").unwrap()
        );
        assert!(
            derived.nutrients_min.get("crude_protein").unwrap()
                < base.nutrients_min.get("crude_protein").unwrap()
        );
        assert!(
            derived.nutrients_target.get("energy_oe_pig").unwrap()
                > base.nutrients_target.get("energy_oe_pig").unwrap()
        );
        assert!(derived.feed_intake_max.unwrap() > base.feed_intake_max.unwrap());
    }

    #[test]
    fn dairy_methodology_reports_driver_metrics_and_factors() {
        let methodology = describe_norm_methodology(
            "cattle_dairy_fresh",
            Some(&AnimalContext {
                live_weight_kg: Some(640.0),
                milk_yield_kg: Some(33.0),
                milk_fat_pct: Some(4.1),
                ..Default::default()
            }),
        )
        .unwrap();

        assert_eq!(methodology.key, "dairy_lactation_interpolation");
        assert!(methodology.dynamic);
        assert!(methodology
            .driver_metrics
            .iter()
            .any(|metric| metric.key == "milk_yield_kg" && metric.current_value == Some(33.0)));
        assert!(methodology
            .scaling_factors
            .iter()
            .any(|factor| factor.key == "energy_factor" && factor.value > 0.9));
    }

    #[test]
    fn reference_table_methodology_is_used_without_dynamic_inputs() {
        let methodology = describe_norm_methodology(
            "poultry_broiler_starter",
            Some(&AnimalContext {
                species: Some("poultry".to_string()),
                production_type: Some("broiler".to_string()),
                ..Default::default()
            }),
        )
        .unwrap();

        assert_eq!(methodology.key, "reference_table");
        assert!(!methodology.dynamic);
    }

    #[test]
    fn ratio_targets_are_not_scaled() {
        let base = get_norms_for_group("swine_grower").unwrap();
        let context = AnimalContext {
            live_weight_kg: Some(45.0),
            daily_gain_g: Some(780),
            ..Default::default()
        };

        let derived = derive_norms_for_context("swine_grower", Some(&context)).unwrap();

        assert_eq!(
            derived.nutrients_target.get("methionine_cystine_lys_ratio"),
            base.nutrients_target.get("methionine_cystine_lys_ratio")
        );
        assert_eq!(
            derived.nutrients_target.get("ca_p_ratio"),
            base.nutrients_target.get("ca_p_ratio")
        );
    }
}
