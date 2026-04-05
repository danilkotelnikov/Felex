//! Poultry feeding norms
//! Based on: VNITIP 2017, NRC Poultry 1994

use super::factorial::NutrientCalculator;
use super::{AnimalContext, AnimalNorm, NutrientRequirement};
use std::collections::HashMap;

fn insert_values(map: &mut HashMap<String, f64>, values: &[(&str, f64)]) {
    for (key, value) in values {
        map.insert((*key).to_string(), *value);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BroilerPhase {
    Starter,
    Grower,
    Finisher,
}

/// TID amino acid profile for broiler phases.
pub struct BroilerAminoAcids {
    pub phase: BroilerPhase,
}

impl BroilerAminoAcids {
    pub fn tid_lysine_pct(&self) -> f64 {
        match self.phase {
            BroilerPhase::Starter => 1.28,
            BroilerPhase::Grower => 1.15,
            BroilerPhase::Finisher => 1.00,
        }
    }

    pub fn tid_met_cys_pct(&self) -> f64 {
        match self.phase {
            BroilerPhase::Starter => 0.95,
            BroilerPhase::Grower => 0.86,
            BroilerPhase::Finisher => 0.74,
        }
    }
}

/// Layer calcium calculator based on shell output and body-weight maintenance.
pub struct LayerCalciumCalculator;

impl NutrientCalculator for LayerCalciumCalculator {
    fn maintenance(&self, ctx: &AnimalContext) -> f64 {
        let bw = ctx.live_weight_kg.unwrap_or(2.0);
        bw * 0.004
    }

    fn production(&self, ctx: &AnimalContext) -> f64 {
        let eggs_per_day = ctx.egg_production_per_year.unwrap_or(280.0) / 365.0;
        let egg_mass_g = 60.0;
        let shell_g = egg_mass_g * 0.10;
        let shell_calcium_g = (shell_g * 0.40) / 0.50;

        eggs_per_day * shell_calcium_g
    }

    fn growth(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn gestation(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn min_max_margin(&self) -> (f64, f64) {
        (0.90, 1.15)
    }
}

pub fn get_broiler_phase(age_days: i32) -> BroilerPhase {
    if age_days <= 10 {
        BroilerPhase::Starter
    } else if age_days <= 24 {
        BroilerPhase::Grower
    } else {
        BroilerPhase::Finisher
    }
}

pub fn calculate_broiler_metabolizable_energy(age_days: i32) -> f64 {
    let me_kcal_per_kg = (3100 + age_days.max(0) * 15).min(3350) as f64;
    me_kcal_per_kg * 0.004184
}

/// Get norms for a poultry group
pub fn get_norms(group_id: &str) -> Option<AnimalNorm> {
    match group_id {
        "poultry_broiler_starter" => Some(broiler_starter()),
        "poultry_broiler_grower" => Some(broiler_grower()),
        "poultry_broiler_finisher" => Some(broiler_finisher()),
        "poultry_layer_peak" => Some(layer_peak()),
        _ => Some(broiler_finisher()),
    }
}

/// Broiler starter (0-10 days)
fn broiler_starter() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();

    // Energy: 12.3-12.5 MJ/kg
    min.insert("energy_oe_poultry".to_string(), 12.3);
    target.insert("energy_oe_poultry".to_string(), 12.4);

    // Protein: 22-23%
    min.insert("crude_protein_pct".to_string(), 22.0);
    target.insert("crude_protein_pct".to_string(), 22.5);
    min.insert("dig_protein_poultry".to_string(), 180.0);
    target.insert("dig_protein_poultry".to_string(), 185.0);

    // Amino acids (% of feed)
    min.insert("lysine_tid_pct".to_string(), 1.28);
    target.insert("lysine_tid_pct".to_string(), 1.35);
    min.insert("lysine".to_string(), 1.28);
    target.insert("lysine".to_string(), 1.35);
    min.insert("methionine_cystine".to_string(), 0.95);
    target.insert("methionine_cystine".to_string(), 1.0);
    min.insert("methionine_cystine_lys_ratio".to_string(), 0.74);
    target.insert("methionine_cystine_lys_ratio".to_string(), 0.741);

    // Minerals
    min.insert("calcium_pct".to_string(), 1.0);
    target.insert("calcium_pct".to_string(), 1.1);
    min.insert("phosphorus".to_string(), 7.2);
    target.insert("phosphorus".to_string(), 7.5);
    insert_values(
        &mut min,
        &[
            ("sodium", 0.16),
            ("iron", 80.0),
            ("copper", 8.0),
            ("zinc", 40.0),
            ("manganese", 60.0),
            ("iodine", 0.35),
            ("vit_d3", 600.0),
            ("vit_e", 40.0),
        ],
    );

    AnimalNorm {
        id: "poultry_broiler_starter".to_string(),
        species: "poultry".to_string(),
        production_type: Some("broiler".to_string()),
        age_from_days: Some(0),
        age_to_days: Some(10),
        nutrients_min: min,
        nutrients_max: HashMap::new(),
        nutrients_target: target,
        source: Some("VNITIP 2017".to_string()),
        feed_intake_min: Some(0.020),
        feed_intake_max: Some(0.045),
        ..Default::default()
    }
}

/// Broiler grower (11-25 days)
fn broiler_grower() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();

    min.insert("energy_oe_poultry".to_string(), 12.6);
    target.insert("energy_oe_poultry".to_string(), 12.8);

    min.insert("crude_protein_pct".to_string(), 20.0);
    target.insert("crude_protein_pct".to_string(), 21.0);
    min.insert("dig_protein_poultry".to_string(), 165.0);
    target.insert("dig_protein_poultry".to_string(), 173.0);

    min.insert("lysine_tid_pct".to_string(), 1.10);
    target.insert("lysine_tid_pct".to_string(), 1.15);
    min.insert("lysine".to_string(), 1.10);
    target.insert("lysine".to_string(), 1.15);
    min.insert("methionine_cystine".to_string(), 0.81);
    target.insert("methionine_cystine".to_string(), 0.85);
    min.insert("methionine_cystine_lys_ratio".to_string(), 0.74);
    target.insert("methionine_cystine_lys_ratio".to_string(), 0.783);

    min.insert("calcium_pct".to_string(), 0.9);
    target.insert("calcium_pct".to_string(), 1.0);
    min.insert("phosphorus".to_string(), 6.3);
    target.insert("phosphorus".to_string(), 6.9);
    insert_values(
        &mut min,
        &[
            ("sodium", 0.15),
            ("iron", 80.0),
            ("copper", 8.0),
            ("zinc", 40.0),
            ("manganese", 60.0),
            ("iodine", 0.35),
            ("vit_d3", 500.0),
            ("vit_e", 30.0),
        ],
    );

    AnimalNorm {
        id: "poultry_broiler_grower".to_string(),
        species: "poultry".to_string(),
        production_type: Some("broiler".to_string()),
        age_from_days: Some(11),
        age_to_days: Some(25),
        nutrients_min: min,
        nutrients_max: HashMap::new(),
        nutrients_target: target,
        source: Some("VNITIP 2017".to_string()),
        feed_intake_min: Some(0.070),
        feed_intake_max: Some(0.130),
        ..Default::default()
    }
}

/// Broiler finisher (26-42 days)
fn broiler_finisher() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();
    let mut max = HashMap::new();

    // Energy: 13.0-13.2 MJ/kg
    min.insert("energy_oe_poultry".to_string(), 13.0);
    target.insert("energy_oe_poultry".to_string(), 13.1);

    // Protein: 17-18%
    min.insert("crude_protein_pct".to_string(), 17.0);
    target.insert("crude_protein_pct".to_string(), 18.0);
    max.insert("crude_protein_pct".to_string(), 19.0);
    min.insert("dig_protein_poultry".to_string(), 140.0);
    target.insert("dig_protein_poultry".to_string(), 149.0);

    // Amino acids
    min.insert("lysine_tid_pct".to_string(), 0.90);
    target.insert("lysine_tid_pct".to_string(), 0.92);
    min.insert("lysine".to_string(), 0.90);
    target.insert("lysine".to_string(), 0.92);
    min.insert("methionine_cystine".to_string(), 0.67);
    target.insert("methionine_cystine".to_string(), 0.68);
    min.insert("methionine_cystine_lys_ratio".to_string(), 0.74);
    target.insert("methionine_cystine_lys_ratio".to_string(), 0.750);

    // Minerals
    min.insert("calcium_pct".to_string(), 0.85);
    target.insert("calcium_pct".to_string(), 0.90);
    min.insert("phosphorus".to_string(), 5.7);
    target.insert("phosphorus".to_string(), 6.3);
    insert_values(
        &mut min,
        &[
            ("sodium", 0.13),
            ("iron", 80.0),
            ("copper", 8.0),
            ("zinc", 40.0),
            ("manganese", 60.0),
            ("iodine", 0.35),
            ("vit_d3", 400.0),
            ("vit_e", 25.0),
        ],
    );

    AnimalNorm {
        id: "poultry_broiler_finisher".to_string(),
        species: "poultry".to_string(),
        production_type: Some("broiler".to_string()),
        age_from_days: Some(26),
        age_to_days: Some(42),
        nutrients_min: min,
        nutrients_max: max,
        nutrients_target: target,
        source: Some("VNITIP 2017".to_string()),
        feed_intake_min: Some(0.140),
        feed_intake_max: Some(0.210),
        ..Default::default()
    }
}

/// Laying hen at peak production
fn layer_peak() -> AnimalNorm {
    let mut min = HashMap::new();
    let mut target = HashMap::new();

    // Energy: 11.0-11.5 MJ/kg
    min.insert("energy_oe_poultry".to_string(), 11.0);
    target.insert("energy_oe_poultry".to_string(), 11.3);

    // Protein: 16-17%
    min.insert("crude_protein_pct".to_string(), 16.0);
    target.insert("crude_protein_pct".to_string(), 16.5);
    min.insert("dig_protein_poultry".to_string(), 132.0);
    target.insert("dig_protein_poultry".to_string(), 137.0);

    // Amino acids
    min.insert("lysine_tid_pct".to_string(), 0.72);
    target.insert("lysine_tid_pct".to_string(), 0.75);
    min.insert("lysine".to_string(), 0.72);
    target.insert("lysine".to_string(), 0.75);
    min.insert("methionine_cystine".to_string(), 0.59);
    target.insert("methionine_cystine".to_string(), 0.62);
    min.insert("methionine_cystine_lys_ratio".to_string(), 0.82);
    target.insert("methionine_cystine_lys_ratio".to_string(), 0.867);

    // Minerals (high calcium for egg production)
    min.insert("calcium_pct".to_string(), 3.6);
    target.insert("calcium_pct".to_string(), 4.0);
    min.insert("phosphorus".to_string(), 5.7);
    target.insert("phosphorus".to_string(), 6.0);
    insert_values(
        &mut min,
        &[
            ("sodium", 0.12),
            ("iron", 50.0),
            ("copper", 5.0),
            ("zinc", 50.0),
            ("manganese", 60.0),
            ("iodine", 0.35),
            ("vit_d3", 750.0),
            ("vit_e", 20.0),
        ],
    );

    AnimalNorm {
        id: "poultry_layer_peak".to_string(),
        species: "poultry".to_string(),
        production_type: Some("layer".to_string()),
        nutrients_min: min,
        nutrients_max: HashMap::new(),
        nutrients_target: target,
        notes: Some("Peak production, up to 40 weeks".to_string()),
        source: Some("VNITIP 2017".to_string()),
        feed_intake_min: Some(0.105),
        feed_intake_max: Some(0.125),
        ..Default::default()
    }
}

/// Required nutrients for poultry
pub fn required_nutrients() -> Vec<NutrientRequirement> {
    vec![
        NutrientRequirement {
            key: "energy_oe_poultry".to_string(),
            name_ru: "ОЭ птицы".to_string(),
            name_en: "ME poultry".to_string(),
            unit: "МДж/кг".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "crude_protein_pct".to_string(),
            name_ru: "Сырой протеин".to_string(),
            name_en: "Crude protein".to_string(),
            unit: "%".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "lysine_tid_pct".to_string(),
            name_ru: "Лизин TID".to_string(),
            name_en: "Lysine TID".to_string(),
            unit: "%".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "calcium_pct".to_string(),
            name_ru: "Кальций".to_string(),
            name_en: "Calcium".to_string(),
            unit: "%".to_string(),
            min_value: None,
            max_value: None,
            target_value: None,
        },
        NutrientRequirement {
            key: "phosphorus".to_string(),
            name_ru: "Фосфор общий".to_string(),
            name_en: "Total P".to_string(),
            unit: "г/кг".to_string(),
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
        keys.len()
    }

    #[test]
    fn poultry_norms_include_amino_ratio_targets() {
        let broiler = get_norms("poultry_broiler_grower").unwrap();
        let layer = get_norms("poultry_layer_peak").unwrap();

        for norms in [broiler, layer] {
            assert!(norms
                .nutrients_target
                .contains_key("methionine_cystine_lys_ratio"));
        }
    }

    #[test]
    fn broiler_lysine_declines_by_phase() {
        let starter = BroilerAminoAcids {
            phase: BroilerPhase::Starter,
        };
        let finisher = BroilerAminoAcids {
            phase: BroilerPhase::Finisher,
        };

        assert!(starter.tid_lysine_pct() > finisher.tid_lysine_pct());
        assert!((starter.tid_lysine_pct() - 1.28).abs() < 0.01);
        assert!((finisher.tid_met_cys_pct() - 0.74).abs() < 0.01);
    }

    #[test]
    fn broiler_me_caps_at_finisher_curve() {
        let day_28 = calculate_broiler_metabolizable_energy(28);
        let day_60 = calculate_broiler_metabolizable_energy(60);

        assert!(
            day_28 > 13.5,
            "expected broiler ME above 13.5 MJ/kg at day 28, got {day_28}"
        );
        assert!(
            (day_60 - 14.02).abs() < 0.05,
            "expected capped ME near 14.02 MJ/kg, got {day_60}"
        );
    }

    #[test]
    fn layer_calcium_tracks_egg_output() {
        let ctx = AnimalContext {
            species: Some("poultry".to_string()),
            production_type: Some("layer".to_string()),
            live_weight_kg: Some(2.0),
            egg_production_per_year: Some(300.0),
            ..Default::default()
        };
        let calcium = LayerCalciumCalculator.total(&ctx);

        assert!(
            calcium > 3.9 && calcium < 4.0,
            "expected layer calcium near 3.95 g/day, got {calcium}"
        );
    }

    #[test]
    fn poultry_profiles_cover_checkpoint_two_rows() {
        let broiler = get_norms("poultry_broiler_finisher").unwrap();
        let layer = get_norms("poultry_layer_peak").unwrap();

        assert!((16..=22).contains(&norm_bar_key_count(&broiler)));
        assert!((14..=20).contains(&norm_bar_key_count(&layer)));
        assert!(broiler.nutrients_target.contains_key("lysine"));
        assert!(layer.nutrients_min.contains_key("vit_d3"));
    }
}
