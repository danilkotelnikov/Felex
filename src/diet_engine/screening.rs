use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::{
    db::{feed_labels::display_feed_name, feeds::Feed, rations::RationItem},
    norms::AnimalNorm,
};

use super::{
    feed_groups::{
        classify_feed, is_feed_allowed_for_context, preferred_groups_for_nutrient,
        species_fit_bonus, stage_fit_bonus,
    },
    nutrient_calc::{self, NutrientSummary},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedRecommendation {
    pub feed_id: i64,
    pub feed_name: String,
    pub reason: String,
    pub suggested_amount_kg: f64,
    pub category: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScreeningReport {
    pub can_meet_reference: bool,
    pub limiting_nutrients: Vec<String>,
    pub recommendations: Vec<FeedRecommendation>,
}

pub fn screen_current_feed_set(
    items: &[RationItem],
    feeds: &[Feed],
    norms: &AnimalNorm,
) -> ScreeningReport {
    let summary = nutrient_calc::calculate_nutrients(items);
    let present_ids: HashSet<i64> = items.iter().map(|item| item.feed_id).collect();
    let baseline_total = baseline_total_weight(norms, &summary);

    let deficits = screening_targets(norms)
        .into_iter()
        .filter_map(|(key, label, priority)| {
            let required = norms
                .nutrients_target
                .get(key)
                .copied()
                .or_else(|| norms.nutrients_min.get(key).copied())?;
            let actual = nutrient_calc::metric_value_for_norm(&summary, norms, key)?;
            if actual + 1e-6 >= required {
                None
            } else {
                Some((key, label, priority, required - actual))
            }
        })
        .collect::<Vec<_>>();

    let mut limiting_nutrients = Vec::new();
    let mut picked = HashMap::<i64, FeedRecommendation>::new();

    for (key, label, priority, deficit) in deficits {
        limiting_nutrients.push(label.to_string());
        let Some(feed) = best_feed_for_gap(
            feeds,
            &present_ids,
            key,
            norms.species.as_str(),
            norms.id.as_str(),
        ) else {
            continue;
        };
        let suggested_amount_kg = estimate_suggested_amount(feed, key, deficit, baseline_total);
        let recommendation = FeedRecommendation {
            feed_id: feed.id.unwrap_or_default(),
            feed_name: display_feed_name(feed),
            reason: concise_reason(norms.species.as_str(), label),
            suggested_amount_kg: (suggested_amount_kg * 1000.0).round() / 1000.0,
            category: feed.category.clone(),
            priority,
        };

        picked
            .entry(recommendation.feed_id)
            .and_modify(|current| {
                if recommendation.priority < current.priority {
                    *current = recommendation.clone();
                }
            })
            .or_insert(recommendation);
    }

    let mut recommendations = picked.into_values().collect::<Vec<_>>();
    recommendations.sort_by_key(|item| item.priority);
    recommendations.truncate(3);

    ScreeningReport {
        can_meet_reference: limiting_nutrients.is_empty(),
        limiting_nutrients,
        recommendations,
    }
}

fn screening_label(key: &str) -> &'static str {
    match key {
        "energy_eke" | "energy_oe_cattle" | "energy_oe_pig" | "energy_oe_poultry" => "energy",
        "crude_protein" | "crude_protein_pct" => "protein",
        "dig_protein_cattle" => "digestible protein",
        "lysine_sid" | "lysine_sid_pct" | "lysine_tid_pct" => "lysine",
        "methionine_cystine_sid" | "methionine_cystine_tid_pct" => "methionine+cystine",
        "crude_fiber" | "crude_fiber_pct" => "crude fiber",
        "starch_pct_dm" => "starch",
        "calcium" | "calcium_pct" => "calcium",
        "phosphorus" | "phosphorus_pct" => "phosphorus",
        "vit_d3" => "vitamin D3",
        "vit_e" => "vitamin E",
        "sodium" => "sodium",
        "magnesium" => "magnesium",
        "potassium" => "potassium",
        _ => "nutrient",
    }
}

fn append_screening_targets(
    targets: &mut Vec<(&'static str, &'static str, u8)>,
    norms: &AnimalNorm,
    catalog: &[(&'static str, u8)],
) {
    for &(key, priority) in catalog {
        if norms.nutrients_target.contains_key(key) || norms.nutrients_min.contains_key(key) {
            targets.push((key, screening_label(key), priority));
        }
    }
}

pub(crate) fn screening_targets(norms: &AnimalNorm) -> Vec<(&'static str, &'static str, u8)> {
    let mut targets = Vec::new();

    match norms.species.as_str() {
        "swine" => {
            append_screening_targets(
                &mut targets,
                norms,
                &[
                    ("energy_oe_pig", 1),
                    ("crude_protein_pct", 1),
                    ("crude_protein", 1),
                    ("lysine_sid_pct", 1),
                    ("lysine_sid", 1),
                    ("methionine_cystine_sid", 2),
                    ("methionine_cystine_tid_pct", 2),
                    ("calcium", 2),
                    ("phosphorus", 2),
                    ("sodium", 3),
                    ("magnesium", 3),
                    ("potassium", 3),
                    ("iron", 3),
                    ("copper", 3),
                    ("zinc", 3),
                    ("manganese", 3),
                    ("cobalt", 3),
                    ("iodine", 3),
                    ("vit_d3", 3),
                    ("vit_e", 3),
                ],
            );
        }
        "poultry" => {
            append_screening_targets(
                &mut targets,
                norms,
                &[
                    ("energy_oe_poultry", 1),
                    ("crude_protein_pct", 1),
                    ("lysine_tid_pct", 1),
                    ("methionine_cystine_tid_pct", 2),
                    ("calcium_pct", 2),
                    ("phosphorus", 2),
                    ("sodium", 3),
                    ("iron", 3),
                    ("copper", 3),
                    ("zinc", 3),
                    ("manganese", 3),
                    ("iodine", 3),
                    ("vit_d3", 3),
                    ("vit_e", 3),
                ],
            );
        }
        _ => {
            append_screening_targets(
                &mut targets,
                norms,
                &[
                    ("energy_eke", 1),
                    ("crude_protein", 1),
                    ("dig_protein_cattle", 1),
                    ("crude_fiber", 2),
                    ("calcium", 2),
                    ("phosphorus", 2),
                    ("starch_pct_dm", 3),
                    ("iron", 3),
                    ("copper", 3),
                    ("zinc", 3),
                    ("manganese", 3),
                    ("cobalt", 3),
                    ("iodine", 3),
                    ("vit_d3", 3),
                    ("vit_e", 3),
                ],
            );
        }
    }

    targets
}

fn best_feed_for_gap<'a>(
    feeds: &'a [Feed],
    present_ids: &HashSet<i64>,
    key: &str,
    species: &str,
    stage_context: &str,
) -> Option<&'a Feed> {
    let preferred_groups = preferred_groups_for_nutrient(key, species);

    feeds
        .iter()
        .filter(|feed| feed.id.is_some())
        .filter(|feed| {
            if let Some(id) = feed.id {
                !present_ids.contains(&id)
            } else {
                false
            }
        })
        .filter(|feed| is_feed_allowed_for_context(feed, species, Some(stage_context)))
        .filter(|feed| preferred_groups.contains(&classify_feed(feed)))
        .filter(|feed| nutrient_density(feed, key) > 0.0)
        .max_by(|left, right| {
            (nutrient_density(left, key)
                + species_fit_bonus(left, species)
                + stage_fit_bonus(left, stage_context))
            .partial_cmp(
                &(nutrient_density(right, key)
                    + species_fit_bonus(right, species)
                    + stage_fit_bonus(right, stage_context)),
            )
            .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn concise_reason(species: &str, label: &str) -> String {
    let species_label = match species {
        "cattle" => "for cattle",
        "swine" => "for swine",
        "poultry" => "for poultry",
        _ => "",
    };

    if species_label.is_empty() {
        format!("{label} support.")
    } else {
        format!("{label} support {species_label}.")
    }
}

pub(crate) fn nutrient_density(feed: &Feed, key: &str) -> f64 {
    let dm_share = (feed.dry_matter.unwrap_or(86.0) / 100.0).clamp(0.1, 1.0);
    match key {
        "energy_eke" => feed.energy_oe_cattle.unwrap_or(0.0) * dm_share / 10.5,
        "energy_oe_cattle" => feed.energy_oe_cattle.unwrap_or(0.0) * dm_share,
        "energy_oe_pig" => feed.energy_oe_pig.unwrap_or(0.0) * dm_share,
        "energy_oe_poultry" => feed.energy_oe_poultry.unwrap_or(0.0),
        "crude_protein" => feed.crude_protein.unwrap_or(0.0),
        "crude_protein_pct" => feed.crude_protein.unwrap_or(0.0) / 10.0,
        "dig_protein_cattle" => feed.dig_protein_cattle.unwrap_or(0.0),
        "dig_protein_cattle_pct_cp" => {
            let crude_protein = feed.crude_protein.unwrap_or(0.0);
            if crude_protein > 0.0 {
                (feed.dig_protein_cattle.unwrap_or(0.0) / crude_protein) * 100.0
            } else {
                0.0
            }
        }
        "lysine" | "lysine_sid" => feed.lysine.unwrap_or(0.0),
        "lysine_sid_pct" | "lysine_tid_pct" => feed.lysine.unwrap_or(0.0) / 10.0,
        "methionine_cystine" | "methionine_cystine_sid" => feed.methionine_cystine.unwrap_or(0.0),
        "methionine_cystine_tid_pct" => feed.methionine_cystine.unwrap_or(0.0) / 10.0,
        "crude_fiber" => feed.crude_fiber.unwrap_or(0.0),
        "crude_fiber_pct" => feed.crude_fiber.unwrap_or(0.0) / 10.0,
        "calcium" => feed.calcium.unwrap_or(0.0),
        "calcium_pct" => feed.calcium.unwrap_or(0.0) / 10.0,
        "phosphorus" => feed.phosphorus.unwrap_or(0.0),
        "phosphorus_pct" => feed.phosphorus.unwrap_or(0.0) / 10.0,
        "vit_d3" => feed.vit_d3.unwrap_or(0.0),
        "vit_e" => feed.vit_e.unwrap_or(0.0),
        _ => feed.nutrient_value(key),
    }
}

pub(crate) fn estimate_suggested_amount(
    feed: &Feed,
    key: &str,
    deficit: f64,
    baseline_total: f64,
) -> f64 {
    let density = nutrient_density(feed, key);
    if density <= 0.0 {
        return 0.0;
    }

    match key {
        "crude_protein_pct"
        | "lysine_sid_pct"
        | "lysine_tid_pct"
        | "methionine_cystine_tid_pct"
        | "calcium_pct"
        | "crude_fiber_pct"
        | "phosphorus_pct"
        | "energy_oe_poultry" => (baseline_total * 0.05).clamp(0.01, 1.0),
        _ => (deficit / density).clamp(0.01, 3.0),
    }
}

pub(crate) fn baseline_total_weight(norms: &AnimalNorm, summary: &NutrientSummary) -> f64 {
    if summary.total_weight_kg > 0.0 {
        summary.total_weight_kg
    } else {
        match norms.species.as_str() {
            "swine" => 3.0,
            "poultry" => 0.12,
            _ => 25.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_calcium_recommendation_for_layer_ration() {
        let items = vec![RationItem {
            id: Some(1),
            ration_id: 1,
            feed_id: 1,
            feed: Some(Feed {
                id: Some(1),
                name_ru: "Wheat".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(88.0),
                energy_oe_poultry: Some(12.4),
                crude_protein: Some(120.0),
                calcium: Some(0.8),
                phosphorus: Some(3.0),
                ..Default::default()
            }),
            amount_kg: 0.1,
            is_locked: false,
            sort_order: 0,
        }];

        let feeds = vec![Feed {
            id: Some(2),
            name_ru: "Feed chalk".to_string(),
            category: "mineral".to_string(),
            calcium: Some(360.0),
            ..Default::default()
        }];

        let mut norm = AnimalNorm {
            species: "poultry".to_string(),
            ..Default::default()
        };
        norm.nutrients_min.insert("calcium_pct".to_string(), 3.6);

        let report = screen_current_feed_set(&items, &feeds, &norm);
        assert!(!report.recommendations.is_empty());
        assert_eq!(report.recommendations[0].feed_id, 2);
    }

    #[test]
    fn uses_supported_swine_phosphorus_screening() {
        let items = vec![RationItem {
            id: Some(1),
            ration_id: 1,
            feed_id: 1,
            feed: Some(Feed {
                id: Some(1),
                name_ru: "Barley".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(88.0),
                calcium: Some(7.0),
                phosphorus: Some(4.6),
                lysine: Some(7.5),
                crude_protein: Some(160.0),
                ..Default::default()
            }),
            amount_kg: 2.0,
            is_locked: false,
            sort_order: 0,
        }];

        let norm = AnimalNorm {
            id: "swine_finisher".to_string(),
            species: "swine".to_string(),
            nutrients_min: HashMap::from([
                ("calcium".to_string(), 6.5),
                ("phosphorus".to_string(), 4.2),
            ]),
            nutrients_target: HashMap::from([("lysine_sid".to_string(), 7.5)]),
            ..Default::default()
        };

        let report = screen_current_feed_set(&items, &[], &norm);
        assert!(report.can_meet_reference);
        assert!(report.limiting_nutrients.is_empty());
    }

    #[test]
    fn recommends_phosphorus_source_for_poultry() {
        let items = vec![RationItem {
            id: Some(1),
            ration_id: 1,
            feed_id: 1,
            feed: Some(Feed {
                id: Some(1),
                name_ru: "Corn".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(88.0),
                energy_oe_poultry: Some(12.5),
                crude_protein: Some(90.0),
                calcium: Some(0.6),
                phosphorus: Some(3.0),
                ..Default::default()
            }),
            amount_kg: 0.1,
            is_locked: false,
            sort_order: 0,
        }];

        let feeds = vec![Feed {
            id: Some(2),
            name_ru: "Monocalcium phosphate".to_string(),
            category: "mineral".to_string(),
            phosphorus: Some(220.0),
            ..Default::default()
        }];

        let mut norm = AnimalNorm {
            id: "poultry_broiler_test".to_string(),
            species: "poultry".to_string(),
            ..Default::default()
        };
        norm.nutrients_min.insert("phosphorus".to_string(), 6.0);

        let report = screen_current_feed_set(&items, &feeds, &norm);
        assert!(!report.can_meet_reference);
        assert_eq!(report.limiting_nutrients, vec!["phosphorus"]);
        assert_eq!(report.recommendations[0].feed_id, 2);
    }

    #[test]
    fn prefers_species_appropriate_premix_for_cattle() {
        let items = vec![RationItem {
            id: Some(1),
            ration_id: 1,
            feed_id: 1,
            feed: Some(Feed {
                id: Some(1),
                name_ru: "Hay".to_string(),
                category: "roughage".to_string(),
                dry_matter: Some(88.0),
                ..Default::default()
            }),
            amount_kg: 10.0,
            is_locked: false,
            sort_order: 0,
        }];

        let feeds = vec![
            Feed {
                id: Some(2),
                name_ru: "Премикс для кур-несушек".to_string(),
                category: "premix".to_string(),
                vit_d3: Some(300_000.0),
                vit_e: Some(8_000.0),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Премикс П60-1 для КРС".to_string(),
                category: "premix".to_string(),
                vit_d3: Some(200_000.0),
                vit_e: Some(5_000.0),
                ..Default::default()
            },
        ];

        let norm = AnimalNorm {
            id: "cattle_dairy".to_string(),
            species: "cattle".to_string(),
            nutrients_min: HashMap::from([("vit_d3".to_string(), 20_000_000.0)]),
            ..Default::default()
        };

        let report = screen_current_feed_set(&items, &feeds, &norm);
        assert!(!report.recommendations.is_empty());
        assert_eq!(report.recommendations[0].feed_id, 3);
        assert_eq!(
            report.recommendations[0].reason,
            "vitamin D3 support for cattle."
        );
    }

    #[test]
    fn prefers_stage_appropriate_swine_premix() {
        let items = vec![RationItem {
            id: Some(1),
            ration_id: 1,
            feed_id: 1,
            feed: Some(Feed {
                id: Some(1),
                name_ru: "Barley".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(88.0),
                calcium: Some(0.7),
                ..Default::default()
            }),
            amount_kg: 3.0,
            is_locked: false,
            sort_order: 0,
        }];

        let feeds = vec![
            Feed {
                id: Some(2),
                name_ru: "Премикс стартер для поросят".to_string(),
                category: "premix".to_string(),
                calcium: Some(35.0),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Премикс для свиноматок".to_string(),
                category: "premix".to_string(),
                calcium: Some(30.0),
                ..Default::default()
            },
        ];

        let norm = AnimalNorm {
            id: "swine_sow_lactating".to_string(),
            species: "swine".to_string(),
            nutrients_min: HashMap::from([("calcium".to_string(), 6.5)]),
            ..Default::default()
        };

        let report = screen_current_feed_set(&items, &feeds, &norm);
        assert!(!report.recommendations.is_empty());
        assert_eq!(report.recommendations[0].feed_id, 3);
    }

    #[test]
    fn supports_cattle_digestible_protein_metric() {
        let norm = AnimalNorm {
            id: "cattle_dairy_early_lact".to_string(),
            species: "cattle".to_string(),
            ..Default::default()
        };
        let summary = NutrientSummary {
            dig_protein_cattle: 2150.0,
            dig_protein_cattle_pct_cp: 67.2,
            ..Default::default()
        };
        let feed = Feed {
            id: Some(9),
            name_ru: "Soybean meal".to_string(),
            category: "protein".to_string(),
            crude_protein: Some(440.0),
            dig_protein_cattle: Some(340.0),
            ..Default::default()
        };

        assert_eq!(
            nutrient_calc::metric_value_for_norm(&summary, &norm, "dig_protein_cattle"),
            Some(2150.0)
        );
        assert_eq!(nutrient_density(&feed, "dig_protein_cattle"), 340.0);
    }
}
