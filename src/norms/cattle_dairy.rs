//! Dairy cattle feeding norms
//! Based on: Kalashnikov et al. (2003), NRC Dairy (2001)

use super::{AnimalNorm, NutrientRequirement};
use std::collections::HashMap;

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

/// Get norms for a dairy cattle group
pub fn get_norms(group_id: &str) -> Option<AnimalNorm> {
    match group_id {
        "cattle_dairy_fresh" => Some(fresh_cow_35kg()),
        "cattle_dairy_early_lact" => Some(early_lactation_30kg()),
        "cattle_dairy_dry_early" => Some(dry_cow_early()),
        "cattle_dairy_heifer_12_18" => Some(heifer_12_18_months()),
        _ => {
            // Default dairy cow norm
            Some(early_lactation_30kg())
        }
    }
}

/// Fresh cow (0-60 days lactation, 35 kg milk)
fn fresh_cow_35kg() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut max = HashMap::new();
    let mut target = HashMap::new();

    insert_values(
        &mut min,
        &[
            ("energy_eke", 23.0),
            ("energy_oe_cattle", 247.0),
            ("crude_protein", 3200.0),
            ("dig_protein_cattle", 2350.0),
            ("dig_protein_cattle_pct_cp", 73.4),
            ("crude_fiber", 5800.0),
            ("starch", 4300.0),
            ("starch_pct_dm", 19.0),
            ("sugar", 1400.0),
            ("calcium", 140.0),
            ("phosphorus", 95.0),
            ("magnesium", 35.0),
            ("potassium", 140.0),
            ("sodium", 30.0),
            ("iron", 1200.0),
            ("copper", 175.0),
            ("zinc", 700.0),
            ("manganese", 700.0),
            ("cobalt", 6.0),
            ("iodine", 10.0),
            ("vit_d3", 25000.0),
            ("vit_e", 600.0),
            ("carotene", 200.0),
            ("ca_p_ratio", 1.47),
        ],
    );
    insert_values(
        &mut target,
        &[
            ("energy_eke", 24.0),
            ("energy_oe_cattle", 263.0),
            ("crude_protein", 3400.0),
            ("dig_protein_cattle", 2550.0),
            ("dig_protein_cattle_pct_cp", 75.0),
            ("crude_fiber", 6800.0),
            ("starch", 5200.0),
            ("starch_pct_dm", 23.0),
            ("sugar", 1800.0),
            ("calcium", 160.0),
            ("phosphorus", 108.0),
            ("magnesium", 45.0),
            ("potassium", 175.0),
            ("sodium", 42.0),
            ("iron", 1800.0),
            ("copper", 240.0),
            ("zinc", 1050.0),
            ("manganese", 1050.0),
            ("cobalt", 10.0),
            ("iodine", 15.0),
            ("vit_d3", 28000.0),
            ("vit_e", 650.0),
            ("carotene", 250.0),
            ("ca_p_ratio", 1.48),
        ],
    );
    insert_values(
        &mut max,
        &[
            ("crude_fiber", 7800.0),
            ("starch", 6500.0),
            ("starch_pct_dm", 30.0),
            ("sugar", 2800.0),
        ],
    );

    insert_values(
        &mut min,
        &[
            ("lysine", round_norm_value(2350.0 * 0.072)),
            ("methionine_cystine", round_norm_value(2350.0 * 0.024 * 1.55)),
        ],
    );
    insert_values(
        &mut target,
        &[
            ("lysine", round_norm_value(2550.0 * 0.072)),
            ("methionine_cystine", round_norm_value(2550.0 * 0.024 * 1.55)),
        ],
    );

    AnimalNorm {
        id: "cattle_dairy_fresh".to_string(),
        species: "cattle".to_string(),
        production_type: Some("dairy".to_string()),
        milk_yield_kg: Some(35.0),
        milk_fat_pct: Some(3.8),
        weight_from_kg: Some(600.0),
        weight_to_kg: Some(650.0),
        nutrients_min: min,
        nutrients_max: max,
        nutrients_target: target,
        feed_intake_min: Some(19.0),
        feed_intake_max: Some(23.0),
        source: Some("Kalashnikov 2003, NRC Dairy 2001".to_string()),
        ..Default::default()
    }
}

/// Early lactation (60-150 days, 30 kg milk)
fn early_lactation_30kg() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();
    let mut max = HashMap::new();

    insert_values(
        &mut min,
        &[
            ("energy_eke", 20.5),
            ("energy_oe_cattle", 215.0),
            ("crude_protein", 3050.0),
            ("dig_protein_cattle", 2050.0),
            ("dig_protein_cattle_pct_cp", 67.2),
            ("crude_fiber", 5500.0),
            ("starch", 4200.0),
            ("starch_pct_dm", 18.0),
            ("sugar", 1200.0),
            ("calcium", 120.0),
            ("phosphorus", 80.0),
            ("magnesium", 30.0),
            ("potassium", 120.0),
            ("sodium", 25.0),
            ("iron", 1000.0),
            ("copper", 150.0),
            ("zinc", 600.0),
            ("manganese", 600.0),
            ("cobalt", 5.0),
            ("iodine", 8.0),
            ("vit_d3", 20000.0),
            ("vit_e", 500.0),
            ("carotene", 180.0),
            ("ca_p_ratio", 1.5),
        ],
    );
    insert_values(
        &mut target,
        &[
            ("energy_eke", 21.5),
            ("energy_oe_cattle", 225.0),
            ("crude_protein", 3200.0),
            ("dig_protein_cattle", 2200.0),
            ("dig_protein_cattle_pct_cp", 68.8),
            ("crude_fiber", 6400.0),
            ("starch", 5000.0),
            ("starch_pct_dm", 22.0),
            ("sugar", 1700.0),
            ("calcium", 135.0),
            ("phosphorus", 90.0),
            ("magnesium", 40.0),
            ("potassium", 150.0),
            ("sodium", 35.0),
            ("iron", 1500.0),
            ("copper", 200.0),
            ("zinc", 900.0),
            ("manganese", 900.0),
            ("cobalt", 8.0),
            ("iodine", 12.0),
            ("vit_d3", 22000.0),
            ("vit_e", 550.0),
            ("carotene", 225.0),
            ("ca_p_ratio", 1.5),
        ],
    );
    insert_values(
        &mut max,
        &[
            ("crude_fiber", 7500.0),
            ("starch", 5500.0),
            ("starch_pct_dm", 27.0),
            ("sugar", 2500.0),
        ],
    );

    insert_values(
        &mut min,
        &[
            ("lysine", round_norm_value(2050.0 * 0.072)),
            ("methionine_cystine", round_norm_value(2050.0 * 0.024 * 1.55)),
        ],
    );
    insert_values(
        &mut target,
        &[
            ("lysine", round_norm_value(2200.0 * 0.072)),
            ("methionine_cystine", round_norm_value(2200.0 * 0.024 * 1.55)),
        ],
    );

    AnimalNorm {
        id: "cattle_dairy_early_lact".to_string(),
        species: "cattle".to_string(),
        production_type: Some("dairy".to_string()),
        milk_yield_kg: Some(30.0),
        milk_fat_pct: Some(3.8),
        weight_from_kg: Some(580.0),
        weight_to_kg: Some(620.0),
        nutrients_min: min,
        nutrients_max: max,
        nutrients_target: target,
        feed_intake_min: Some(18.0),
        feed_intake_max: Some(21.0),
        source: Some("Kalashnikov 2003".to_string()),
        ..Default::default()
    }
}

/// Dry cow early period (60-25 days before calving)
fn dry_cow_early() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();
    let mut max = HashMap::new();

    min.insert("energy_eke".to_string(), 10.5);
    target.insert("energy_eke".to_string(), 11.5);

    min.insert("crude_protein".to_string(), 1400.0);
    target.insert("crude_protein".to_string(), 1600.0);
    min.insert("dig_protein_cattle".to_string(), 950.0);
    target.insert("dig_protein_cattle".to_string(), 1050.0);

    max.insert("crude_fiber".to_string(), 300.0);

    min.insert("calcium".to_string(), 60.0);
    target.insert("calcium".to_string(), 70.0);

    min.insert("phosphorus".to_string(), 40.0);
    target.insert("phosphorus".to_string(), 50.0);

    AnimalNorm {
        id: "cattle_dairy_dry_early".to_string(),
        species: "cattle".to_string(),
        production_type: Some("dairy".to_string()),
        weight_from_kg: Some(600.0),
        weight_to_kg: Some(700.0),
        nutrients_min: min,
        nutrients_max: max,
        nutrients_target: target,
        feed_intake_min: Some(10.0),
        feed_intake_max: Some(13.0),
        source: Some("NRC Dairy 2001".to_string()),
        ..Default::default()
    }
}

/// Heifer 12-18 months (target gain 750 g/day)
fn heifer_12_18_months() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();

    min.insert("energy_eke".to_string(), 7.5);
    target.insert("energy_eke".to_string(), 8.5);

    min.insert("crude_protein".to_string(), 1100.0);
    target.insert("crude_protein".to_string(), 1250.0);
    min.insert("dig_protein_cattle".to_string(), 740.0);
    target.insert("dig_protein_cattle".to_string(), 840.0);

    min.insert("calcium".to_string(), 50.0);
    target.insert("calcium".to_string(), 60.0);

    min.insert("phosphorus".to_string(), 35.0);
    target.insert("phosphorus".to_string(), 40.0);

    AnimalNorm {
        id: "cattle_dairy_heifer_12_18".to_string(),
        species: "cattle".to_string(),
        production_type: Some("replacement".to_string()),
        age_from_days: Some(365),
        age_to_days: Some(548),
        weight_from_kg: Some(280.0),
        weight_to_kg: Some(380.0),
        daily_gain_g: Some(750),
        nutrients_min: min,
        nutrients_max: HashMap::new(),
        nutrients_target: target,
        feed_intake_min: Some(7.0),
        feed_intake_max: Some(9.0),
        source: Some("Kalashnikov 2003".to_string()),
        ..Default::default()
    }
}

/// Required nutrients for dairy cattle
pub fn required_nutrients() -> Vec<NutrientRequirement> {
    vec![
        NutrientRequirement {
            key: "energy_eke".to_string(),
            name_ru: "ЭКЕ".to_string(),
            name_en: "Energy units".to_string(),
            unit: "ед.".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "crude_protein".to_string(),
            name_ru: "Сырой протеин".to_string(),
            name_en: "Crude protein".to_string(),
            unit: "г".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "dig_protein_cattle".to_string(),
            name_ru: "Переваримый протеин".to_string(),
            name_en: "Digestible protein".to_string(),
            unit: "г".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "calcium".to_string(),
            name_ru: "Кальций".to_string(),
            name_en: "Calcium".to_string(),
            unit: "г".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "phosphorus".to_string(),
            name_ru: "Фосфор".to_string(),
            name_en: "Phosphorus".to_string(),
            unit: "г".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
    ]
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
    fn dairy_norms_include_digestible_protein_targets() {
        let fresh = get_norms("cattle_dairy_fresh").unwrap();
        let early = get_norms("cattle_dairy_early_lact").unwrap();
        let dry = get_norms("cattle_dairy_dry_early").unwrap();
        let heifer = get_norms("cattle_dairy_heifer_12_18").unwrap();

        assert_eq!(fresh.nutrients_min.get("dig_protein_cattle"), Some(&2350.0));
        assert_eq!(
            early.nutrients_target.get("dig_protein_cattle"),
            Some(&2200.0)
        );
        assert_eq!(dry.nutrients_min.get("dig_protein_cattle"), Some(&950.0));
        assert_eq!(
            heifer.nutrients_target.get("dig_protein_cattle"),
            Some(&840.0)
        );
    }

    #[test]
    fn fresh_and_early_lactation_cover_checkpoint_two_rows() {
        let fresh = get_norms("cattle_dairy_fresh").unwrap();
        let early = get_norms("cattle_dairy_early_lact").unwrap();

        // Reduced range: authority-backed set keeps crude_fat and carotene,
        // and excludes unsupported legacy energy/vitamin fields.
        // Net: removed ~6 unique keys, added 1 = ~5 fewer keys.
        assert!((24..=30).contains(&norm_bar_key_count(&fresh)));
        assert!(early.nutrients_target.contains_key("carotene"));
        assert!(early.nutrients_target.contains_key("lysine"));
    }
}
