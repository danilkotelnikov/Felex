//! Nutrient category taxonomy used for UI grouping and norm resolution ordering.

/// High-level category that a nutrient belongs to.
///
/// Categories determine how nutrients are grouped in the UI panel and the
/// order in which norm constraints are applied during LP optimisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientCategory {
    /// Dry matter, feed intake, moisture
    General,
    /// Gross energy, metabolisable energy, net energy
    Energy,
    /// Crude protein, digestible protein, true protein
    Protein,
    /// Essential and non-essential amino acids
    AminoAcids,
    /// Crude fiber, starch, sugars
    FiberCarbs,
    /// Crude fat, fatty acids
    Fats,
    /// Ca, P, Mg, Na, K, Cl, S
    Macrominerals,
    /// Fe, Cu, Zn, Mn, I, Se, Co, Mo
    TraceMinerals,
    /// Fat-soluble (A, D, E, K) and water-soluble (B-group, C) vitamins
    Vitamins,
    /// Derived ratios, e.g. Ca:P, lysine:energy
    Ratios,
}

impl NutrientCategory {
    /// English display label.
    pub fn label_en(self) -> &'static str {
        match self {
            NutrientCategory::General => "General",
            NutrientCategory::Energy => "Energy",
            NutrientCategory::Protein => "Protein",
            NutrientCategory::AminoAcids => "Amino Acids",
            NutrientCategory::FiberCarbs => "Fiber & Carbohydrates",
            NutrientCategory::Fats => "Fats",
            NutrientCategory::Macrominerals => "Macrominerals",
            NutrientCategory::TraceMinerals => "Trace Minerals",
            NutrientCategory::Vitamins => "Vitamins",
            NutrientCategory::Ratios => "Ratios",
        }
    }

    /// Russian display label.
    pub fn label_ru(self) -> &'static str {
        match self {
            NutrientCategory::General => "Общие показатели",
            NutrientCategory::Energy => "Энергия",
            NutrientCategory::Protein => "Протеин",
            NutrientCategory::AminoAcids => "Аминокислоты",
            NutrientCategory::FiberCarbs => "Клетчатка и углеводы",
            NutrientCategory::Fats => "Жиры",
            NutrientCategory::Macrominerals => "Макроэлементы",
            NutrientCategory::TraceMinerals => "Микроэлементы",
            NutrientCategory::Vitamins => "Витамины",
            NutrientCategory::Ratios => "Соотношения",
        }
    }

    /// Ascending sort key used to order category groups in the UI panel.
    ///
    /// Lower numbers appear first.
    pub fn sort_order(self) -> u8 {
        match self {
            NutrientCategory::General => 0,
            NutrientCategory::Energy => 1,
            NutrientCategory::Protein => 2,
            NutrientCategory::AminoAcids => 3,
            NutrientCategory::FiberCarbs => 4,
            NutrientCategory::Fats => 5,
            NutrientCategory::Macrominerals => 6,
            NutrientCategory::TraceMinerals => 7,
            NutrientCategory::Vitamins => 8,
            NutrientCategory::Ratios => 9,
        }
    }

    /// Iterate all variants in `sort_order` order.
    pub fn all_sorted() -> [NutrientCategory; 10] {
        [
            NutrientCategory::General,
            NutrientCategory::Energy,
            NutrientCategory::Protein,
            NutrientCategory::AminoAcids,
            NutrientCategory::FiberCarbs,
            NutrientCategory::Fats,
            NutrientCategory::Macrominerals,
            NutrientCategory::TraceMinerals,
            NutrientCategory::Vitamins,
            NutrientCategory::Ratios,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_order_is_unique() {
        let orders: Vec<u8> = NutrientCategory::all_sorted()
            .iter()
            .map(|c| c.sort_order())
            .collect();
        let mut sorted = orders.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), orders.len(), "sort_order values must be unique");
    }

    #[test]
    fn all_sorted_is_in_order() {
        let categories = NutrientCategory::all_sorted();
        for window in categories.windows(2) {
            assert!(
                window[0].sort_order() < window[1].sort_order(),
                "all_sorted must be strictly ascending"
            );
        }
    }

    #[test]
    fn labels_non_empty() {
        for cat in NutrientCategory::all_sorted() {
            assert!(!cat.label_en().is_empty());
            assert!(!cat.label_ru().is_empty());
        }
    }
}
