use serde::{Deserialize, Serialize};

use crate::db::feeds::Feed;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedSourceKind {
    Normalized,
    Curated,
    Custom,
    Imported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedTranslationStatus {
    Ready,
    SourceOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedProfileStatus {
    Complete,
    Partial,
    Limited,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedProfileSectionKey {
    Energy,
    Protein,
    Fiber,
    Minerals,
    Vitamins,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedProfileSectionStatus {
    Present,
    Partial,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedProfileSectionAudit {
    pub key: FeedProfileSectionKey,
    pub status: FeedProfileSectionStatus,
    pub present: usize,
    pub expected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedQualityAudit {
    pub source_kind: FeedSourceKind,
    pub translation_status: FeedTranslationStatus,
    pub profile_status: FeedProfileStatus,
    pub profile_sections: Vec<FeedProfileSectionAudit>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FeedCriticalNutrientKey {
    DryMatter,
    EnergyOeCattle,
    EnergyOePig,
    EnergyOePoultry,
    CrudeProtein,
    DigProteinCattle,
    Calcium,
    Phosphorus,
    Lysine,
    MethionineCystine,
    VitD3,
    VitE,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedCriticalCoverageAudit {
    pub species: String,
    pub stage_context: Option<String>,
    pub coverage_status: FeedProfileStatus,
    pub present_required: usize,
    pub required_total: usize,
    pub required_keys: Vec<FeedCriticalNutrientKey>,
    pub missing_keys: Vec<FeedCriticalNutrientKey>,
}

#[derive(Debug, Clone, Copy)]
struct SectionRequirements {
    energy: usize,
    protein: usize,
    fiber: usize,
    minerals: usize,
    vitamins: usize,
}

fn normalize_text(value: Option<&str>) -> String {
    value
        .unwrap_or_default()
        .trim()
        .to_lowercase()
        .replace('ё', "е")
}

fn has_value(value: Option<f64>) -> bool {
    value.is_some_and(|number| number.is_finite() && number.abs() > 1e-9)
}

fn feed_has_critical_key(feed: &Feed, key: FeedCriticalNutrientKey) -> bool {
    match key {
        FeedCriticalNutrientKey::DryMatter => has_value(feed.dry_matter),
        FeedCriticalNutrientKey::EnergyOeCattle => has_value(feed.energy_oe_cattle),
        FeedCriticalNutrientKey::EnergyOePig => has_value(feed.energy_oe_pig),
        FeedCriticalNutrientKey::EnergyOePoultry => has_value(feed.energy_oe_poultry),
        FeedCriticalNutrientKey::CrudeProtein => has_value(feed.crude_protein),
        FeedCriticalNutrientKey::DigProteinCattle => has_value(feed.dig_protein_cattle),
        FeedCriticalNutrientKey::Calcium => has_value(feed.calcium),
        FeedCriticalNutrientKey::Phosphorus => has_value(feed.phosphorus),
        FeedCriticalNutrientKey::Lysine => has_value(feed.lysine),
        FeedCriticalNutrientKey::MethionineCystine => has_value(feed.methionine_cystine),
        FeedCriticalNutrientKey::VitD3 => has_value(feed.vit_d3),
        FeedCriticalNutrientKey::VitE => has_value(feed.vit_e),
    }
}

fn count_present<const N: usize>(values: [Option<f64>; N]) -> usize {
    values.into_iter().filter(|value| has_value(*value)).count()
}

fn normalized_feed_text(feed: &Feed) -> String {
    let combined = format!(
        "{} {} {} {} {}",
        feed.name_ru,
        feed.name_en.as_deref().unwrap_or_default(),
        feed.category,
        feed.subcategory.as_deref().unwrap_or_default(),
        feed.source_subcategory_en.as_deref().unwrap_or_default(),
    );
    normalize_text(Some(combined.as_str()))
}

fn matches_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

fn push_required_key(keys: &mut Vec<FeedCriticalNutrientKey>, key: FeedCriticalNutrientKey) {
    if !keys.contains(&key) {
        keys.push(key);
    }
}

fn is_cattle_primary_category(category: &str) -> bool {
    matches!(
        category,
        "grain"
            | "concentrate"
            | "oilseed_meal"
            | "protein"
            | "roughage"
            | "silage"
            | "succulent"
            | "green_forage"
            | "byproduct"
            | "compound_feed"
            | "animal_origin"
    )
}

fn is_monogastric_primary_category(category: &str) -> bool {
    matches!(
        category,
        "grain"
            | "concentrate"
            | "oilseed_meal"
            | "protein"
            | "byproduct"
            | "compound_feed"
            | "animal_origin"
    )
}

fn is_salt_like(feed_text: &str) -> bool {
    matches_any(
        feed_text,
        &[
            "соль",
            "поваренн",
            "salt",
            "sodium chloride",
            "chloride sodium",
        ],
    )
}

fn is_shell_like(feed_text: &str) -> bool {
    matches_any(
        feed_text,
        &["ракуш", "shell grit", "shell", "limestone", "известняк"],
    )
}

fn is_calcium_or_phosphorus_source(feed_text: &str) -> bool {
    matches_any(
        feed_text,
        &[
            "мел",
            "chalk",
            "calcium carbonate",
            "фосфат",
            "phosphate",
            "дикальц",
            "dicalcium",
            "монокальц",
            "monocalcium",
            "трикальц",
            "tricalcium",
            "bone meal",
            "костн",
            "ракуш",
            "shell",
        ],
    )
}

fn is_trace_premix_like(category: &str, feed_text: &str) -> bool {
    matches!(category, "premix" | "additive")
        || matches_any(feed_text, &["премикс", "premix", "бав", "trace", "витамин"])
}

fn required_critical_keys(feed: &Feed, species: &str) -> Option<Vec<FeedCriticalNutrientKey>> {
    let category = feed.category.trim();
    let feed_text = normalized_feed_text(feed);
    let mut keys = Vec::new();

    if is_trace_premix_like(category, &feed_text) {
        push_required_key(&mut keys, FeedCriticalNutrientKey::VitD3);
        push_required_key(&mut keys, FeedCriticalNutrientKey::VitE);
        return Some(keys);
    }

    if category == "mineral" {
        if is_salt_like(&feed_text) {
            return None;
        }

        if is_shell_like(&feed_text) {
            push_required_key(&mut keys, FeedCriticalNutrientKey::Calcium);
            return Some(keys);
        }

        if is_calcium_or_phosphorus_source(&feed_text) {
            push_required_key(&mut keys, FeedCriticalNutrientKey::Calcium);
            push_required_key(&mut keys, FeedCriticalNutrientKey::Phosphorus);
            return Some(keys);
        }

        return None;
    }

    match species {
        "cattle" if is_cattle_primary_category(category) => {
            push_required_key(&mut keys, FeedCriticalNutrientKey::DryMatter);
            push_required_key(&mut keys, FeedCriticalNutrientKey::EnergyOeCattle);
            push_required_key(&mut keys, FeedCriticalNutrientKey::CrudeProtein);
            push_required_key(&mut keys, FeedCriticalNutrientKey::DigProteinCattle);
            push_required_key(&mut keys, FeedCriticalNutrientKey::Calcium);
            push_required_key(&mut keys, FeedCriticalNutrientKey::Phosphorus);
        }
        "swine" | "poultry" if is_monogastric_primary_category(category) => {
            push_required_key(&mut keys, FeedCriticalNutrientKey::DryMatter);
            push_required_key(
                &mut keys,
                if species == "swine" {
                    FeedCriticalNutrientKey::EnergyOePig
                } else {
                    FeedCriticalNutrientKey::EnergyOePoultry
                },
            );
            push_required_key(&mut keys, FeedCriticalNutrientKey::CrudeProtein);
            push_required_key(&mut keys, FeedCriticalNutrientKey::Lysine);
            push_required_key(&mut keys, FeedCriticalNutrientKey::MethionineCystine);
            push_required_key(&mut keys, FeedCriticalNutrientKey::Calcium);
            push_required_key(&mut keys, FeedCriticalNutrientKey::Phosphorus);
        }
        _ => {}
    }

    if keys.is_empty() {
        None
    } else {
        Some(keys)
    }
}

fn section_audit(
    key: FeedProfileSectionKey,
    present: usize,
    expected: usize,
) -> Option<FeedProfileSectionAudit> {
    if expected == 0 && present == 0 {
        return None;
    }

    let status = if present >= expected && present > 0 {
        FeedProfileSectionStatus::Present
    } else if present > 0 {
        FeedProfileSectionStatus::Partial
    } else {
        FeedProfileSectionStatus::Missing
    };

    Some(FeedProfileSectionAudit {
        key,
        status,
        present,
        expected,
    })
}

fn section_requirements(category: &str) -> SectionRequirements {
    match category.trim() {
        "grain" | "concentrate" => SectionRequirements {
            energy: 3,
            protein: 2,
            fiber: 1,
            minerals: 2,
            vitamins: 0,
        },
        "oilseed_meal" | "protein" | "animal_origin" => SectionRequirements {
            energy: 2,
            protein: 3,
            fiber: 1,
            minerals: 2,
            vitamins: 0,
        },
        "roughage" | "silage" => SectionRequirements {
            energy: 2,
            protein: 1,
            fiber: 2,
            minerals: 2,
            vitamins: 0,
        },
        "succulent" => SectionRequirements {
            energy: 2,
            protein: 1,
            fiber: 1,
            minerals: 2,
            vitamins: 0,
        },
        "mineral" => SectionRequirements {
            energy: 0,
            protein: 0,
            fiber: 0,
            minerals: 2,
            vitamins: 0,
        },
        "premix" => SectionRequirements {
            energy: 0,
            protein: 0,
            fiber: 0,
            minerals: 1,
            vitamins: 1,
        },
        "additive" => SectionRequirements {
            energy: 1,
            protein: 0,
            fiber: 0,
            minerals: 0,
            vitamins: 0,
        },
        _ => SectionRequirements {
            energy: 1,
            protein: 1,
            fiber: 0,
            minerals: 1,
            vitamins: 0,
        },
    }
}

pub fn has_translated_feed_name(feed: &Feed) -> bool {
    let ru = normalize_text(Some(feed.name_ru.as_str()));
    let en = normalize_text(feed.name_en.as_deref());
    !en.is_empty() && en != ru
}

pub fn audit_feed(feed: &Feed) -> FeedQualityAudit {
    let requirements = section_requirements(&feed.category);
    let energy = count_present([
        feed.dry_matter,
        feed.koe,
        feed.energy_oe_cattle,
        feed.energy_oe_pig,
        feed.energy_oe_poultry,
    ]);
    let protein = count_present([
        feed.crude_protein,
        feed.dig_protein_cattle,
        feed.dig_protein_pig,
        feed.dig_protein_poultry,
        feed.lysine,
        feed.methionine_cystine,
    ]);
    let fiber = count_present([
        feed.crude_fiber,
        feed.starch,
        feed.sugar,
    ]);
    let minerals = count_present([
        feed.calcium,
        feed.phosphorus,
        feed.magnesium,
        feed.potassium,
        feed.sodium,
        feed.sulfur,
        feed.iron,
        feed.copper,
        feed.zinc,
        feed.manganese,
        feed.cobalt,
        feed.iodine,
    ]);
    let vitamins = count_present([
        feed.carotene,
        feed.vit_d3,
        feed.vit_e,
    ]);

    let profile_sections = [
        section_audit(FeedProfileSectionKey::Energy, energy, requirements.energy),
        section_audit(
            FeedProfileSectionKey::Protein,
            protein,
            requirements.protein,
        ),
        section_audit(FeedProfileSectionKey::Fiber, fiber, requirements.fiber),
        section_audit(
            FeedProfileSectionKey::Minerals,
            minerals,
            requirements.minerals,
        ),
        section_audit(
            FeedProfileSectionKey::Vitamins,
            vitamins,
            requirements.vitamins,
        ),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let required_sections = profile_sections
        .iter()
        .filter(|section| section.expected > 0)
        .collect::<Vec<_>>();
    let present_required = required_sections
        .iter()
        .filter(|section| section.status == FeedProfileSectionStatus::Present)
        .count();
    let missing_required = required_sections
        .iter()
        .filter(|section| section.status == FeedProfileSectionStatus::Missing)
        .count();
    let profile_status = if required_sections.is_empty() {
        if profile_sections
            .iter()
            .any(|section| section.status != FeedProfileSectionStatus::Missing)
        {
            FeedProfileStatus::Partial
        } else {
            FeedProfileStatus::Limited
        }
    } else if missing_required == 0 {
        FeedProfileStatus::Complete
    } else if present_required >= required_sections.len().saturating_sub(1).max(1) {
        FeedProfileStatus::Partial
    } else {
        FeedProfileStatus::Limited
    };

    let source_kind = if feed.is_custom {
        FeedSourceKind::Custom
    } else if feed
        .source_id
        .as_deref()
        .is_some_and(|source| source.starts_with("seed:normalized-db:"))
    {
        FeedSourceKind::Normalized
    } else if feed
        .source_id
        .as_deref()
        .is_some_and(|source| source.starts_with("seed:"))
    {
        FeedSourceKind::Curated
    } else {
        FeedSourceKind::Imported
    };

    FeedQualityAudit {
        source_kind,
        translation_status: if has_translated_feed_name(feed) {
            FeedTranslationStatus::Ready
        } else {
            FeedTranslationStatus::SourceOnly
        },
        profile_status,
        profile_sections,
    }
}

pub fn audit_feed_critical_coverage(
    feed: &Feed,
    species: &str,
    stage_context: Option<&str>,
) -> Option<FeedCriticalCoverageAudit> {
    let normalized_species = normalize_text(Some(species));
    if normalized_species.is_empty() {
        return None;
    }

    let required_keys = required_critical_keys(feed, normalized_species.as_str())?;
    let required_total = required_keys.len();
    if required_total == 0 {
        return None;
    }

    let present_required = required_keys
        .iter()
        .filter(|key| feed_has_critical_key(feed, **key))
        .count();
    let missing_keys = required_keys
        .iter()
        .copied()
        .filter(|key| !feed_has_critical_key(feed, *key))
        .collect::<Vec<_>>();
    let coverage_status = if missing_keys.is_empty() {
        FeedProfileStatus::Complete
    } else if present_required >= required_total.saturating_sub(1).max(1) {
        FeedProfileStatus::Partial
    } else {
        FeedProfileStatus::Limited
    };

    Some(FeedCriticalCoverageAudit {
        species: normalized_species,
        stage_context: stage_context.map(|value| value.to_string()),
        coverage_status,
        present_required,
        required_total,
        required_keys,
        missing_keys,
    })
}

pub fn enrich_feed(mut feed: Feed) -> Feed {
    let audit = audit_feed(&feed);
    feed.source_kind = Some(audit.source_kind);
    feed.translation_status = Some(audit.translation_status);
    feed.profile_status = Some(audit.profile_status);
    feed.profile_sections = Some(audit.profile_sections);
    feed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_normalized_source_and_ready_translation() {
        let feed = Feed {
            source_id: Some("seed:normalized-db:n42".to_string()),
            name_ru: "Подсолнечный шрот".to_string(),
            name_en: Some("Sunflower meal".to_string()),
            category: "oilseed_meal".to_string(),
            crude_protein: Some(345.0),
            dig_protein_cattle: Some(260.0),
            lysine: Some(8.1),
            dry_matter: Some(89.0),
            energy_oe_cattle: Some(11.8),
            calcium: Some(3.5),
            phosphorus: Some(8.1),
            ..Default::default()
        };

        let audit = audit_feed(&feed);
        assert_eq!(audit.source_kind, FeedSourceKind::Normalized);
        assert_eq!(audit.translation_status, FeedTranslationStatus::Ready);
        assert_eq!(audit.profile_status, FeedProfileStatus::Partial);
    }

    #[test]
    fn grain_profile_becomes_partial_when_structural_block_is_sparse() {
        let feed = Feed {
            source_id: Some("seed:wheat".to_string()),
            name_ru: "Пшеница".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            koe: Some(1.18),
            energy_oe_cattle: Some(13.2),
            crude_protein: Some(125.0),
            dig_protein_cattle: Some(92.0),
            calcium: Some(0.8),
            phosphorus: Some(3.5),
            ..Default::default()
        };

        let audit = audit_feed(&feed);
        assert_eq!(audit.profile_status, FeedProfileStatus::Partial);
        assert!(audit.profile_sections.iter().any(|section| {
            section.key == FeedProfileSectionKey::Fiber
                && section.status == FeedProfileSectionStatus::Missing
        }));
    }

    #[test]
    fn premix_requires_vitamin_and_mineral_sections() {
        let feed = Feed {
            source_id: Some("seed:premix".to_string()),
            name_ru: "Премикс 1%".to_string(),
            category: "premix".to_string(),
            calcium: Some(120.0),
            ..Default::default()
        };

        let audit = audit_feed(&feed);
        assert_eq!(audit.profile_status, FeedProfileStatus::Partial);
        assert!(audit.profile_sections.iter().any(|section| {
            section.key == FeedProfileSectionKey::Minerals
                && section.status == FeedProfileSectionStatus::Present
        }));
        assert!(audit.profile_sections.iter().any(|section| {
            section.key == FeedProfileSectionKey::Vitamins
                && section.status == FeedProfileSectionStatus::Missing
        }));
    }

    #[test]
    fn enrich_feed_populates_computed_fields() {
        let feed = Feed {
            source_id: Some("https://example.test/feed".to_string()),
            name_ru: "Imported sample".to_string(),
            category: "other".to_string(),
            crude_protein: Some(160.0),
            ..Default::default()
        };

        let enriched = enrich_feed(feed);
        assert_eq!(enriched.source_kind, Some(FeedSourceKind::Imported));
        assert_eq!(
            enriched.translation_status,
            Some(FeedTranslationStatus::SourceOnly)
        );
        assert_eq!(enriched.profile_status, Some(FeedProfileStatus::Partial));
        assert!(enriched.profile_sections.is_some());
    }

    #[test]
    fn cattle_structural_audit_highlights_missing_digestible_protein() {
        let feed = Feed {
            name_ru: "Силос кукурузный".to_string(),
            category: "silage".to_string(),
            dry_matter: Some(34.0),
            energy_oe_cattle: Some(10.3),
            crude_protein: Some(82.0),
            calcium: Some(2.1),
            phosphorus: Some(2.2),
            ..Default::default()
        };

        let audit = audit_feed_critical_coverage(&feed, "cattle", Some("cattle_dairy_early_lact"))
            .expect("cattle audit");
        assert_eq!(audit.coverage_status, FeedProfileStatus::Partial);
        assert_eq!(audit.required_total, 6);
        assert_eq!(audit.present_required, 5);
        assert_eq!(audit.missing_keys, vec![FeedCriticalNutrientKey::DigProteinCattle]);
    }

    #[test]
    fn premix_audit_requires_supported_vitamin_payload() {
        let feed = Feed {
            name_ru: "Премикс 1%".to_string(),
            category: "premix".to_string(),
            calcium: Some(120.0),
            ..Default::default()
        };

        let audit = audit_feed_critical_coverage(&feed, "cattle", Some("cattle_dairy_fresh"))
            .expect("premix audit");
        assert_eq!(audit.coverage_status, FeedProfileStatus::Limited);
        assert_eq!(
            audit.required_keys,
            vec![
                FeedCriticalNutrientKey::VitD3,
                FeedCriticalNutrientKey::VitE
            ]
        );
        assert_eq!(
            audit.missing_keys,
            vec![
                FeedCriticalNutrientKey::VitD3,
                FeedCriticalNutrientKey::VitE
            ]
        );
    }

    #[test]
    fn salt_like_mineral_is_not_scored_for_critical_coverage() {
        let feed = Feed {
            name_ru: "Соль поваренная".to_string(),
            category: "mineral".to_string(),
            sodium: Some(390.0),
            ..Default::default()
        };

        assert!(
            audit_feed_critical_coverage(&feed, "cattle", Some("cattle_dairy_early_lact"))
                .is_none()
        );
    }

    #[test]
    fn swine_primary_audit_requires_supported_phosphorus_and_amino_acids() {
        let feed = Feed {
            name_ru: "Пшеница фуражная".to_string(),
            category: "grain".to_string(),
            dry_matter: Some(86.0),
            energy_oe_pig: Some(13.8),
            crude_protein: Some(124.0),
            lysine: Some(3.5),
            calcium: Some(0.7),
            ..Default::default()
        };

        let audit = audit_feed_critical_coverage(&feed, "swine", Some("swine_finisher"))
            .expect("swine audit");
        assert_eq!(audit.coverage_status, FeedProfileStatus::Limited);
        assert!(audit
            .missing_keys
            .contains(&FeedCriticalNutrientKey::MethionineCystine));
        assert!(audit
            .missing_keys
            .contains(&FeedCriticalNutrientKey::Phosphorus));
    }
}
