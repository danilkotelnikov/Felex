use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::{
    db::{feeds::Feed, rations::RationItem},
    norms::AnimalNorm,
};

use super::feed_groups::{
    assess_feed_suitability, is_feed_allowed_for_context, matches_runtime_category,
    score_feed_for_group, template_for_group, FeedGroup, FeedSuitabilityStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPopulateItem {
    pub feed: Feed,
    pub amount_kg: f64,
    pub group: FeedGroup,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoPopulatePlan {
    pub items: Vec<AutoPopulateItem>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AutoPopulateCandidateSlot {
    pub group: FeedGroup,
    pub share: f64,
    pub candidates: Vec<AutoPopulateItem>,
}

pub fn build_auto_populate_plan(
    group_id: Option<&str>,
    norms: Option<&AnimalNorm>,
    feeds: &[Feed],
) -> AutoPopulatePlan {
    let (slots, mut notes) = build_auto_populate_candidate_slots(group_id, norms, feeds);
    let mut items = Vec::new();

    for slot in &slots {
        let diversity_count = diversity_count_for_group(slot.group);
        if diversity_count <= 1 || slot.candidates.len() <= 1 {
            if let Some(candidate) = slot.candidates.first() {
                items.push(candidate.clone());
            }
        } else {
            let take = diversity_count.min(slot.candidates.len());
            for candidate in slot.candidates.iter().take(take) {
                let mut item = candidate.clone();
                // Split the slot's amount equally among diverse feeds
                item.amount_kg = (candidate.amount_kg / take as f64 * 1000.0).round() / 1000.0;
                items.push(item);
            }
        }
    }

    if items.is_empty() {
        notes.push(
            "Starter ration could not be assembled from the current feed library.".to_string(),
        );
    }

    AutoPopulatePlan { items, notes }
}

pub(crate) fn build_auto_populate_candidate_slots(
    group_id: Option<&str>,
    norms: Option<&AnimalNorm>,
    feeds: &[Feed],
) -> (Vec<AutoPopulateCandidateSlot>, Vec<String>) {
    let stage_context = norms
        .map(|norm| norm.id.as_str())
        .filter(|id| !id.is_empty())
        .or(group_id);
    let species = norms
        .map(|norm| norm.species.as_str())
        .filter(|species| !species.is_empty())
        .unwrap_or_else(|| infer_species(group_id));
    let template = template_for_group(stage_context, species);
    let target_intake = default_total_intake(stage_context, norms, species);
    let uses_dm = species == "cattle";

    let mut notes = Vec::new();
    let mut slots = Vec::new();

    // Pre-compute suitability for all feeds once (avoid O(n log n) repeated calls)
    let suitability_cache: HashMap<i64, FeedSuitabilityStatus> = feeds
        .iter()
        .filter_map(|feed| {
            let id = feed.id?;
            let status = assess_feed_suitability(feed, species, stage_context).status;
            Some((id, status))
        })
        .collect();

    for share in template {
        let mut candidates = feeds
            .iter()
            .filter(|feed| feed.id.is_some())
            .filter(|feed| matches_runtime_category(feed, share.category))
            .filter(|feed| is_feed_allowed_for_context(feed, species, stage_context))
            .cloned()
            .collect::<Vec<_>>();

        candidates.sort_by(|left, right| {
            let left_suitability = left
                .id
                .and_then(|id| suitability_cache.get(&id).copied())
                .unwrap_or(FeedSuitabilityStatus::Restricted);
            let right_suitability = right
                .id
                .and_then(|id| suitability_cache.get(&id).copied())
                .unwrap_or(FeedSuitabilityStatus::Restricted);

            suitability_rank(left_suitability)
                .cmp(&suitability_rank(right_suitability))
                .then_with(|| {
                    score_feed_for_group(right, share.group, species)
                        .partial_cmp(&score_feed_for_group(left, share.group, species))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        let has_appropriate_candidate = candidates.iter().any(|feed| {
            feed.id
                .and_then(|id| suitability_cache.get(&id).copied())
                .map_or(false, |s| s == FeedSuitabilityStatus::Appropriate)
        });
        if has_appropriate_candidate {
            candidates.retain(|feed| {
                feed.id
                    .and_then(|id| suitability_cache.get(&id).copied())
                    .map_or(false, |s| s == FeedSuitabilityStatus::Appropriate)
            });
        }

        let candidate_items = candidates
            .into_iter()
            .take(candidate_limit_for_group(share.group))
            .filter_map(|feed| {
                let amount_kg = estimate_group_amount(
                    &feed,
                    share.group,
                    share.category,
                    share.share,
                    target_intake,
                    uses_dm,
                    species,
                );
                if amount_kg <= 0.0 {
                    return None;
                }

                Some(AutoPopulateItem {
                    reason: format!(
                        "Starter {} source for {}.",
                        share.category,
                        species
                    ),
                    feed,
                    amount_kg: (amount_kg * 1000.0).round() / 1000.0,
                    group: share.group,
                })
            })
            .collect::<Vec<_>>();

        if candidate_items.is_empty() {
            notes.push(format!(
                "No starter feed found for {}.",
                share.category
            ));
        }

        slots.push(AutoPopulateCandidateSlot {
            group: share.group,
            share: share.share,
            candidates: candidate_items,
        });
    }

    (slots, notes)
}

pub fn plan_to_ration_items(plan: &AutoPopulatePlan, ration_id: i64) -> Vec<RationItem> {
    plan.items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            Some(RationItem {
                id: None,
                ration_id,
                feed_id: item.feed.id?,
                feed: Some(item.feed.clone()),
                amount_kg: item.amount_kg,
                is_locked: false,
                sort_order: index as i32,
            })
        })
        .collect()
}

fn estimate_group_amount(
    feed: &Feed,
    group: FeedGroup,
    category: &str,
    share: f64,
    target_intake: f64,
    uses_dm: bool,
    species: &str,
) -> f64 {
    let dm_share = (feed.dry_matter.unwrap_or(86.0) / 100.0).clamp(0.1, 1.0);
    let base = if uses_dm {
        target_intake * share / dm_share
    } else {
        target_intake * share
    };

    if category == "additive" {
        return match species {
            "poultry" => base.clamp(0.001, 0.02),
            "swine" => base.clamp(0.005, 0.10),
            _ => base.clamp(0.02, 0.25),
        };
    }

    match group {
        FeedGroup::Mineral => match species {
            "poultry" => base.clamp(0.003, 0.025),
            "swine" => base.clamp(0.03, 0.20),
            _ => base.clamp(0.05, 0.25),
        },
        FeedGroup::Premix | FeedGroup::Vitamin => match species {
            "poultry" => base.clamp(0.002, 0.015),
            "swine" => base.clamp(0.01, 0.08),
            _ => base.clamp(0.02, 0.10),
        },
        FeedGroup::AnimalOrigin => base.clamp(0.02, 0.80),
        _ => base.max(0.05),
    }
}

fn infer_species(group_id: Option<&str>) -> &'static str {
    let group_id = group_id.unwrap_or_default();
    if group_id.starts_with("swine") {
        "swine"
    } else if group_id.starts_with("poultry") {
        "poultry"
    } else {
        "cattle"
    }
}

fn default_total_intake(group_id: Option<&str>, norms: Option<&AnimalNorm>, species: &str) -> f64 {
    if let Some(norm) = norms {
        if let (Some(min), Some(max)) = (norm.feed_intake_min, norm.feed_intake_max) {
            return (min + max) / 2.0;
        }
        if let Some(min) = norm.feed_intake_min {
            return min;
        }
        if let Some(max) = norm.feed_intake_max {
            return max;
        }
    }

    let group_id = group_id.unwrap_or_default();
    match species {
        "swine" if group_id.contains("sow") => 6.5,
        "swine" => 2.8,
        "poultry" if group_id.contains("layer") => 0.12,
        "poultry" => 0.10,
        _ if group_id.contains("beef") => 9.5,
        _ => 19.5,
    }
}

fn candidate_limit_for_group(group: FeedGroup) -> usize {
    match group {
        FeedGroup::Roughage
        | FeedGroup::Succulent
        | FeedGroup::Concentrate
        | FeedGroup::Protein => 3,
        FeedGroup::Mineral | FeedGroup::Premix | FeedGroup::Vitamin => 2,
        FeedGroup::AnimalOrigin => 2,
        FeedGroup::Other => 2,
    }
}

/// How many distinct feeds to include per group in the auto-populated ration.
/// Major groups get 2 feeds for diversity; minor groups keep 1.
fn diversity_count_for_group(group: FeedGroup) -> usize {
    match group {
        FeedGroup::Concentrate | FeedGroup::Protein => 2,
        FeedGroup::Roughage | FeedGroup::Succulent => 2,
        _ => 1,
    }
}

fn suitability_rank(status: FeedSuitabilityStatus) -> u8 {
    match status {
        FeedSuitabilityStatus::Appropriate => 0,
        FeedSuitabilityStatus::Conditional => 1,
        FeedSuitabilityStatus::Restricted => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::norms::AnimalContext;

    #[test]
    fn builds_non_empty_starter_for_dairy() {
        let feeds = vec![
            Feed {
                id: Some(1),
                name_ru: "Hay".to_string(),
                category: "roughage".to_string(),
                dry_matter: Some(87.0),
                energy_oe_cattle: Some(9.1),
                crude_fiber: Some(520.0),
                ..Default::default()
            },
            Feed {
                id: Some(2),
                name_ru: "Corn silage".to_string(),
                category: "silage".to_string(),
                dry_matter: Some(32.0),
                energy_oe_cattle: Some(10.7),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Barley".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(86.0),
                energy_oe_cattle: Some(12.4),
                ..Default::default()
            },
            Feed {
                id: Some(4),
                name_ru: "Soybean meal".to_string(),
                category: "oilseed_meal".to_string(),
                crude_protein: Some(430.0),
                ..Default::default()
            },
            Feed {
                id: Some(5),
                name_ru: "Feed chalk".to_string(),
                category: "mineral".to_string(),
                calcium: Some(360.0),
                ..Default::default()
            },
            Feed {
                id: Some(6),
                name_ru: "Premix P60".to_string(),
                category: "premix".to_string(),
                vit_d3: Some(250_000.0),
                vit_e: Some(2000.0),
                ..Default::default()
            },
        ];

        let plan = build_auto_populate_plan(Some("cattle_dairy"), None, &feeds);
        assert!(!plan.items.is_empty());
        assert!(plan
            .items
            .iter()
            .any(|item| item.group == FeedGroup::Roughage));
        assert!(plan
            .items
            .iter()
            .any(|item| item.group == FeedGroup::Concentrate));
    }

    #[test]
    fn dynamic_norms_increase_starter_amounts_for_high_yield_dairy() {
        let feeds = vec![
            Feed {
                id: Some(1),
                name_ru: "Hay".to_string(),
                category: "roughage".to_string(),
                dry_matter: Some(87.0),
                energy_oe_cattle: Some(9.1),
                crude_fiber: Some(520.0),
                ..Default::default()
            },
            Feed {
                id: Some(2),
                name_ru: "Corn silage".to_string(),
                category: "silage".to_string(),
                dry_matter: Some(32.0),
                energy_oe_cattle: Some(10.7),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Barley".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(86.0),
                energy_oe_cattle: Some(12.4),
                ..Default::default()
            },
            Feed {
                id: Some(4),
                name_ru: "Soybean meal".to_string(),
                category: "oilseed_meal".to_string(),
                crude_protein: Some(430.0),
                ..Default::default()
            },
            Feed {
                id: Some(5),
                name_ru: "Feed chalk".to_string(),
                category: "mineral".to_string(),
                calcium: Some(360.0),
                ..Default::default()
            },
            Feed {
                id: Some(6),
                name_ru: "Premix P60".to_string(),
                category: "premix".to_string(),
                vit_d3: Some(250_000.0),
                vit_e: Some(2000.0),
                ..Default::default()
            },
        ];

        let base = crate::norms::get_norms_for_group("cattle_dairy_early_lact").unwrap();
        let dynamic = crate::norms::derive_norms_for_context(
            "cattle_dairy_early_lact",
            Some(&AnimalContext {
                live_weight_kg: Some(650.0),
                milk_yield_kg: Some(35.0),
                milk_fat_pct: Some(3.8),
                ..Default::default()
            }),
        )
        .unwrap();

        let base_plan =
            build_auto_populate_plan(Some("cattle_dairy_early_lact"), Some(&base), &feeds);
        let dynamic_plan =
            build_auto_populate_plan(Some("cattle_dairy_early_lact"), Some(&dynamic), &feeds);

        let base_total = base_plan
            .items
            .iter()
            .map(|item| item.amount_kg)
            .sum::<f64>();
        let dynamic_total = dynamic_plan
            .items
            .iter()
            .map(|item| item.amount_kg)
            .sum::<f64>();
        let base_roughage = base_plan
            .items
            .iter()
            .find(|item| item.group == FeedGroup::Roughage)
            .map(|item| item.amount_kg)
            .unwrap_or_default();
        let dynamic_roughage = dynamic_plan
            .items
            .iter()
            .find(|item| item.group == FeedGroup::Roughage)
            .map(|item| item.amount_kg)
            .unwrap_or_default();

        assert!(dynamic_total > base_total);
        assert!(dynamic_roughage > base_roughage);
    }

    #[test]
    fn excludes_layer_shell_grit_from_broiler_starter_candidates() {
        let feeds = vec![
            Feed {
                id: Some(1),
                name_ru: "Corn".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(88.0),
                energy_oe_poultry: Some(12.7),
                ..Default::default()
            },
            Feed {
                id: Some(2),
                name_ru: "Soybean meal".to_string(),
                category: "oilseed_meal".to_string(),
                crude_protein: Some(430.0),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Ракушка кормовая для несушек".to_string(),
                category: "mineral".to_string(),
                subcategory: Some("layer_shell_grit".to_string()),
                calcium: Some(360.0),
                ..Default::default()
            },
            Feed {
                id: Some(4),
                name_ru: "Feed chalk".to_string(),
                category: "mineral".to_string(),
                calcium: Some(360.0),
                ..Default::default()
            },
            Feed {
                id: Some(5),
                name_ru: "Премикс для бройлеров 1%".to_string(),
                category: "premix".to_string(),
                vit_e: Some(8_000.0),
                ..Default::default()
            },
        ];

        let plan = build_auto_populate_plan(Some("poultry_broiler"), None, &feeds);

        assert!(plan
            .items
            .iter()
            .all(|item| item.feed.subcategory.as_deref() != Some("layer_shell_grit")));
        assert!(plan
            .items
            .iter()
            .any(|item| item.feed.name_ru == "Feed chalk"));
    }

    #[test]
    fn excludes_later_broiler_premix_when_norm_context_is_starter() {
        let feeds = vec![
            Feed {
                id: Some(1),
                name_ru: "Corn".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(88.0),
                energy_oe_poultry: Some(12.7),
                ..Default::default()
            },
            Feed {
                id: Some(2),
                name_ru: "Soybean meal".to_string(),
                category: "oilseed_meal".to_string(),
                crude_protein: Some(430.0),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Премикс для бройлеров старше 4 нед.".to_string(),
                category: "premix".to_string(),
                vit_e: Some(8_000.0),
                ..Default::default()
            },
            Feed {
                id: Some(4),
                name_ru: "Премикс для бройлеров 1-4 нед.".to_string(),
                category: "premix".to_string(),
                vit_e: Some(8_000.0),
                ..Default::default()
            },
        ];

        let norm = crate::norms::get_norms_for_group("poultry_broiler_starter").unwrap();
        let plan = build_auto_populate_plan(Some("poultry_broiler"), Some(&norm), &feeds);

        assert!(plan
            .items
            .iter()
            .all(|item| item.feed.name_ru != "Премикс для бройлеров старше 4 нед."));
        assert!(plan
            .items
            .iter()
            .any(|item| item.feed.name_ru == "Премикс для бройлеров 1-4 нед."));
    }

    #[test]
    fn prefers_appropriate_succulent_over_conditional_potato_for_blank_dairy() {
        let feeds = vec![
            Feed {
                id: Some(1),
                name_ru: "Кукурузный силос".to_string(),
                category: "silage".to_string(),
                dry_matter: Some(32.0),
                energy_oe_cattle: Some(10.7),
                ..Default::default()
            },
            Feed {
                id: Some(2),
                name_ru: "Из картофеля сырого".to_string(),
                category: "succulent".to_string(),
                dry_matter: Some(20.0),
                energy_oe_cattle: Some(11.0),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Сено луговое".to_string(),
                category: "roughage".to_string(),
                dry_matter: Some(87.0),
                energy_oe_cattle: Some(9.1),
                crude_fiber: Some(520.0),
                ..Default::default()
            },
            Feed {
                id: Some(4),
                name_ru: "Ячмень".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(86.0),
                energy_oe_cattle: Some(12.4),
                ..Default::default()
            },
            Feed {
                id: Some(5),
                name_ru: "Шрот соевый".to_string(),
                category: "oilseed_meal".to_string(),
                crude_protein: Some(430.0),
                ..Default::default()
            },
            Feed {
                id: Some(6),
                name_ru: "Фосфат обесфторенный".to_string(),
                category: "mineral".to_string(),
                phosphorus: Some(180.0),
                ..Default::default()
            },
            Feed {
                id: Some(7),
                name_ru: "Премикс для КРС".to_string(),
                category: "premix".to_string(),
                vit_d3: Some(250_000.0),
                vit_e: Some(2000.0),
                ..Default::default()
            },
        ];

        let norm = crate::norms::get_norms_for_group("cattle_dairy_fresh").unwrap();
        let plan = build_auto_populate_plan(Some("cattle_dairy"), Some(&norm), &feeds);

        let succulent = plan
            .items
            .iter()
            .find(|item| item.group == FeedGroup::Succulent)
            .map(|item| item.feed.name_ru.as_str());

        assert_eq!(succulent, Some("Кукурузный силос"));
    }

    #[test]
    fn prefers_adult_dairy_premix_over_calf_premix_for_fresh_cows() {
        let feeds = vec![
            Feed {
                id: Some(1),
                name_ru: "Hay".to_string(),
                category: "roughage".to_string(),
                dry_matter: Some(87.0),
                energy_oe_cattle: Some(9.1),
                crude_fiber: Some(520.0),
                ..Default::default()
            },
            Feed {
                id: Some(2),
                name_ru: "Corn silage".to_string(),
                category: "silage".to_string(),
                dry_matter: Some(32.0),
                energy_oe_cattle: Some(10.7),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Barley".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(86.0),
                energy_oe_cattle: Some(12.4),
                ..Default::default()
            },
            Feed {
                id: Some(4),
                name_ru: "Soybean meal".to_string(),
                category: "oilseed_meal".to_string(),
                crude_protein: Some(430.0),
                ..Default::default()
            },
            Feed {
                id: Some(5),
                name_ru: "Feed chalk".to_string(),
                category: "mineral".to_string(),
                calcium: Some(360.0),
                ..Default::default()
            },
            Feed {
                id: Some(6),
                name_ru: "Рецепты премиксов для молодняка крупного рогатого скота, на 1 тонну До 6-ти мес. возраста П 62-3-89".to_string(),
                category: "premix".to_string(),
                vit_d3: Some(230_000.0),
                vit_e: Some(1900.0),
                ..Default::default()
            },
            Feed {
                id: Some(7),
                name_ru: "Нормы ввода БАВ в премиксы для КРС Молочные коровы П 60-1 ( стойловый период)".to_string(),
                category: "premix".to_string(),
                vit_d3: Some(260_000.0),
                vit_e: Some(2100.0),
                ..Default::default()
            },
        ];

        let norm = crate::norms::get_norms_for_group("cattle_dairy_fresh").unwrap();
        let plan = build_auto_populate_plan(Some("cattle_dairy_fresh"), Some(&norm), &feeds);

        let premix = plan
            .items
            .iter()
            .find(|item| item.group == FeedGroup::Premix)
            .expect("expected a premix candidate for fresh dairy cows");

        assert!(premix.feed.name_ru.contains("Молочные коровы"));
        assert!(!premix.feed.name_ru.contains("До 6-ти мес."));
    }

    #[test]
    fn dairy_starter_uses_seven_or_more_feed_roles_when_library_supports_them() {
        let feeds = vec![
            Feed {
                id: Some(1),
                name_ru: "Hay".to_string(),
                category: "roughage".to_string(),
                dry_matter: Some(87.0),
                energy_oe_cattle: Some(9.1),
                crude_fiber: Some(520.0),
                ..Default::default()
            },
            Feed {
                id: Some(2),
                name_ru: "Haylage".to_string(),
                category: "silage".to_string(),
                dry_matter: Some(44.0),
                energy_oe_cattle: Some(10.5),
                ..Default::default()
            },
            Feed {
                id: Some(3),
                name_ru: "Beet".to_string(),
                category: "succulent".to_string(),
                dry_matter: Some(14.0),
                energy_oe_cattle: Some(8.5),
                ..Default::default()
            },
            Feed {
                id: Some(4),
                name_ru: "Barley".to_string(),
                category: "grain".to_string(),
                dry_matter: Some(86.0),
                energy_oe_cattle: Some(12.4),
                ..Default::default()
            },
            Feed {
                id: Some(5),
                name_ru: "Soybean meal".to_string(),
                category: "oilseed_meal".to_string(),
                crude_protein: Some(430.0),
                ..Default::default()
            },
            Feed {
                id: Some(6),
                name_ru: "Feed chalk".to_string(),
                category: "mineral".to_string(),
                calcium: Some(360.0),
                ..Default::default()
            },
            Feed {
                id: Some(7),
                name_ru: "Dairy premix".to_string(),
                category: "premix".to_string(),
                vit_d3: Some(220_000.0),
                vit_e: Some(2_000.0),
                ..Default::default()
            },
            Feed {
                id: Some(8),
                name_ru: "Propylene glycol".to_string(),
                category: "additive".to_string(),
                ..Default::default()
            },
        ];

        let plan = build_auto_populate_plan(Some("cattle_dairy_fresh"), None, &feeds);

        assert!(plan.items.len() >= 7, "starter roles: {:?}", plan.items);
    }
}
