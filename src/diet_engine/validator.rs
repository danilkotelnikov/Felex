//! Ration validation module

use super::{nutrient_calc, NutritionWarning};
use crate::db::{feed_labels::display_feed_name, rations::RationItem};
use crate::norms::AnimalNorm;

/// Validate a ration and return warnings
pub fn validate(items: &[RationItem], norms: &AnimalNorm) -> Vec<NutritionWarning> {
    let mut warnings = Vec::new();

    let nutrients = nutrient_calc::calculate_nutrients(items);

    // Check energy
    if let Some(min_eke) = norms.nutrients_min.get("energy_eke") {
        if nutrients.energy_eke < *min_eke {
            let deficit_pct = (1.0 - nutrients.energy_eke / min_eke) * 100.0;
            warnings.push(NutritionWarning::EnergyDeficit { deficit_pct });
        }
    }

    // Check protein
    if let Some(min_cp) = norms.nutrients_min.get("crude_protein") {
        if nutrients.crude_protein < *min_cp {
            let deficit_g = min_cp - nutrients.crude_protein;
            warnings.push(NutritionWarning::ProteinDeficit { deficit_g });
        }
    }

    // Check Ca:P ratio (should be between 1.5 and 2.0 for most animals)
    if nutrients.phosphorus > 0.0 {
        let ratio = nutrients.ca_p_ratio;
        if ratio < 1.2 || ratio > 2.5 {
            warnings.push(NutritionWarning::CaPhosImbalance {
                ratio,
                normal: (1.5, 2.0),
            });
        }
    }

    // Check starch for ruminants (should be < 32% DM to avoid acidosis risk)
    if norms.species == "cattle" && nutrients.total_dm_kg > 0.0 && nutrients.starch_pct_dm > 32.0 {
        warnings.push(NutritionWarning::HighStarchRumen {
            starch_pct: nutrients.starch_pct_dm,
        });
    }

    // Check for feeds without prices
    let feeds_without_price: Vec<String> = items
        .iter()
        .filter_map(|i| {
            let feed = i.feed.as_ref()?;
            if feed.price_per_ton.is_none() || feed.price_per_ton == Some(0.0) {
                Some(display_feed_name(feed))
            } else {
                None
            }
        })
        .collect();

    if !feeds_without_price.is_empty() {
        warnings.push(NutritionWarning::PriceDataMissing {
            feed_names: feeds_without_price,
        });
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_ration_no_crash() {
        let items = vec![];
        let norms = AnimalNorm {
            species: "cattle".to_string(),
            ..Default::default()
        };
        let warnings = validate(&items, &norms);
        assert!(warnings.is_empty());
    }
}
