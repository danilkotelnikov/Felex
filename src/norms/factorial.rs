//! Factorial nutrient calculation framework.

use super::AnimalContext;

/// Trait for factorial nutrient requirement calculations.
pub trait NutrientCalculator {
    /// Maintenance requirement (basal metabolism).
    fn maintenance(&self, ctx: &AnimalContext) -> f64;

    /// Production requirement (milk, eggs, etc.).
    fn production(&self, ctx: &AnimalContext) -> f64;

    /// Growth requirement (weight gain).
    fn growth(&self, ctx: &AnimalContext) -> f64;

    /// Gestation requirement (pregnancy).
    fn gestation(&self, ctx: &AnimalContext) -> f64;

    /// Total requirement across all modeled components.
    fn total(&self, ctx: &AnimalContext) -> f64 {
        self.maintenance(ctx)
            + self.production(ctx)
            + self.growth(ctx)
            + self.gestation(ctx)
    }

    /// Safety margins for min and max requirement bounds.
    fn min_max_margin(&self) -> (f64, f64);

    /// Lower bound derived from total requirement.
    fn min_requirement(&self, ctx: &AnimalContext) -> f64 {
        let (min_factor, _) = self.min_max_margin();
        self.total(ctx) * min_factor
    }

    /// Upper bound derived from total requirement.
    fn max_requirement(&self, ctx: &AnimalContext) -> f64 {
        let (_, max_factor) = self.min_max_margin();
        self.total(ctx) * max_factor
    }
}

/// Constraint priority tier for optimizer integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintTier {
    /// Hard constraints that must be satisfied.
    Tier1,
    /// Standard constraints that should be satisfied.
    Tier2,
    /// Soft constraints to satisfy when feasible.
    Tier3,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCalculator;

    impl NutrientCalculator for MockCalculator {
        fn maintenance(&self, _ctx: &AnimalContext) -> f64 {
            10.0
        }

        fn production(&self, _ctx: &AnimalContext) -> f64 {
            5.0
        }

        fn growth(&self, _ctx: &AnimalContext) -> f64 {
            3.0
        }

        fn gestation(&self, _ctx: &AnimalContext) -> f64 {
            0.0
        }

        fn min_max_margin(&self) -> (f64, f64) {
            (0.95, 1.10)
        }
    }

    fn test_context() -> AnimalContext {
        AnimalContext {
            species: Some("cattle".to_string()),
            production_type: Some("beef".to_string()),
            live_weight_kg: Some(450.0),
            daily_gain_g: Some(1_000),
            ..Default::default()
        }
    }

    #[test]
    fn total_sums_components() {
        let calc = MockCalculator;
        let ctx = test_context();

        assert_eq!(calc.total(&ctx), 18.0);
    }

    #[test]
    fn min_max_apply_margins() {
        let calc = MockCalculator;
        let ctx = test_context();

        assert!((calc.min_requirement(&ctx) - 17.1).abs() < 0.01);
        assert!((calc.max_requirement(&ctx) - 19.8).abs() < 0.01);
    }
}
