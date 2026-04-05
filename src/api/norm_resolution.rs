use crate::norms::{self, AnimalContext, AnimalNorm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ResolveAnimalProperties {
    pub species: Option<String>,
    pub production_type: Option<String>,
    pub breed: Option<String>,
    pub sex: Option<String>,
    pub live_weight_kg: Option<f64>,
    pub age_from_days: Option<i32>,
    pub age_to_days: Option<i32>,
    pub milk_yield_kg: Option<f64>,
    pub milk_fat_pct: Option<f64>,
    pub daily_gain_g: Option<f64>,
    pub egg_production_per_year: Option<f64>,
    pub litter_size: Option<f64>,
    pub reproductive_stage: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NormResolveRequest {
    pub norm_preset_id: Option<String>,
    pub animal_properties: Option<ResolveAnimalProperties>,
}

pub fn backend_group_for_norm_preset(preset_id: &str) -> Option<&'static str> {
    match preset_id {
        "cattle_dairy_20" | "cattle_dairy_25" | "cattle_dairy_30" => {
            Some("cattle_dairy_early_lact")
        }
        "cattle_dairy_35" => Some("cattle_dairy_fresh"),
        "swine_starter" => Some("swine_piglet_nursery"),
        "swine_grower" => Some("swine_grower"),
        "swine_finisher_preset" => Some("swine_finisher"),
        "swine_sow_gestation" => Some("swine_sow_gestating"),
        "swine_sow_lactation" => Some("swine_sow_lactating"),
        "poultry_broiler_starter" => Some("poultry_broiler_starter"),
        "poultry_broiler_grower" => Some("poultry_broiler_grower"),
        "poultry_broiler_finisher" => Some("poultry_broiler_finisher"),
        "poultry_layer_phase1" | "poultry_layer_phase2" => Some("poultry_layer_peak"),
        _ => None,
    }
}

pub fn backend_group_for_animal_context(
    base_group_id: &str,
    properties: &ResolveAnimalProperties,
) -> Option<&'static str> {
    match base_group_id {
        "cattle_dairy" => properties.milk_yield_kg.map(|milk_yield| {
            if milk_yield >= 33.0 {
                "cattle_dairy_fresh"
            } else {
                "cattle_dairy_early_lact"
            }
        }),
        "cattle_beef" => {
            let age_days = properties.age_to_days.or(properties.age_from_days);
            if properties
                .live_weight_kg
                .is_some_and(|weight| weight < 330.0)
                || age_days.is_some_and(|age| age < 365)
            {
                Some("cattle_beef_stocker")
            } else if properties
                .live_weight_kg
                .is_some_and(|weight| weight >= 1150.0)
            {
                Some("cattle_beef_1200")
            } else if properties
                .live_weight_kg
                .is_some_and(|weight| weight >= 1050.0)
            {
                Some("cattle_beef_1100")
            } else if properties
                .live_weight_kg
                .is_some_and(|weight| weight >= 950.0)
            {
                Some("cattle_beef_1000")
            } else if properties
                .live_weight_kg
                .is_some_and(|weight| weight >= 850.0)
            {
                Some("cattle_beef_900")
            } else if properties
                .live_weight_kg
                .is_some_and(|weight| weight >= 750.0)
            {
                Some("cattle_beef_800")
            } else if properties
                .live_weight_kg
                .is_some_and(|weight| weight >= 650.0)
            {
                Some("cattle_beef_700")
            } else if properties
                .live_weight_kg
                .is_some_and(|weight| weight >= 550.0)
            {
                Some("cattle_beef_600")
            } else if properties
                .live_weight_kg
                .is_some_and(|weight| weight >= 500.0)
            {
                Some("cattle_beef_500")
            } else {
                Some("cattle_beef_finisher")
            }
        }
        "swine_finisher" => {
            let age_days = properties.age_to_days.or(properties.age_from_days);
            if properties
                .live_weight_kg
                .is_some_and(|weight| weight < 25.0)
                || age_days.is_some_and(|age| age <= 70)
            {
                Some("swine_piglet_nursery")
            } else if properties
                .live_weight_kg
                .is_some_and(|weight| weight < 55.0)
                || age_days.is_some_and(|age| age <= 120)
            {
                Some("swine_grower")
            } else {
                Some("swine_finisher")
            }
        }
        "swine_sow" => {
            if properties.reproductive_stage.as_deref() == Some("lactation") {
                Some("swine_sow_lactating")
            } else {
                Some("swine_sow_gestating")
            }
        }
        "poultry_broiler" => properties
            .age_to_days
            .or(properties.age_from_days)
            .map(|age| {
                if age <= 10 {
                    "poultry_broiler_starter"
                } else if age <= 25 {
                    "poultry_broiler_grower"
                } else {
                    "poultry_broiler_finisher"
                }
            }),
        "poultry_layer" => Some("poultry_layer_peak"),
        _ => None,
    }
}

pub fn resolved_norm_group_id(base_group_id: Option<&str>, req: &NormResolveRequest) -> String {
    if let Some(preset_id) = req.norm_preset_id.as_deref() {
        if let Some(group_id) = backend_group_for_norm_preset(preset_id) {
            return group_id.to_string();
        }
    }

    let base_group_id = base_group_id.unwrap_or("custom");

    if let Some(properties) = req.animal_properties.as_ref() {
        if let Some(group_id) = backend_group_for_animal_context(base_group_id, properties) {
            return group_id.to_string();
        }
    }

    base_group_id.to_string()
}

pub fn animal_context_from_properties(
    properties: Option<&ResolveAnimalProperties>,
) -> Option<AnimalContext> {
    properties.map(|properties| AnimalContext {
        species: properties.species.clone(),
        production_type: properties.production_type.clone(),
        breed: properties.breed.clone(),
        sex: properties.sex.clone(),
        live_weight_kg: properties.live_weight_kg,
        age_from_days: properties.age_from_days,
        age_to_days: properties.age_to_days,
        milk_yield_kg: properties.milk_yield_kg,
        milk_fat_pct: properties.milk_fat_pct,
        daily_gain_g: properties.daily_gain_g.map(|value| value.round() as i32),
        egg_production_per_year: properties.egg_production_per_year,
        litter_size: properties.litter_size,
        reproductive_stage: properties.reproductive_stage.clone(),
    })
}

pub fn resolved_default_norm(
    base_group_id: Option<&str>,
    req: &NormResolveRequest,
    resolved_group_id: Option<&str>,
) -> Option<AnimalNorm> {
    let context = animal_context_from_properties(req.animal_properties.as_ref());
    let resolved_group_id = resolved_group_id
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| resolved_norm_group_id(base_group_id, req));

    norms::derive_norms_for_context(&resolved_group_id, context.as_ref())
        .or_else(|| norms::get_norms_for_group(&resolved_group_id))
        .or_else(|| {
            base_group_id
                .and_then(|group_id| norms::derive_norms_for_context(group_id, context.as_ref()))
        })
        .or_else(|| base_group_id.and_then(norms::get_norms_for_group))
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
            keys.insert(
                if norm.species == "cattle" {
                    "dry_matter_intake"
                } else {
                    "feed_intake"
                }
                .to_string(),
            );
        }
        keys.len()
    }

    #[test]
    fn resolves_group_from_high_yield_dairy_preset() {
        let req = NormResolveRequest {
            norm_preset_id: Some("cattle_dairy_35".to_string()),
            animal_properties: Some(ResolveAnimalProperties {
                species: Some("cattle".to_string()),
                production_type: Some("dairy".to_string()),
                live_weight_kg: Some(640.0),
                milk_yield_kg: Some(35.0),
                milk_fat_pct: Some(3.8),
                ..Default::default()
            }),
        };

        assert_eq!(
            resolved_norm_group_id(Some("cattle_dairy"), &req),
            "cattle_dairy_fresh"
        );
    }

    #[test]
    fn resolved_default_norm_uses_dynamic_intake_window() {
        let req = NormResolveRequest {
            norm_preset_id: Some("cattle_dairy_35".to_string()),
            animal_properties: Some(ResolveAnimalProperties {
                species: Some("cattle".to_string()),
                production_type: Some("dairy".to_string()),
                breed: Some("Holstein".to_string()),
                sex: Some("female".to_string()),
                live_weight_kg: Some(650.0),
                milk_yield_kg: Some(35.0),
                milk_fat_pct: Some(3.8),
                ..Default::default()
            }),
        };

        let resolved_group = resolved_norm_group_id(Some("cattle_dairy"), &req);
        let resolved =
            resolved_default_norm(Some("cattle_dairy"), &req, Some(&resolved_group)).unwrap();
        let static_norm = norms::get_norms_for_group(&resolved_group).unwrap();

        assert!(resolved.feed_intake_max.unwrap() > static_norm.feed_intake_max.unwrap());
    }

    #[test]
    fn resolves_heavy_beef_to_weight_bucket() {
        let req = NormResolveRequest {
            animal_properties: Some(ResolveAnimalProperties {
                species: Some("cattle".to_string()),
                production_type: Some("beef".to_string()),
                live_weight_kg: Some(800.0),
                daily_gain_g: Some(900.0),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            resolved_norm_group_id(Some("cattle_beef"), &req),
            "cattle_beef_800"
        );
    }

    #[test]
    fn checkpoint_two_species_resolve_to_expected_status_bar_ranges() {
        let dairy = resolved_default_norm(
            Some("cattle_dairy"),
            &NormResolveRequest {
                norm_preset_id: Some("cattle_dairy_35".to_string()),
                animal_properties: Some(ResolveAnimalProperties {
                    species: Some("cattle".to_string()),
                    production_type: Some("dairy".to_string()),
                    live_weight_kg: Some(650.0),
                    milk_yield_kg: Some(35.0),
                    milk_fat_pct: Some(3.8),
                    ..Default::default()
                }),
            },
            Some("cattle_dairy_fresh"),
        )
        .unwrap();
        let beef = resolved_default_norm(
            Some("cattle_beef"),
            &NormResolveRequest {
                animal_properties: Some(ResolveAnimalProperties {
                    species: Some("cattle".to_string()),
                    production_type: Some("beef".to_string()),
                    live_weight_kg: Some(800.0),
                    daily_gain_g: Some(1_000.0),
                    ..Default::default()
                }),
                ..Default::default()
            },
            Some("cattle_beef_800"),
        )
        .unwrap();
        let swine = resolved_default_norm(
            Some("swine_finisher"),
            &NormResolveRequest {
                norm_preset_id: Some("swine_grower".to_string()),
                animal_properties: Some(ResolveAnimalProperties {
                    species: Some("swine".to_string()),
                    production_type: Some("fattening".to_string()),
                    live_weight_kg: Some(45.0),
                    daily_gain_g: Some(750.0),
                    ..Default::default()
                }),
            },
            Some("swine_grower"),
        )
        .unwrap();
        let broiler = resolved_default_norm(
            Some("poultry_broiler"),
            &NormResolveRequest {
                norm_preset_id: Some("poultry_broiler_finisher".to_string()),
                animal_properties: Some(ResolveAnimalProperties {
                    species: Some("poultry".to_string()),
                    production_type: Some("broiler".to_string()),
                    age_from_days: Some(28),
                    age_to_days: Some(28),
                    ..Default::default()
                }),
            },
            Some("poultry_broiler_finisher"),
        )
        .unwrap();
        let layer = resolved_default_norm(
            Some("poultry_layer"),
            &NormResolveRequest {
                norm_preset_id: Some("poultry_layer_phase1".to_string()),
                animal_properties: Some(ResolveAnimalProperties {
                    species: Some("poultry".to_string()),
                    production_type: Some("layer".to_string()),
                    egg_production_per_year: Some(320.0),
                    ..Default::default()
                }),
            },
            Some("poultry_layer_peak"),
        )
        .unwrap();

        assert!((24..=35).contains(&norm_bar_key_count(&dairy)));
        assert!((22..=33).contains(&norm_bar_key_count(&beef)));
        assert!((16..=25).contains(&norm_bar_key_count(&swine)));
        assert!((14..=22).contains(&norm_bar_key_count(&broiler)));
        assert!((14..=20).contains(&norm_bar_key_count(&layer)));
    }
}
