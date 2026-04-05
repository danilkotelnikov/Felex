//! Factorial mineral calculators for cattle.

use super::factorial::NutrientCalculator;
use super::AnimalContext;

/// Calcium calculator using the cattle formulas from the research report.
pub struct CalciumCalculator;

impl NutrientCalculator for CalciumCalculator {
    fn maintenance(&self, ctx: &AnimalContext) -> f64 {
        let bw = ctx.live_weight_kg.unwrap_or(500.0);
        0.045 * bw
    }

    fn production(&self, ctx: &AnimalContext) -> f64 {
        let milk = ctx.milk_yield_kg.unwrap_or(0.0);
        milk * 1.3
    }

    fn growth(&self, ctx: &AnimalContext) -> f64 {
        let adg_kg = ctx.daily_gain_g.unwrap_or(0).max(0) as f64 / 1000.0;
        adg_kg * 14.0
    }

    fn gestation(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn min_max_margin(&self) -> (f64, f64) {
        (0.90, 1.20)
    }
}

impl CalciumCalculator {
    pub fn total_with_absorption(&self, ctx: &AnimalContext) -> f64 {
        self.total(ctx) / 0.38
    }
}

/// Phosphorus calculator using the cattle formulas from the research report.
pub struct PhosphorusCalculator;

impl NutrientCalculator for PhosphorusCalculator {
    fn maintenance(&self, ctx: &AnimalContext) -> f64 {
        let bw = ctx.live_weight_kg.unwrap_or(500.0);
        0.035 * bw
    }

    fn production(&self, ctx: &AnimalContext) -> f64 {
        let milk = ctx.milk_yield_kg.unwrap_or(0.0);
        milk * 0.95
    }

    fn growth(&self, ctx: &AnimalContext) -> f64 {
        let adg_kg = ctx.daily_gain_g.unwrap_or(0).max(0) as f64 / 1000.0;
        adg_kg * 8.5
    }

    fn gestation(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn min_max_margin(&self) -> (f64, f64) {
        (0.90, 1.15)
    }
}

impl PhosphorusCalculator {
    pub fn total_with_absorption(&self, ctx: &AnimalContext) -> f64 {
        self.total(ctx) / 0.58
    }
}

/// Magnesium approximation retained from the approved implementation plan.
pub struct MagnesiumCalculator;

impl NutrientCalculator for MagnesiumCalculator {
    fn maintenance(&self, ctx: &AnimalContext) -> f64 {
        let bw = ctx.live_weight_kg.unwrap_or(500.0);
        0.003 * bw
    }

    fn production(&self, ctx: &AnimalContext) -> f64 {
        let milk = ctx.milk_yield_kg.unwrap_or(0.0);
        milk * 0.12
    }

    fn growth(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn gestation(&self, _ctx: &AnimalContext) -> f64 {
        0.0
    }

    fn min_max_margin(&self) -> (f64, f64) {
        (0.85, 1.25)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calcium_dairy_cow_matches_factorial_range() {
        let ctx = AnimalContext {
            live_weight_kg: Some(650.0),
            milk_yield_kg: Some(35.0),
            ..Default::default()
        };
        let total = CalciumCalculator.total_with_absorption(&ctx);

        assert!(
            total > 190.0 && total < 200.0,
            "expected about 196.7 g/day calcium, got {total}"
        );
    }

    #[test]
    fn phosphorus_beef_finisher_matches_factorial_range() {
        let ctx = AnimalContext {
            live_weight_kg: Some(800.0),
            daily_gain_g: Some(900),
            ..Default::default()
        };
        let total = PhosphorusCalculator.total_with_absorption(&ctx);

        assert!(
            total > 60.0 && total < 65.0,
            "expected about 61.5 g/day phosphorus, got {total}"
        );
    }
}
