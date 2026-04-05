//! Beef cattle feeding norms.

use super::factorial::NutrientCalculator;
use super::{AnimalContext, AnimalNorm};
use std::collections::HashMap;

const SOURCE_WEIGHT_POINTS: [f64; 8] = [500.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 1100.0, 1200.0];
const SOURCE_DMI_POINTS: [f64; 8] = [10.5, 11.5, 12.4, 13.2, 13.9, 14.5, 15.1, 15.6];
const SOURCE_CP_POINTS: [f64; 8] = [
    1050.0, 1150.0, 1240.0, 1320.0, 1390.0, 1450.0, 1510.0, 1560.0,
];

fn round_norm_value(value: f64) -> f64 {
    if value.abs() >= 100.0 {
        value.round()
    } else {
        (value * 10.0).round() / 10.0
    }
}

fn insert_values(map: &mut HashMap<String, f64>, values: &[(&str, f64)]) {
    for (key, value) in values {
        map.insert((*key).to_string(), *value);
    }
}

/// Beef cattle energy calculator using NASEM 2016 net energy formulas.
pub struct BeefEnergyCalculator;

impl NutrientCalculator for BeefEnergyCalculator {
    fn maintenance(&self, ctx: &AnimalContext) -> f64 {
        let bw = ctx.live_weight_kg.unwrap_or(450.0).max(200.0);
        0.077 * bw.powf(0.75) * 4.184
    }

    fn production(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn growth(&self, ctx: &AnimalContext) -> f64 {
        let bw = ctx.live_weight_kg.unwrap_or(450.0).max(200.0);
        let adg_kg = ctx.daily_gain_g.unwrap_or(1_000).max(0) as f64 / 1000.0;

        if adg_kg <= 0.0 {
            return 0.0;
        }

        0.0557 * bw.powf(0.75) * adg_kg.powf(1.097) * 4.184
    }

    fn gestation(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn min_max_margin(&self) -> (f64, f64) {
        (0.95, 1.10)
    }
}

fn linear_interpolate(weight: f64, weights: &[f64], values: &[f64]) -> f64 {
    if weight <= weights[0] {
        return values[0];
    }

    for index in 0..(weights.len() - 1) {
        let left = weights[index];
        let right = weights[index + 1];
        if weight <= right {
            let ratio = (weight - left) / (right - left);
            return values[index] + (values[index + 1] - values[index]) * ratio;
        }
    }

    values[values.len() - 1]
}

fn clamp_weight(weight_kg: f64) -> f64 {
    weight_kg.clamp(200.0, 1200.0)
}

fn weight_window(weight_kg: f64) -> (f64, f64) {
    let weight_kg = clamp_weight(weight_kg);
    (
        (weight_kg - 50.0).max(200.0),
        (weight_kg + 50.0).min(1200.0),
    )
}

fn legacy_stocker() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();

    min.insert("energy_eke".to_string(), 7.0);
    target.insert("energy_eke".to_string(), 8.0);

    min.insert("crude_protein".to_string(), 950.0);
    target.insert("crude_protein".to_string(), 1100.0);
    min.insert("dig_protein_cattle".to_string(), 650.0);
    target.insert("dig_protein_cattle".to_string(), 750.0);

    AnimalNorm {
        id: "cattle_beef_stocker".to_string(),
        species: "cattle".to_string(),
        production_type: Some("beef".to_string()),
        weight_from_kg: Some(200.0),
        weight_to_kg: Some(300.0),
        daily_gain_g: Some(800),
        nutrients_min: min,
        nutrients_max: HashMap::new(),
        nutrients_target: target,
        feed_intake_min: Some(6.0),
        feed_intake_max: Some(8.0),
        source: Some("Kalashnikov 2003".to_string()),
        ..Default::default()
    }
}

fn apply_group_identity(
    mut norm: AnimalNorm,
    id: String,
    weight_kg: f64,
    daily_gain_g: i32,
) -> AnimalNorm {
    let (weight_from_kg, weight_to_kg) = weight_window(weight_kg);
    norm.id = id;
    norm.weight_from_kg = Some(weight_from_kg);
    norm.weight_to_kg = Some(weight_to_kg);
    norm.daily_gain_g = Some(daily_gain_g);
    norm
}

fn daily_gain_for_weight(weight_kg: f64) -> i32 {
    match clamp_weight(weight_kg) {
        weight if weight < 350.0 => 800,
        weight if weight < 500.0 => 1_000,
        weight if weight < 700.0 => 950,
        weight if weight < 900.0 => 900,
        weight if weight < 1000.0 => 800,
        weight if weight < 1100.0 => 750,
        _ => 700,
    }
}

/// Calculate total beef energy in MJ/day and requirement bounds.
pub fn calculate_beef_energy(ctx: &AnimalContext) -> (f64, f64, f64) {
    let calculator = BeefEnergyCalculator;
    let target = calculator.total(ctx);
    let min = calculator.min_requirement(ctx);
    let max = calculator.max_requirement(ctx);
    (min, target, max)
}

/// Get norms for beef cattle at a specific live weight.
pub fn get_norms_for_weight(weight_kg: f64, daily_gain_g: i32) -> AnimalNorm {
    let weight_kg = clamp_weight(weight_kg);

    if weight_kg < 330.0 {
        return apply_group_identity(
            legacy_stocker(),
            format!("cattle_beef_{:.0}", weight_kg),
            weight_kg,
            daily_gain_g,
        );
    }

    // For 330+ kg: use factorial norm calculation (full nutrient set)
    // Interpolation tables start at 500 kg; clamp low weights to 500 for the formula
    let interp_weight = weight_kg.max(500.0);

    let ctx = AnimalContext {
        species: Some("cattle".to_string()),
        production_type: Some("beef".to_string()),
        live_weight_kg: Some(weight_kg),
        daily_gain_g: Some(daily_gain_g),
        ..Default::default()
    };

    let (energy_min_mj, energy_target_mj, energy_max_mj) = calculate_beef_energy(&ctx);
    let cp_target = linear_interpolate(interp_weight, &SOURCE_WEIGHT_POINTS, &SOURCE_CP_POINTS);
    let dmi_target = linear_interpolate(interp_weight, &SOURCE_WEIGHT_POINTS, &SOURCE_DMI_POINTS);
    let digestible_protein_target = cp_target * 0.69;
    let calcium_target = ((0.045 * weight_kg) + ((daily_gain_g as f64 / 1000.0) * 14.0)) / 0.38;
    let phosphorus_target = ((0.035 * weight_kg) + ((daily_gain_g as f64 / 1000.0) * 8.5)) / 0.58;
    let mineral_factor = dmi_target / 11.5;
    let vitamin_factor = dmi_target / 11.5;
    let calcium_min = (calcium_target * 0.95 * 10.0).round() / 10.0;
    let phosphorus_min = (phosphorus_target * 0.95 * 10.0).round() / 10.0;
    let calcium_target_rounded = (calcium_target * 10.0).round() / 10.0;
    let phosphorus_target_rounded = (phosphorus_target * 10.0).round() / 10.0;
    let ca_p_ratio = round_norm_value(calcium_target_rounded / phosphorus_target_rounded.max(1.0));

    let mut min = HashMap::new();
    let mut max = HashMap::new();
    let mut target = HashMap::new();

    min.insert(
        "energy_eke".to_string(),
        (energy_min_mj / 10.47 * 100.0).round() / 100.0,
    );
    target.insert(
        "energy_eke".to_string(),
        (energy_target_mj / 10.47 * 100.0).round() / 100.0,
    );
    max.insert(
        "energy_eke".to_string(),
        (energy_max_mj / 10.47 * 100.0).round() / 100.0,
    );
    target.insert("energy_oe_cattle".to_string(), energy_target_mj.round());

    min.insert("crude_protein".to_string(), (cp_target * 0.95).round());
    target.insert("crude_protein".to_string(), cp_target.round());
    min.insert(
        "dig_protein_cattle".to_string(),
        (digestible_protein_target * 0.95).round(),
    );
    target.insert(
        "dig_protein_cattle".to_string(),
        digestible_protein_target.round(),
    );

    min.insert("calcium".to_string(), calcium_min);
    target.insert("calcium".to_string(), calcium_target_rounded);
    max.insert(
        "calcium".to_string(),
        (calcium_target * 1.10 * 10.0).round() / 10.0,
    );

    min.insert("phosphorus".to_string(), phosphorus_min);
    target.insert("phosphorus".to_string(), phosphorus_target_rounded);
    max.insert(
        "phosphorus".to_string(),
        (phosphorus_target * 1.10 * 10.0).round() / 10.0,
    );

    insert_values(
        &mut min,
        &[
            ("crude_fiber", round_norm_value(dmi_target * 270.0)),
            ("crude_fat", round_norm_value(dmi_target * 45.0)),
            ("starch", round_norm_value(dmi_target * 260.0)),
            ("sugar", round_norm_value(dmi_target * 35.0)),
            ("magnesium", round_norm_value(15.0 * mineral_factor)),
            ("potassium", round_norm_value(90.0 * mineral_factor)),
            ("sodium", round_norm_value(12.0 * mineral_factor)),
            ("iron", round_norm_value(850.0 * mineral_factor)),
            ("copper", round_norm_value(115.0 * mineral_factor)),
            ("zinc", round_norm_value(520.0 * mineral_factor)),
            ("manganese", round_norm_value(500.0 * mineral_factor)),
            ("iodine", round_norm_value(6.5 * mineral_factor)),
            ("cobalt", round_norm_value(4.5 * mineral_factor)),
            ("carotene", round_norm_value(105.0 * vitamin_factor)),
            ("vit_d3", round_norm_value(7500.0 * vitamin_factor)),
            ("vit_e", round_norm_value(320.0 * vitamin_factor)),
            ("ca_p_ratio", round_norm_value(ca_p_ratio * 0.98)),
            ("lysine", round_norm_value(digestible_protein_target * 0.95 * 0.072)),
            ("methionine_cystine", round_norm_value(digestible_protein_target * 0.95 * 0.024 * 1.55)),
        ],
    );
    insert_values(
        &mut target,
        &[
            ("crude_fiber", round_norm_value(dmi_target * 330.0)),
            ("crude_fat", round_norm_value(dmi_target * 50.0)),
            ("starch", round_norm_value(dmi_target * 300.0)),
            ("sugar", round_norm_value(dmi_target * 45.0)),
            ("magnesium", round_norm_value(20.0 * mineral_factor)),
            ("potassium", round_norm_value(105.0 * mineral_factor)),
            ("sodium", round_norm_value(18.0 * mineral_factor)),
            ("iron", round_norm_value(1000.0 * mineral_factor)),
            ("copper", round_norm_value(135.0 * mineral_factor)),
            ("zinc", round_norm_value(620.0 * mineral_factor)),
            ("manganese", round_norm_value(600.0 * mineral_factor)),
            ("iodine", round_norm_value(7.5 * mineral_factor)),
            ("cobalt", round_norm_value(6.0 * mineral_factor)),
            ("carotene", round_norm_value(120.0 * vitamin_factor)),
            ("vit_d3", round_norm_value(8200.0 * vitamin_factor)),
            ("vit_e", round_norm_value(360.0 * vitamin_factor)),
            ("ca_p_ratio", ca_p_ratio),
            ("lysine", round_norm_value(digestible_protein_target * 0.072)),
            ("methionine_cystine", round_norm_value(digestible_protein_target * 0.024 * 1.55)),
        ],
    );
    insert_values(
        &mut max,
        &[
            ("crude_fiber", round_norm_value(dmi_target * 420.0)),
            ("crude_fat", round_norm_value(dmi_target * 60.0)),
            ("starch", round_norm_value(dmi_target * 350.0)),
            ("sugar", round_norm_value(dmi_target * 65.0)),
        ],
    );

    let (weight_from_kg, weight_to_kg) = weight_window(weight_kg);

    AnimalNorm {
        id: format!("cattle_beef_{:.0}", weight_kg),
        species: "cattle".to_string(),
        production_type: Some("beef".to_string()),
        weight_from_kg: Some(weight_from_kg),
        weight_to_kg: Some(weight_to_kg),
        daily_gain_g: Some(daily_gain_g),
        nutrients_min: min,
        nutrients_max: max,
        nutrients_target: target,
        feed_intake_min: Some((dmi_target * 0.92 * 100.0).round() / 100.0),
        feed_intake_max: Some((dmi_target * 1.08 * 100.0).round() / 100.0),
        source: Some("NASEM Beef 2016".to_string()),
        ..Default::default()
    }
}

/// Get norms for a beef cattle group.
pub fn get_norms(group_id: &str) -> Option<AnimalNorm> {
    match group_id {
        "cattle_beef_stocker" => Some(legacy_stocker()),
        "cattle_beef_finisher" => Some(get_norms_for_weight(400.0, daily_gain_for_weight(400.0))),
        "cattle_beef_500" => Some(get_norms_for_weight(500.0, daily_gain_for_weight(500.0))),
        "cattle_beef_600" => Some(get_norms_for_weight(600.0, daily_gain_for_weight(600.0))),
        "cattle_beef_700" => Some(get_norms_for_weight(700.0, daily_gain_for_weight(700.0))),
        "cattle_beef_800" => Some(get_norms_for_weight(800.0, daily_gain_for_weight(800.0))),
        "cattle_beef_900" => Some(get_norms_for_weight(900.0, daily_gain_for_weight(900.0))),
        "cattle_beef_1000" => Some(get_norms_for_weight(1000.0, daily_gain_for_weight(1000.0))),
        "cattle_beef_1100" => Some(get_norms_for_weight(1100.0, daily_gain_for_weight(1100.0))),
        "cattle_beef_1200" => Some(get_norms_for_weight(1200.0, daily_gain_for_weight(1200.0))),
        _ => group_id
            .strip_prefix("cattle_beef_")
            .and_then(|weight| weight.parse::<f64>().ok())
            .map(|weight| get_norms_for_weight(weight, daily_gain_for_weight(weight)))
            .or_else(|| Some(get_norms_for_weight(450.0, daily_gain_for_weight(450.0)))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn norm_bar_key_count(norm: &AnimalNorm) -> usize {
        let mut keys = HashSet::new();
        keys.extend(norm.nutrients_min.keys().cloned());
        keys.extend(norm.nutrients_target.keys().cloned());
        if norm.feed_intake_min.is_some() || norm.feed_intake_max.is_some() {
            keys.insert("dry_matter_intake".to_string());
        }
        keys.len()
    }

    #[test]
    fn beef_norms_include_digestible_protein_targets() {
        let finisher = get_norms("cattle_beef_finisher").unwrap();
        let stocker = get_norms("cattle_beef_stocker").unwrap();

        // Finisher now uses factorial norms (full nutrient set)
        assert!(finisher.nutrients_min.contains_key("dig_protein_cattle"));
        assert!(finisher.nutrients_target.contains_key("dig_protein_cattle"));
        // Verify finisher has minerals/vitamins (not just 5 legacy keys)
        assert!(finisher.nutrients_min.contains_key("zinc"));
        assert!(finisher.nutrients_min.contains_key("vit_e"));

        assert_eq!(
            stocker.nutrients_min.get("dig_protein_cattle"),
            Some(&650.0)
        );
        assert_eq!(
            stocker.nutrients_target.get("dig_protein_cattle"),
            Some(&750.0)
        );
    }

    #[test]
    fn beef_energy_matches_nasem_formula_at_800kg() {
        let ctx = AnimalContext {
            live_weight_kg: Some(800.0),
            daily_gain_g: Some(900),
            ..Default::default()
        };

        let (_, target, _) = calculate_beef_energy(&ctx);

        assert!(
            (target - 79.9).abs() < 1.0,
            "expected about 79.9 MJ/day, got {target}"
        );
    }

    #[test]
    fn beef_energy_handles_1100kg_finishers() {
        let ctx = AnimalContext {
            live_weight_kg: Some(1100.0),
            daily_gain_g: Some(700),
            ..Default::default()
        };

        let (_, target, _) = calculate_beef_energy(&ctx);

        assert!(
            target > 80.0,
            "expected heavy finisher energy above 80 MJ/day, got {target}"
        );
    }

    #[test]
    fn get_norms_extended_range_uses_weight_specific_groups() {
        let norm_800 = get_norms("cattle_beef_800").unwrap();
        let norm_1100 = get_norms("cattle_beef_1100").unwrap();

        assert_eq!(norm_800.weight_from_kg, Some(750.0));
        assert_eq!(norm_800.weight_to_kg, Some(850.0));
        assert_eq!(norm_1100.weight_from_kg, Some(1050.0));
        assert_eq!(norm_1100.weight_to_kg, Some(1150.0));
        assert_eq!(
            norm_1100.nutrients_target.get("crude_protein"),
            Some(&1510.0)
        );
        assert!((25..=35).contains(&norm_bar_key_count(&norm_800)));
        assert!(norm_1100.nutrients_target.contains_key("ca_p_ratio"));
        assert!(norm_1100
            .nutrients_target
            .contains_key("methionine_cystine"));
    }
}
