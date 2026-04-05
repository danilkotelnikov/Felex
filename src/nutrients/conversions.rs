//! Cross-nutrient conversion layer for Felex.
//!
//! The current implemented feed authority only carries direct nutrient values.
//! No cross-walk from unsupported nutrients should be injected into the runtime
//! surface, so the conversion table is intentionally empty.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Species {
    Cattle,
    Swine,
    Poultry,
}

impl Species {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cattle" | "cow" | "beef" | "dairy" | "крс" | "корова" | "говядина" => {
                Some(Species::Cattle)
            }
            "swine" | "pig" | "hog" | "pork" | "свинья" | "свиньи" | "свиноводство" => {
                Some(Species::Swine)
            }
            "poultry" | "chicken" | "broiler" | "layer" | "turkey" | "птица" | "курица"
            | "бройлер" => Some(Species::Poultry),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionMode {
    Stacked,
    FillIfMissing,
}

pub struct Conversion {
    pub source_key: &'static str,
    pub target_key: &'static str,
    pub get_factor: fn(Species) -> Option<f64>,
    pub mode: ConversionMode,
}

pub static CONVERSIONS: &[Conversion] = &[];

pub fn apply_conversions(_nutrients: &mut HashMap<String, f64>, _species: Species) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn species_from_str_supported_variants() {
        assert_eq!(Species::from_str("cattle"), Some(Species::Cattle));
        assert_eq!(Species::from_str("pig"), Some(Species::Swine));
        assert_eq!(Species::from_str("птица"), Some(Species::Poultry));
    }

    #[test]
    fn no_runtime_conversions_are_registered() {
        assert!(CONVERSIONS.is_empty());
    }

    #[test]
    fn apply_conversions_is_noop() {
        let mut nutrients = HashMap::from([
            ("carotene".to_string(), 10.0),
            ("vitamin_e".to_string(), 25.0),
        ]);
        apply_conversions(&mut nutrients, Species::Cattle);
        assert_eq!(nutrients.get("carotene"), Some(&10.0));
        assert_eq!(nutrients.get("vitamin_e"), Some(&25.0));
        assert_eq!(nutrients.len(), 2);
    }
}
