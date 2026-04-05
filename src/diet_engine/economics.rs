//! Economic calculations module

use crate::db::rations::RationItem;
use serde::{Deserialize, Serialize};

/// Category cost breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCost {
    pub category: String,
    pub cost_per_day: f64,
    pub percentage: f64,
}

/// Economic analysis of a ration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicAnalysis {
    pub feed_cost_per_day: f64,
    pub feed_cost_per_month: f64,
    pub feed_cost_per_year: f64,
    pub feed_cost_per_unit: f64, // rubles per kg of milk/gain
    pub cost_by_category: Vec<CategoryCost>,
    pub cost_per_animal_day: f64,
    pub animal_count: i32,
    pub total_daily_cost: f64,
}

impl Default for EconomicAnalysis {
    fn default() -> Self {
        Self {
            feed_cost_per_day: 0.0,
            feed_cost_per_month: 0.0,
            feed_cost_per_year: 0.0,
            feed_cost_per_unit: 0.0,
            cost_by_category: vec![],
            cost_per_animal_day: 0.0,
            animal_count: 1,
            total_daily_cost: 0.0,
        }
    }
}

/// Calculate economics for ration items
pub fn calculate_economics(items: &[RationItem], animal_count: i32) -> EconomicAnalysis {
    let mut category_costs: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();
    let mut total_cost = 0.0;

    for item in items {
        let feed = match &item.feed {
            Some(f) => f,
            None => continue,
        };

        let cost = item.amount_kg * feed.price_per_kg();
        total_cost += cost;

        let category = feed.category.clone();
        *category_costs.entry(category).or_insert(0.0) += cost;
    }

    let cost_by_category: Vec<CategoryCost> = category_costs
        .into_iter()
        .map(|(category, cost)| CategoryCost {
            category,
            cost_per_day: cost,
            percentage: if total_cost > 0.0 {
                cost / total_cost * 100.0
            } else {
                0.0
            },
        })
        .collect();

    let total_daily_cost = total_cost * animal_count as f64;

    EconomicAnalysis {
        feed_cost_per_day: total_cost,
        feed_cost_per_month: total_cost * 30.0,
        feed_cost_per_year: total_cost * 365.0,
        feed_cost_per_unit: 0.0, // Would need production data to calculate
        cost_by_category,
        cost_per_animal_day: total_cost,
        animal_count,
        total_daily_cost,
    }
}

/// Compare two rations economically
pub fn compare_rations(
    ration_a: &EconomicAnalysis,
    ration_b: &EconomicAnalysis,
) -> RationComparison {
    let daily_diff = ration_b.feed_cost_per_day - ration_a.feed_cost_per_day;
    let monthly_diff = daily_diff * 30.0;
    let yearly_diff = daily_diff * 365.0;

    RationComparison {
        daily_difference: daily_diff,
        monthly_difference: monthly_diff,
        yearly_difference: yearly_diff,
        percent_difference: if ration_a.feed_cost_per_day > 0.0 {
            (daily_diff / ration_a.feed_cost_per_day) * 100.0
        } else {
            0.0
        },
    }
}

/// Ration comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RationComparison {
    pub daily_difference: f64,
    pub monthly_difference: f64,
    pub yearly_difference: f64,
    pub percent_difference: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::feeds::Feed;

    #[test]
    fn test_economics_calculation() {
        let items = vec![RationItem {
            id: Some(1),
            ration_id: 1,
            feed_id: 1,
            feed: Some(Feed {
                name_ru: "Test Feed".to_string(),
                category: "concentrate".to_string(),
                price_per_ton: Some(15000.0), // 15 rubles/kg
                ..Default::default()
            }),
            amount_kg: 5.0,
            is_locked: false,
            sort_order: 1,
        }];

        let analysis = calculate_economics(&items, 1);

        assert!((analysis.feed_cost_per_day - 75.0).abs() < 0.01);
        assert!((analysis.feed_cost_per_month - 2250.0).abs() < 0.01);
    }
}
