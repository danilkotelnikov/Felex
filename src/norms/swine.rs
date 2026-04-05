//! Swine feeding norms
//! Based on: VIZh 2016, NRC Swine 2012

use super::factorial::NutrientCalculator;
use super::{AnimalContext, AnimalNorm, NutrientRequirement};
use std::collections::HashMap;

fn round_norm_value(value: f64) -> f64 {
    if value.abs() >= 100.0 {
        value.round()
    } else {
        (value * 100.0).round() / 100.0
    }
}

fn insert_values(map: &mut HashMap<String, f64>, values: &[(&str, f64)]) {
    for (key, value) in values {
        map.insert((*key).to_string(), *value);
    }
}

/// SID lysine calculator for growing and finishing swine.
pub struct SwineLysineCalculator;

impl NutrientCalculator for SwineLysineCalculator {
    fn maintenance(&self, ctx: &AnimalContext) -> f64 {
        let bw = ctx.live_weight_kg.unwrap_or(80.0);
        0.036 * bw
    }

    fn production(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn growth(&self, ctx: &AnimalContext) -> f64 {
        let adg_kg = ctx.daily_gain_g.unwrap_or(900).max(0) as f64 / 1000.0;
        let protein_deposition = adg_kg * 160.0;
        protein_deposition * 0.073
    }

    fn gestation(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn min_max_margin(&self) -> (f64, f64) {
        (0.95, 1.05)
    }
}

/// SID amino acid ratios relative to lysine.
pub const SID_MET_CYS_RATIO: f64 = 0.60;
pub const SID_THR_RATIO: f64 = 0.65;
pub const SID_TRP_RATIO: f64 = 0.19;

/// Calculate swine metabolizable energy in MJ/day using the VIZh practical formula.
pub fn calculate_swine_metabolizable_energy(ctx: &AnimalContext) -> f64 {
    let bw = ctx.live_weight_kg.unwrap_or(80.0).max(1.0);
    let adg_kg = ctx.daily_gain_g.unwrap_or(900).max(0) as f64 / 1000.0;

    0.44 * bw.powf(0.75) + adg_kg * 17.6
}

/// Calculate factorial SID lysine requirement in g/day.
pub fn calculate_sid_lysine(ctx: &AnimalContext) -> f64 {
    let calculator = SwineLysineCalculator;
    calculator.total(ctx)
}

/// Get norms for a swine group
pub fn get_norms(group_id: &str) -> Option<AnimalNorm> {
    match group_id {
        "swine_finisher" => Some(finisher()),
        "swine_sow_lactating" => Some(lactating_sow()),
        "swine_sow_gestating" => Some(gestating_sow()),
        "swine_piglet_nursery" => Some(nursery_piglet()),
        "swine_grower" => Some(grower()),
        _ => Some(finisher()),
    }
}

/// Finisher (55-110 kg, 900 g/day gain)
fn finisher() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();
    let max = HashMap::new();

    // Energy: OE 31-34 MJ/day
    min.insert("energy_oe_pig".to_string(), 31.0);
    target.insert("energy_oe_pig".to_string(), 33.0);

    // Protein and amino acids
    min.insert("crude_protein".to_string(), 140.0); // g/kg feed
    min.insert("lysine".to_string(), 7.5);
    target.insert("lysine".to_string(), 7.8);
    min.insert("lysine_sid".to_string(), 7.5);
    target.insert("lysine_sid".to_string(), 7.5); // g/kg feed
    min.insert("methionine_cystine".to_string(), 4.5);
    target.insert("methionine_cystine".to_string(), 4.5);
    min.insert("methionine_cystine_sid".to_string(), 4.5);
    target.insert("methionine_cystine_sid".to_string(), 4.5);
    target.insert("methionine_cystine_lys_ratio".to_string(), 0.600);

    // Minerals
    min.insert("calcium".to_string(), 6.5); // g/kg feed
    target.insert("calcium".to_string(), 6.8);
    min.insert("phosphorus".to_string(), 4.2);
    target.insert("phosphorus".to_string(), 4.8);
    min.insert("sodium".to_string(), 3.0);
    target.insert("sodium".to_string(), 4.0);
    min.insert("iron".to_string(), 160.0);
    target.insert("iron".to_string(), 200.0);
    min.insert("copper".to_string(), 20.0);
    target.insert("copper".to_string(), 30.0);
    min.insert("zinc".to_string(), 100.0);
    target.insert("zinc".to_string(), 150.0);
    min.insert("manganese".to_string(), 80.0);
    target.insert("manganese".to_string(), 100.0);
    min.insert("iodine".to_string(), 0.3);
    target.insert("iodine".to_string(), 0.5);
    target.insert("ca_p_ratio".to_string(), 1.42);
    insert_values(
        &mut min,
        &[
            ("vit_d3", 1000.0),
            ("vit_e", 80.0),
        ],
    );

    AnimalNorm {
        id: "swine_finisher".to_string(),
        species: "swine".to_string(),
        production_type: Some("fattening".to_string()),
        age_from_days: Some(120),
        age_to_days: Some(170),
        weight_from_kg: Some(55.0),
        weight_to_kg: Some(110.0),
        daily_gain_g: Some(900),
        nutrients_min: min,
        nutrients_max: max,
        nutrients_target: target,
        feed_intake_min: Some(2.5),
        feed_intake_max: Some(3.2),
        source: Some("VIZh 2016, NRC Swine 2012".to_string()),
        ..Default::default()
    }
}

/// Lactating sow (12 piglets)
fn lactating_sow() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();

    // Energy: 58-65 MJ OE/day
    min.insert("energy_oe_pig".to_string(), 58.0);
    target.insert("energy_oe_pig".to_string(), 62.0);

    // Lysine SID: 42 g/day
    min.insert("lysine_sid".to_string(), 38.0);
    target.insert("lysine_sid".to_string(), 42.0);
    target.insert("methionine_cystine_lys_ratio".to_string(), 0.591);

    // Calcium and phosphorus
    min.insert("calcium".to_string(), 32.0);
    target.insert("calcium".to_string(), 35.0);
    min.insert("phosphorus".to_string(), 30.0);
    target.insert("phosphorus".to_string(), 32.5);

    AnimalNorm {
        id: "swine_sow_lactating".to_string(),
        species: "swine".to_string(),
        production_type: Some("breeding".to_string()),
        weight_from_kg: Some(180.0),
        weight_to_kg: Some(220.0),
        nutrients_min: min,
        nutrients_max: HashMap::new(),
        nutrients_target: target,
        feed_intake_min: Some(5.5),
        feed_intake_max: Some(7.5),
        notes: Some("12 piglets".to_string()),
        source: Some("VIZh 2016".to_string()),
        ..Default::default()
    }
}

/// Gestating sow
fn gestating_sow() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();

    // Energy: moderate gestation demand, practical 30-36 MJ OE/day.
    min.insert("energy_oe_pig".to_string(), 30.0);
    target.insert("energy_oe_pig".to_string(), 33.0);

    // Lysine and supporting amino-acid ratios per day.
    min.insert("lysine_sid".to_string(), 10.5);
    target.insert("lysine_sid".to_string(), 12.0);
    target.insert("methionine_cystine_lys_ratio".to_string(), 0.625);

    // Calcium and phosphorus per day.
    min.insert("calcium".to_string(), 14.0);
    target.insert("calcium".to_string(), 16.0);
    min.insert("phosphorus".to_string(), 12.5);
    target.insert("phosphorus".to_string(), 15.0);

    AnimalNorm {
        id: "swine_sow_gestating".to_string(),
        species: "swine".to_string(),
        production_type: Some("breeding".to_string()),
        weight_from_kg: Some(180.0),
        weight_to_kg: Some(220.0),
        nutrients_min: min,
        nutrients_max: HashMap::new(),
        nutrients_target: target,
        feed_intake_min: Some(2.6),
        feed_intake_max: Some(3.4),
        notes: Some("Gestating sow".to_string()),
        source: Some("VIZh 2016, practical gestation target".to_string()),
        ..Default::default()
    }
}

/// Nursery piglet (8-25 kg)
fn nursery_piglet() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();

    min.insert("energy_oe_pig".to_string(), 14.0);
    target.insert("crude_protein_pct".to_string(), 20.0);
    min.insert("lysine".to_string(), 12.0);
    target.insert("lysine".to_string(), 13.5);
    target.insert("lysine_sid_pct".to_string(), 1.35);
    min.insert("methionine_cystine".to_string(), 7.2);
    target.insert("methionine_cystine".to_string(), 8.1);
    target.insert("methionine_cystine_lys_ratio".to_string(), 0.600);
    insert_values(
        &mut min,
        &[
            ("calcium", 8.0),
            ("phosphorus", 6.0),
            ("sodium", 2.0),
            ("iron", 180.0),
            ("copper", 25.0),
            ("zinc", 120.0),
            ("manganese", 90.0),
            ("iodine", 0.3),
            ("vit_d3", 800.0),
            ("vit_e", 60.0),
            ("ca_p_ratio", 1.33),
        ],
    );
    insert_values(
        &mut target,
        &[
            ("calcium", 10.0),
            ("phosphorus", 8.0),
            ("sodium", 3.0),
            ("iron", 220.0),
            ("copper", 35.0),
            ("zinc", 170.0),
            ("manganese", 120.0),
            ("iodine", 0.5),
            ("ca_p_ratio", 1.25),
        ],
    );

    AnimalNorm {
        id: "swine_piglet_nursery".to_string(),
        species: "swine".to_string(),
        production_type: Some("fattening".to_string()),
        age_from_days: Some(28),
        age_to_days: Some(70),
        weight_from_kg: Some(8.0),
        weight_to_kg: Some(25.0),
        daily_gain_g: Some(450),
        nutrients_min: min,
        nutrients_max: HashMap::new(),
        nutrients_target: target,
        feed_intake_min: Some(0.6),
        feed_intake_max: Some(1.2),
        source: Some("NRC Swine 2012".to_string()),
        ..Default::default()
    }
}

/// Grower (25-55 kg)
fn grower() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();
    let lysine_g_per_kg = 11.0;

    min.insert("energy_oe_pig".to_string(), 24.0);
    target.insert("crude_protein_pct".to_string(), 18.0);
    min.insert("lysine".to_string(), 10.5);
    target.insert("lysine".to_string(), lysine_g_per_kg);
    target.insert("lysine_sid_pct".to_string(), 1.1);
    min.insert(
        "methionine_cystine".to_string(),
        round_norm_value(lysine_g_per_kg * 0.600),
    );
    target.insert(
        "methionine_cystine".to_string(),
        round_norm_value(lysine_g_per_kg * 0.600),
    );
    target.insert("methionine_cystine_lys_ratio".to_string(), 0.600);
    insert_values(
        &mut min,
        &[
            ("calcium", 9.0),
            ("phosphorus", 7.0),
            ("sodium", 2.5),
            ("iron", 170.0),
            ("copper", 22.0),
            ("zinc", 110.0),
            ("manganese", 85.0),
            ("iodine", 0.3),
            ("vit_d3", 900.0),
            ("vit_e", 70.0),
            ("ca_p_ratio", 1.29),
        ],
    );
    insert_values(
        &mut target,
        &[
            ("calcium", 12.0),
            ("phosphorus", 9.0),
            ("sodium", 4.0),
            ("iron", 210.0),
            ("copper", 32.0),
            ("zinc", 160.0),
            ("manganese", 110.0),
            ("iodine", 0.5),
            ("ca_p_ratio", 1.33),
        ],
    );

    AnimalNorm {
        id: "swine_grower".to_string(),
        species: "swine".to_string(),
        production_type: Some("fattening".to_string()),
        age_from_days: Some(70),
        age_to_days: Some(120),
        weight_from_kg: Some(25.0),
        weight_to_kg: Some(55.0),
        daily_gain_g: Some(700),
        nutrients_min: min,
        nutrients_max: HashMap::new(),
        nutrients_target: target,
        feed_intake_min: Some(1.5),
        feed_intake_max: Some(2.2),
        source: Some("VIZh 2016".to_string()),
        ..Default::default()
    }
}

/// Required nutrients for swine
pub fn required_nutrients() -> Vec<NutrientRequirement> {
    vec![
        NutrientRequirement {
            key: "energy_oe_pig".to_string(),
            name_ru: "ОЭ свиней".to_string(),
            name_en: "ME swine".to_string(),
            unit: "МДж".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "lysine_sid".to_string(),
            name_ru: "Лизин SID".to_string(),
            name_en: "Lysine SID".to_string(),
            unit: "г".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "methionine_cystine_sid".to_string(),
            name_ru: "Мет+Цис SID".to_string(),
            name_en: "Met+Cys SID".to_string(),
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
            keys.insert("feed_intake".to_string());
        }
        keys.len()
    }

    #[test]
    fn sid_lysine_matches_factorial_formula_for_grower() {
        let ctx = AnimalContext {
            live_weight_kg: Some(60.0),
            daily_gain_g: Some(900),
            ..Default::default()
        };

        let lysine = calculate_sid_lysine(&ctx);

        assert!(
            (lysine - 12.672).abs() < 0.05,
            "expected about 12.67 g/day SID lysine, got {lysine}"
        );
    }

    #[test]
    fn swine_profiles_cover_checkpoint_two_rows() {
        let grower = get_norms("swine_grower").unwrap();
        let finisher = get_norms("swine_finisher").unwrap();
        let nursery = get_norms("swine_piglet_nursery").unwrap();

        // Reduced range: unsupported amino-acid ratio targets were removed.
        assert!((16..=25).contains(&norm_bar_key_count(&grower)));
        assert!((16..=25).contains(&norm_bar_key_count(&finisher)));
        assert!(nursery
            .nutrients_target
            .contains_key("methionine_cystine_lys_ratio"));
        // vit_b12 assertion removed — B vitamins no longer in norms
    }

    #[test]
    fn swine_metabolizable_energy_scales_with_weight_and_gain() {
        let grower = calculate_swine_metabolizable_energy(&AnimalContext {
            live_weight_kg: Some(60.0),
            daily_gain_g: Some(900),
            ..Default::default()
        });
        let finisher = calculate_swine_metabolizable_energy(&AnimalContext {
            live_weight_kg: Some(100.0),
            daily_gain_g: Some(900),
            ..Default::default()
        });

        assert!(
            grower > 25.0 && grower < 30.0,
            "expected grower ME near 27 MJ/day, got {grower}"
        );
        assert!(
            finisher > grower,
            "expected heavier finisher to require more ME"
        );
    }
}
