pub mod categories;
pub mod matching;

use crate::db::feeds::Feed;
use serde::{Deserialize, Serialize};

pub use categories::preset_categories;
pub use matching::{match_preset_feeds, recommendation_label, MatchReasonParts};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresetAnimalParams {
    pub live_weight_kg: Option<f64>,
    pub daily_gain_g: Option<f64>,
    pub milk_yield_kg: Option<f64>,
    pub age_days: Option<i32>,
    pub age_weeks: Option<i32>,
    pub target_weight_g: Option<i32>,
    pub production_pct: Option<f64>,
    pub days_pregnant: Option<i32>,
    pub days_to_calving: Option<i32>,
    pub piglets: Option<i32>,
    pub lactation_stage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSubcategory {
    pub id: String,
    pub name_ru: String,
    pub name_en: String,
    pub animal_group_id: String,
    pub norm_preset_id: Option<String>,
    pub legacy_preset_id: Option<String>,
    pub params: PresetAnimalParams,
    pub research_source: Option<String>,
    pub feed_recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetCategory {
    pub species: String,
    pub production_type: String,
    pub subcategories: Vec<PresetSubcategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedMatch {
    pub feed: Feed,
    pub match_score: f64,
    pub match_reason: MatchReasonParts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetRecommendationMatches {
    pub key: String,
    pub label_ru: String,
    pub label_en: String,
    pub matches: Vec<FeedMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedPresetSubcategory {
    pub id: String,
    pub name_ru: String,
    pub name_en: String,
    pub animal_group_id: String,
    pub norm_preset_id: Option<String>,
    pub legacy_preset_id: Option<String>,
    pub params: PresetAnimalParams,
    pub research_source: Option<String>,
    pub recommendations: Vec<PresetRecommendationMatches>,
    pub matched_feed_count: usize,
    pub fully_matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedPresetCategory {
    pub species: String,
    pub production_type: String,
    pub subcategories: Vec<MatchedPresetSubcategory>,
}

pub fn matched_preset_categories(available_feeds: &[Feed]) -> Vec<MatchedPresetCategory> {
    preset_categories()
        .into_iter()
        .map(|category| MatchedPresetCategory {
            species: category.species,
            production_type: category.production_type,
            subcategories: category
                .subcategories
                .into_iter()
                .map(|preset| {
                    let recommendations = preset
                        .feed_recommendations
                        .iter()
                        .map(|recommendation_key| {
                            let (label_ru, label_en) = recommendation_label(recommendation_key);
                            let matches =
                                match_preset_feeds(recommendation_key, available_feeds, 5);
                            PresetRecommendationMatches {
                                key: recommendation_key.clone(),
                                label_ru: label_ru.to_string(),
                                label_en: label_en.to_string(),
                                matches,
                            }
                        })
                        .collect::<Vec<_>>();

                    let matched_feed_count = recommendations
                        .iter()
                        .map(|recommendation| recommendation.matches.len())
                        .sum();
                    let fully_matched = recommendations
                        .iter()
                        .all(|recommendation| !recommendation.matches.is_empty());

                    MatchedPresetSubcategory {
                        id: preset.id,
                        name_ru: preset.name_ru,
                        name_en: preset.name_en,
                        animal_group_id: preset.animal_group_id,
                        norm_preset_id: preset.norm_preset_id,
                        legacy_preset_id: preset.legacy_preset_id,
                        params: preset.params,
                        research_source: preset.research_source,
                        recommendations,
                        matched_feed_count,
                        fully_matched,
                    }
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sourced_catalog_contains_25_subcategories() {
        let count: usize = preset_categories()
            .iter()
            .map(|category| category.subcategories.len())
            .sum();
        assert_eq!(count, 25);
    }
}
