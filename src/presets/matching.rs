use super::FeedMatch;
use crate::db::feeds::Feed;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Structured match reason data for frontend localization.
/// Frontend formats this using locale keys instead of displaying raw strings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchReasonParts {
    /// The name fragment that matched (e.g., "силос кукуруз")
    pub name_fragment: Option<String>,
    /// The feed category that matched (e.g., "silage")
    pub category: Option<String>,
    /// The nutrient key that was checked (e.g., "dry_matter")
    pub nutrient_key: Option<String>,
    /// The nutrient value that matched the criteria
    pub nutrient_value: Option<f64>,
    /// Whether the feed is from a verified source
    pub verified: bool,
}

struct MatchRule {
    label_ru: &'static str,
    label_en: &'static str,
    name_fragments: &'static [&'static str],
    categories: &'static [&'static str],
    nutrient_key: Option<&'static str>,
    nutrient_min: Option<f64>,
    nutrient_max: Option<f64>,
}

fn rule_for(recommendation_key: &str) -> MatchRule {
    match recommendation_key {
        "corn_silage" => MatchRule {
            label_ru: "Кукурузный силос",
            label_en: "Corn Silage",
            name_fragments: &["силос кукуруз", "corn silage"],
            categories: &["silage"],
            nutrient_key: Some("dry_matter"),
            nutrient_min: Some(25.0),
            nutrient_max: Some(40.0),
        },
        "alfalfa_hay" => MatchRule {
            label_ru: "Люцерновое сено",
            label_en: "Alfalfa Hay",
            name_fragments: &["люцерн", "alfalfa"],
            categories: &["roughage"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(140.0),
            nutrient_max: None,
        },
        "soybean_meal" => MatchRule {
            label_ru: "Соевый шрот",
            label_en: "Soybean Meal",
            name_fragments: &["соев", "soybean"],
            categories: &["oilseed_meal", "protein"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(350.0),
            nutrient_max: Some(550.0),
        },
        "corn_grain" | "corn" => MatchRule {
            label_ru: "Кукуруза зерно",
            label_en: "Corn Grain",
            name_fragments: &["кукуруз", "corn"],
            categories: &["grain"],
            nutrient_key: Some("energy_oe_pig"),
            nutrient_min: Some(12.0),
            nutrient_max: None,
        },
        "grass_silage" => MatchRule {
            label_ru: "Травяной силос или сенаж",
            label_en: "Grass Silage or Haylage",
            name_fragments: &["трав", "grass", "сенаж"],
            categories: &["silage"],
            nutrient_key: Some("dry_matter"),
            nutrient_min: Some(20.0),
            nutrient_max: Some(60.0),
        },
        "hay" | "grass_hay" => MatchRule {
            label_ru: "Сено",
            label_en: "Hay",
            name_fragments: &["сено", "hay"],
            categories: &["roughage"],
            nutrient_key: Some("dry_matter"),
            nutrient_min: Some(75.0),
            nutrient_max: None,
        },
        "barley" => MatchRule {
            label_ru: "Ячмень",
            label_en: "Barley",
            name_fragments: &["ячм", "barley"],
            categories: &["grain"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(90.0),
            nutrient_max: Some(150.0),
        },
        "sunflower_meal" => MatchRule {
            label_ru: "Подсолнечный шрот",
            label_en: "Sunflower Meal",
            name_fragments: &["подсолнеч", "sunflower"],
            categories: &["oilseed_meal", "protein"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(250.0),
            nutrient_max: Some(420.0),
        },
        "straw" => MatchRule {
            label_ru: "Солома",
            label_en: "Straw",
            name_fragments: &["солом", "straw"],
            categories: &["roughage"],
            nutrient_key: Some("crude_fiber"),
            nutrient_min: Some(250.0),
            nutrient_max: None,
        },
        "mineral_premix" => MatchRule {
            label_ru: "Минеральный премикс",
            label_en: "Mineral Premix",
            name_fragments: &["премикс", "минерал", "premix"],
            categories: &["premix", "mineral"],
            nutrient_key: Some("calcium"),
            nutrient_min: Some(50.0),
            nutrient_max: None,
        },
        "close_up_ration" => MatchRule {
            label_ru: "Рацион предродового периода",
            label_en: "Close-up Ration",
            name_fragments: &["close", "транзит", "предрод"],
            categories: &["compound_feed", "premix"],
            nutrient_key: None,
            nutrient_min: None,
            nutrient_max: None,
        },
        "anionic_salts" => MatchRule {
            label_ru: "Анионные соли",
            label_en: "Anionic Salts",
            name_fragments: &["анион", "anion", "хлорид", "сульфат"],
            categories: &["mineral", "additive"],
            nutrient_key: Some("sulfur"),
            nutrient_min: Some(1.0),
            nutrient_max: None,
        },
        "pasture" => MatchRule {
            label_ru: "Пастбищная трава",
            label_en: "Pasture",
            name_fragments: &["пастбищ", "pasture", "зелен"],
            categories: &["green_forage", "roughage"],
            nutrient_key: Some("dry_matter"),
            nutrient_min: Some(15.0),
            nutrient_max: Some(40.0),
        },
        "grain_supplement" => MatchRule {
            label_ru: "Зерновая добавка",
            label_en: "Grain Supplement",
            name_fragments: &["зерн", "grain", "ячм", "кукуруз", "пшениц"],
            categories: &["grain", "concentrate"],
            nutrient_key: Some("energy_oe_cattle"),
            nutrient_min: Some(11.0),
            nutrient_max: None,
        },
        "protein_supplement" | "high_lysine" => MatchRule {
            label_ru: "Белковая добавка",
            label_en: "Protein Supplement",
            name_fragments: &["шрот", "жмых", "lysine", "лизин", "соев", "протеин"],
            categories: &["oilseed_meal", "protein", "additive"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(250.0),
            nutrient_max: None,
        },
        "high_energy_grain" | "high_energy" => MatchRule {
            label_ru: "Высокоэнергетический концентрат",
            label_en: "High-Energy Concentrate",
            name_fragments: &["кукуруз", "corn", "ячм", "зерн", "масло", "жир"],
            categories: &["grain", "concentrate", "oil_fat"],
            nutrient_key: Some("energy_oe_pig"),
            nutrient_min: Some(12.0),
            nutrient_max: None,
        },
        "fat_supplement" => MatchRule {
            label_ru: "Жировая добавка",
            label_en: "Fat Supplement",
            name_fragments: &["жир", "масло", "fat", "oil"],
            categories: &["oil_fat", "additive"],
            nutrient_key: Some("crude_fat"),
            nutrient_min: Some(100.0),
            nutrient_max: None,
        },
        "maintenance_ration" | "holding_ration" => MatchRule {
            label_ru: "Поддерживающий рацион",
            label_en: "Maintenance Ration",
            name_fragments: &["сено", "солом", "maintenance", "holding"],
            categories: &["roughage", "silage", "compound_feed"],
            nutrient_key: Some("dry_matter"),
            nutrient_min: Some(25.0),
            nutrient_max: None,
        },
        "controlled_energy" => MatchRule {
            label_ru: "Рацион с контролируемой энергией",
            label_en: "Controlled-Energy Ration",
            name_fragments: &["солом", "сено", "fiber", "контрол"],
            categories: &["roughage", "compound_feed"],
            nutrient_key: Some("crude_fiber"),
            nutrient_min: Some(120.0),
            nutrient_max: None,
        },
        "prestarter_complete" => MatchRule {
            label_ru: "Престартерный комбикорм",
            label_en: "Prestarter Complete Feed",
            name_fragments: &["престарт", "prestarter", "prestart"],
            categories: &["compound_feed"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(180.0),
            nutrient_max: None,
        },
        "milk_replacer" => MatchRule {
            label_ru: "Заменитель молока",
            label_en: "Milk Replacer",
            name_fragments: &["зцм", "заменитель молока", "milk replacer"],
            categories: &["compound_feed", "additive"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(180.0),
            nutrient_max: None,
        },
        "starter_complete" => MatchRule {
            label_ru: "Стартерный комбикорм",
            label_en: "Starter Complete Feed",
            name_fragments: &["стартер", "starter"],
            categories: &["compound_feed"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(150.0),
            nutrient_max: None,
        },
        "lysine" => MatchRule {
            label_ru: "Лизиновая добавка",
            label_en: "Lysine Additive",
            name_fragments: &["лизин", "lysine"],
            categories: &["additive", "premix"],
            nutrient_key: Some("lysine"),
            nutrient_min: Some(50.0),
            nutrient_max: None,
        },
        "wheat" => MatchRule {
            label_ru: "Пшеница",
            label_en: "Wheat",
            name_fragments: &["пшениц", "wheat"],
            categories: &["grain"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(90.0),
            nutrient_max: Some(180.0),
        },
        "reduced_protein" => MatchRule {
            label_ru: "Рацион с пониженным протеином",
            label_en: "Reduced-Protein Feed",
            name_fragments: &["ячм", "пшениц", "reduced protein", "низкобел"],
            categories: &["grain", "compound_feed"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: None,
            nutrient_max: Some(170.0),
        },
        "gilt_developer" => MatchRule {
            label_ru: "Комбикорм для ремонтных свинок",
            label_en: "Gilt Developer Feed",
            name_fragments: &["ремонт", "gilt", "developer"],
            categories: &["compound_feed"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(140.0),
            nutrient_max: None,
        },
        "gestation_ration" => MatchRule {
            label_ru: "Комбикорм для супоросных свиноматок",
            label_en: "Gestation Ration",
            name_fragments: &["супорос", "gestation"],
            categories: &["compound_feed"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(120.0),
            nutrient_max: None,
        },
        "fiber_source" => MatchRule {
            label_ru: "Источник клетчатки",
            label_en: "Fiber Source",
            name_fragments: &["отруб", "жом", "свекл", "fiber"],
            categories: &["byproduct", "roughage"],
            nutrient_key: Some("crude_fiber"),
            nutrient_min: Some(80.0),
            nutrient_max: None,
        },
        "broiler_starter_crumble" => MatchRule {
            label_ru: "Стартер для бройлеров",
            label_en: "Broiler Starter",
            name_fragments: &["бройлер", "стартер", "starter"],
            categories: &["compound_feed"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(180.0),
            nutrient_max: None,
        },
        "broiler_grower_pellet" => MatchRule {
            label_ru: "Ростовой корм для бройлеров",
            label_en: "Broiler Grower",
            name_fragments: &["бройлер", "grower", "рост"],
            categories: &["compound_feed"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(160.0),
            nutrient_max: None,
        },
        "broiler_finisher_pellet" => MatchRule {
            label_ru: "Финишер для бройлеров",
            label_en: "Broiler Finisher",
            name_fragments: &["бройлер", "финиш", "finisher"],
            categories: &["compound_feed"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(150.0),
            nutrient_max: None,
        },
        "pullet_developer" => MatchRule {
            label_ru: "Корм для молодок",
            label_en: "Pullet Developer",
            name_fragments: &["молод", "pullet", "developer"],
            categories: &["compound_feed"],
            nutrient_key: Some("crude_protein"),
            nutrient_min: Some(130.0),
            nutrient_max: None,
        },
        "prelay_ration" => MatchRule {
            label_ru: "Предкладковый корм",
            label_en: "Pre-lay Ration",
            name_fragments: &["предклад", "prelay", "pre-lay"],
            categories: &["compound_feed"],
            nutrient_key: Some("calcium"),
            nutrient_min: Some(10.0),
            nutrient_max: None,
        },
        "calcium_buildup" | "oyster_shell" => MatchRule {
            label_ru: "Кальциевая добавка",
            label_en: "Calcium Source",
            name_fragments: &["мел", "ракуш", "извест", "calcium", "oyster"],
            categories: &["mineral"],
            nutrient_key: Some("calcium"),
            nutrient_min: Some(100.0),
            nutrient_max: None,
        },
        "layer_peak" => MatchRule {
            label_ru: "Корм для несушек пик",
            label_en: "Layer Peak Feed",
            name_fragments: &["несуш", "layer", "peak"],
            categories: &["compound_feed"],
            nutrient_key: Some("calcium"),
            nutrient_min: Some(25.0),
            nutrient_max: None,
        },
        "layer_phase2" | "layer_post_peak" => MatchRule {
            label_ru: "Корм для несушек, вторая фаза",
            label_en: "Layer Phase 2 Feed",
            name_fragments: &["несуш", "layer", "phase 2", "фаза 2"],
            categories: &["compound_feed"],
            nutrient_key: Some("calcium"),
            nutrient_min: Some(20.0),
            nutrient_max: None,
        },
        "reduced_calcium" => MatchRule {
            label_ru: "Рацион со сниженным кальцием",
            label_en: "Reduced-Calcium Feed",
            name_fragments: &["несуш", "layer", "reduced calcium", "сниж"],
            categories: &["compound_feed"],
            nutrient_key: Some("calcium"),
            nutrient_min: None,
            nutrient_max: Some(30.0),
        },
        _ => MatchRule {
            label_ru: "Рекомендованный корм",
            label_en: "Recommended Feed",
            name_fragments: &[],
            categories: &[],
            nutrient_key: None,
            nutrient_min: None,
            nutrient_max: None,
        },
    }
}

pub fn recommendation_label(recommendation_key: &str) -> (&'static str, &'static str) {
    let rule = rule_for(recommendation_key);
    (rule.label_ru, rule.label_en)
}

fn nutrient_value(feed: &Feed, key: &str) -> Option<f64> {
    match key {
        "dry_matter" => feed.dry_matter,
        "crude_protein" => feed.crude_protein,
        "crude_fiber" => feed.crude_fiber,
        "crude_fat" => feed.crude_fat,
        "calcium" => feed.calcium,
        "energy_oe_cattle" => feed.energy_oe_cattle,
        "energy_oe_pig" => feed.energy_oe_pig,
        "lysine" => feed.lysine,
        _ => None,
    }
}

fn calculate_match_score(feed: &Feed, rule: &MatchRule) -> Option<(f64, MatchReasonParts)> {
    let haystack = format!(
        "{} {} {}",
        feed.name_ru.to_lowercase(),
        feed.name_en.clone().unwrap_or_default().to_lowercase(),
        feed.subcategory.clone().unwrap_or_default().to_lowercase()
    );

    let mut score: f64 = 0.0;
    let mut parts = MatchReasonParts::default();

    if let Some(fragment) = rule
        .name_fragments
        .iter()
        .find(|fragment| haystack.contains(&fragment.to_lowercase()))
    {
        score += 0.68;
        parts.name_fragment = Some(fragment.to_string());
    }

    if rule.categories.iter().any(|category| *category == feed.category) {
        score += 0.22;
        parts.category = Some(feed.category.clone());
    }

    if let Some(nutrient_key) = rule.nutrient_key {
        if let Some(value) = nutrient_value(feed, nutrient_key) {
            let min_ok = rule.nutrient_min.map(|min| value >= min).unwrap_or(true);
            let max_ok = rule.nutrient_max.map(|max| value <= max).unwrap_or(true);
            if min_ok && max_ok {
                score += 0.15;
                parts.nutrient_key = Some(nutrient_key.to_string());
                parts.nutrient_value = Some(value);
            }
        }
    }

    if feed.verified {
        score += 0.05;
        parts.verified = true;
    }

    if score >= 0.5 {
        Some((score.min(1.0), parts))
    } else {
        None
    }
}

pub fn match_preset_feeds(
    recommendation_key: &str,
    available_feeds: &[Feed],
    limit: usize,
) -> Vec<FeedMatch> {
    let rule = rule_for(recommendation_key);

    let mut matches = available_feeds
        .iter()
        .filter_map(|feed| {
            calculate_match_score(feed, &rule).map(|(match_score, parts)| FeedMatch {
                feed: feed.clone(),
                match_score,
                match_reason: parts,
            })
        })
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        right
            .match_score
            .partial_cmp(&left.match_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.feed.verified.cmp(&left.feed.verified))
            .then_with(|| left.feed.name_ru.cmp(&right.feed.name_ru))
    });
    matches.truncate(limit);
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(name_ru: &str, category: &str) -> Feed {
        Feed {
            id: Some(1),
            name_ru: name_ru.to_string(),
            category: category.to_string(),
            dry_matter: Some(35.0),
            crude_protein: Some(420.0),
            crude_fiber: Some(180.0),
            calcium: Some(35.0),
            lysine: Some(60.0),
            verified: true,
            ..Default::default()
        }
    }

    #[test]
    fn corn_silage_rule_prefers_named_silage() {
        let feeds = vec![
            feed("Силос кукурузный", "silage"),
            feed("Ячмень фуражный", "grain"),
        ];

        let matches = match_preset_feeds("corn_silage", &feeds, 5);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].feed.name_ru, "Силос кукурузный");
        assert!(matches[0].match_score >= 0.8);
    }

    #[test]
    fn lysine_rule_matches_additives() {
        let feeds = vec![
            feed("Лизин кормовой", "additive"),
            feed("Шрот подсолнечный", "oilseed_meal"),
        ];

        let matches = match_preset_feeds("lysine", &feeds, 5);
        assert_eq!(matches[0].feed.name_ru, "Лизин кормовой");
    }
}
