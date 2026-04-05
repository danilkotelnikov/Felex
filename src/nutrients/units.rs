//! Canonical unit system for all nutrient values in Felex.
//!
//! Every nutrient value stored or computed in the system carries a `Unit`.
//! Conversion helpers allow normalising values to a common base before
//! arithmetic comparisons.

/// Unit of measurement for a nutrient value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    /// Grams per kilogram of dry matter (g/kg DM)
    GramPerKg,
    /// Milligrams per kilogram of dry matter (mg/kg DM)
    MgPerKg,
    /// Micrograms per kilogram of dry matter (µg/kg DM)
    McgPerKg,
    /// Megajoules per kilogram of dry matter (MJ/kg DM)
    MjPerKg,
    /// International Units per kilogram of dry matter (IU/kg DM)
    IuPerKg,
    /// Percentage of dry matter (%)
    Percent,
    /// Dimensionless ratio (e.g. Ca:P)
    Ratio,
}

impl Unit {
    /// Convert a value expressed in `self` to the canonical base unit.
    ///
    /// Base units:
    /// - Mass nutrients  → g/kg DM  (`GramPerKg`)
    /// - Energy          → MJ/kg DM (`MjPerKg`)
    /// - IU vitamins     → IU/kg DM (`IuPerKg`)
    /// - Percent         → g/kg DM  (× 10)
    /// - Ratio           → unchanged
    pub fn to_base(self, value: f64) -> f64 {
        match self {
            Unit::GramPerKg => value,
            Unit::MgPerKg => value / 1_000.0,
            Unit::McgPerKg => value / 1_000_000.0,
            Unit::MjPerKg => value,
            Unit::IuPerKg => value,
            Unit::Percent => value * 10.0,
            Unit::Ratio => value,
        }
    }

    /// Convert a value from the canonical base unit back to `self`.
    pub fn from_base(self, base_value: f64) -> f64 {
        match self {
            Unit::GramPerKg => base_value,
            Unit::MgPerKg => base_value * 1_000.0,
            Unit::McgPerKg => base_value * 1_000_000.0,
            Unit::MjPerKg => base_value,
            Unit::IuPerKg => base_value,
            Unit::Percent => base_value / 10.0,
            Unit::Ratio => base_value,
        }
    }

    /// Short display suffix for UI labels.
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::GramPerKg => "г/кг",
            Unit::MgPerKg => "мг/кг",
            Unit::McgPerKg => "мкг/кг",
            Unit::MjPerKg => "МДж/кг",
            Unit::IuPerKg => "МЕ/кг",
            Unit::Percent => "%",
            Unit::Ratio => "",
        }
    }

    /// English display suffix.
    pub fn suffix_en(self) -> &'static str {
        match self {
            Unit::GramPerKg => "g/kg",
            Unit::MgPerKg => "mg/kg",
            Unit::McgPerKg => "µg/kg",
            Unit::MjPerKg => "MJ/kg",
            Unit::IuPerKg => "IU/kg",
            Unit::Percent => "%",
            Unit::Ratio => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mg_per_kg_round_trip() {
        let base = Unit::MgPerKg.to_base(500.0); // 0.5 g/kg
        assert!((base - 0.5).abs() < f64::EPSILON);
        let back = Unit::MgPerKg.from_base(base);
        assert!((back - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mcg_per_kg_round_trip() {
        let base = Unit::McgPerKg.to_base(1_000_000.0); // 1.0 g/kg
        assert!((base - 1.0).abs() < f64::EPSILON);
        let back = Unit::McgPerKg.from_base(base);
        assert!((back - 1_000_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percent_to_base() {
        // 10 % == 100 g/kg
        let base = Unit::Percent.to_base(10.0);
        assert!((base - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn gram_per_kg_identity() {
        assert!((Unit::GramPerKg.to_base(42.0) - 42.0).abs() < f64::EPSILON);
        assert!((Unit::GramPerKg.from_base(42.0) - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ratio_identity() {
        assert!((Unit::Ratio.to_base(2.5) - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn suffix_non_empty_for_all_variants() {
        let units = [
            Unit::GramPerKg,
            Unit::MgPerKg,
            Unit::McgPerKg,
            Unit::MjPerKg,
            Unit::IuPerKg,
            Unit::Percent,
        ];
        for u in units {
            assert!(!u.suffix().is_empty(), "{u:?} suffix should not be empty");
        }
    }
}
