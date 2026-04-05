//! Canonical nutrient definitions for the implemented Felex feed authority.

use super::categories::NutrientCategory;
use super::units::Unit;

#[derive(Debug, Clone, PartialEq)]
pub struct NutrientDef {
    pub key: &'static str,
    pub db_column: &'static str,
    pub storage_unit: Unit,
    pub category: NutrientCategory,
    pub name_en: &'static str,
    pub name_ru: &'static str,
}

pub static NUTRIENTS: &[NutrientDef] = &[
    NutrientDef {
        key: "dry_matter",
        db_column: "dry_matter",
        storage_unit: Unit::Percent,
        category: NutrientCategory::General,
        name_en: "Dry Matter",
        name_ru: "Сухое вещество",
    },
    NutrientDef {
        key: "energy_oe_cattle",
        db_column: "energy_oe_cattle",
        storage_unit: Unit::MjPerKg,
        category: NutrientCategory::Energy,
        name_en: "Metabolizable Energy (Cattle)",
        name_ru: "Обменная энергия (КРС)",
    },
    NutrientDef {
        key: "energy_oe_pig",
        db_column: "energy_oe_pig",
        storage_unit: Unit::MjPerKg,
        category: NutrientCategory::Energy,
        name_en: "Metabolizable Energy (Swine)",
        name_ru: "Обменная энергия (свиньи)",
    },
    NutrientDef {
        key: "energy_oe_poultry",
        db_column: "energy_oe_poultry",
        storage_unit: Unit::MjPerKg,
        category: NutrientCategory::Energy,
        name_en: "Metabolizable Energy (Poultry)",
        name_ru: "Обменная энергия (птица)",
    },
    NutrientDef {
        key: "crude_protein",
        db_column: "crude_protein",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Protein,
        name_en: "Crude Protein",
        name_ru: "Сырой протеин",
    },
    NutrientDef {
        key: "dig_protein_cattle",
        db_column: "dig_protein_cattle",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Protein,
        name_en: "Digestible Protein (Cattle)",
        name_ru: "Переваримый протеин (КРС)",
    },
    NutrientDef {
        key: "dig_protein_pig",
        db_column: "dig_protein_pig",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Protein,
        name_en: "Digestible Protein (Swine)",
        name_ru: "Переваримый протеин (свиньи)",
    },
    NutrientDef {
        key: "dig_protein_poultry",
        db_column: "dig_protein_poultry",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Protein,
        name_en: "Digestible Protein (Poultry)",
        name_ru: "Переваримый протеин (птица)",
    },
    NutrientDef {
        key: "lysine",
        db_column: "lysine",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::AminoAcids,
        name_en: "Lysine",
        name_ru: "Лизин",
    },
    NutrientDef {
        key: "methionine_cystine",
        db_column: "methionine_cystine",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::AminoAcids,
        name_en: "Methionine + Cystine",
        name_ru: "Метионин + цистин",
    },
    NutrientDef {
        key: "crude_fat",
        db_column: "crude_fat",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Fats,
        name_en: "Crude Fat",
        name_ru: "Сырой жир",
    },
    NutrientDef {
        key: "crude_fiber",
        db_column: "crude_fiber",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::FiberCarbs,
        name_en: "Crude Fiber",
        name_ru: "Сырая клетчатка",
    },
    NutrientDef {
        key: "starch",
        db_column: "starch",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::FiberCarbs,
        name_en: "Starch",
        name_ru: "Крахмал",
    },
    NutrientDef {
        key: "sugar",
        db_column: "sugar",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::FiberCarbs,
        name_en: "Sugars",
        name_ru: "Сахара",
    },
    NutrientDef {
        key: "calcium",
        db_column: "calcium",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Macrominerals,
        name_en: "Calcium",
        name_ru: "Кальций",
    },
    NutrientDef {
        key: "phosphorus",
        db_column: "phosphorus",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Macrominerals,
        name_en: "Phosphorus",
        name_ru: "Фосфор",
    },
    NutrientDef {
        key: "magnesium",
        db_column: "magnesium",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Macrominerals,
        name_en: "Magnesium",
        name_ru: "Магний",
    },
    NutrientDef {
        key: "potassium",
        db_column: "potassium",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Macrominerals,
        name_en: "Potassium",
        name_ru: "Калий",
    },
    NutrientDef {
        key: "sodium",
        db_column: "sodium",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Macrominerals,
        name_en: "Sodium",
        name_ru: "Натрий",
    },
    NutrientDef {
        key: "sulfur",
        db_column: "sulfur",
        storage_unit: Unit::GramPerKg,
        category: NutrientCategory::Macrominerals,
        name_en: "Sulfur",
        name_ru: "Сера",
    },
    NutrientDef {
        key: "iron",
        db_column: "iron",
        storage_unit: Unit::MgPerKg,
        category: NutrientCategory::TraceMinerals,
        name_en: "Iron",
        name_ru: "Железо",
    },
    NutrientDef {
        key: "copper",
        db_column: "copper",
        storage_unit: Unit::MgPerKg,
        category: NutrientCategory::TraceMinerals,
        name_en: "Copper",
        name_ru: "Медь",
    },
    NutrientDef {
        key: "zinc",
        db_column: "zinc",
        storage_unit: Unit::MgPerKg,
        category: NutrientCategory::TraceMinerals,
        name_en: "Zinc",
        name_ru: "Цинк",
    },
    NutrientDef {
        key: "manganese",
        db_column: "manganese",
        storage_unit: Unit::MgPerKg,
        category: NutrientCategory::TraceMinerals,
        name_en: "Manganese",
        name_ru: "Марганец",
    },
    NutrientDef {
        key: "cobalt",
        db_column: "cobalt",
        storage_unit: Unit::MgPerKg,
        category: NutrientCategory::TraceMinerals,
        name_en: "Cobalt",
        name_ru: "Кобальт",
    },
    NutrientDef {
        key: "iodine",
        db_column: "iodine",
        storage_unit: Unit::MgPerKg,
        category: NutrientCategory::TraceMinerals,
        name_en: "Iodine",
        name_ru: "Йод",
    },
    NutrientDef {
        key: "carotene",
        db_column: "carotene",
        storage_unit: Unit::MgPerKg,
        category: NutrientCategory::Vitamins,
        name_en: "Carotene",
        name_ru: "Каротин",
    },
    NutrientDef {
        key: "vitamin_d3",
        db_column: "vit_d3",
        storage_unit: Unit::IuPerKg,
        category: NutrientCategory::Vitamins,
        name_en: "Vitamin D3",
        name_ru: "Витамин D3",
    },
    NutrientDef {
        key: "vitamin_e",
        db_column: "vit_e",
        storage_unit: Unit::MgPerKg,
        category: NutrientCategory::Vitamins,
        name_en: "Vitamin E",
        name_ru: "Витамин E",
    },
    NutrientDef {
        key: "selenium",
        db_column: "selenium",
        storage_unit: Unit::MgPerKg,
        category: NutrientCategory::TraceMinerals,
        name_en: "Selenium",
        name_ru: "Селен",
    },
    NutrientDef {
        key: "ca_p_ratio",
        db_column: "ca_p_ratio",
        storage_unit: Unit::Ratio,
        category: NutrientCategory::Ratios,
        name_en: "Ca:P Ratio",
        name_ru: "Соотношение Ca:P",
    },
];

pub fn get_nutrient(key: &str) -> Option<&'static NutrientDef> {
    NUTRIENTS.iter().find(|n| n.key == key)
}

pub fn get_nutrient_by_column(column: &str) -> Option<&'static NutrientDef> {
    NUTRIENTS.iter().find(|n| n.db_column == column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_keys_are_unique() {
        let mut keys: Vec<&str> = NUTRIENTS.iter().map(|n| n.key).collect();
        let original_len = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), original_len);
    }

    #[test]
    fn get_nutrient_by_column_supported_key() {
        let def = get_nutrient_by_column("vit_d3").expect("vit_d3 column should be found");
        assert_eq!(def.key, "vitamin_d3");
        assert_eq!(def.storage_unit, Unit::IuPerKg);
    }

    #[test]
    fn names_non_empty() {
        for def in NUTRIENTS {
            assert!(!def.name_en.is_empty());
            assert!(!def.name_ru.is_empty());
        }
    }
}
