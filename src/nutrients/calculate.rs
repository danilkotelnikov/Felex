//! Nutrient calculation utilities for Felex.
//!
//! This module provides the core arithmetic transformations needed when
//! computing how much of each nutrient an animal receives from its daily
//! ration:
//!
//! - **Daily amount** – scale a per-kg concentration by the feed mass fed.
//! - **Dry-matter basis** – convert between as-fed and dry-matter values.
//! - **Combined helper** – unit-aware daily nutrient intake from one feed.
//!
//! All functions are pure and operate on `f64` values.  They do not depend on
//! any database or I/O and are therefore straightforward to unit-test.

use super::units::Unit;

// ── Daily amount ──────────────────────────────────────────────────────────────

/// Convert a per-kg concentration into the total daily amount for one feed.
///
/// # Parameters
/// - `value_per_kg`     – nutrient concentration in the feed (any consistent unit per kg).
/// - `feed_amount_kg`   – daily feed mass offered (kg as-fed).
///
/// # Returns
/// Total daily amount in the same unit as `value_per_kg` (multiplied by kg).
///
/// # Example
/// ```no_run
/// use felex::nutrients::calculate::to_daily_amount;
///
/// // 150 g crude protein per kg × 5 kg feed = 750 g/day
/// let daily = to_daily_amount(150.0, 5.0);
/// assert!((daily - 750.0).abs() < f64::EPSILON);
/// ```
pub fn to_daily_amount(value_per_kg: f64, feed_amount_kg: f64) -> f64 {
    value_per_kg * feed_amount_kg
}

// ── Dry-matter basis conversions ──────────────────────────────────────────────

/// Convert an as-fed nutrient concentration to a dry-matter (DM) basis.
///
/// # Parameters
/// - `as_fed_value`   – nutrient concentration expressed per kg as-fed.
/// - `dm_fraction`    – dry-matter content as a fraction in `[0.0, 1.0]`.
///                      E.g. 88 % DM → `0.88`.
///
/// # Returns
/// Nutrient concentration per kg DM, or `0.0` if `dm_fraction` is zero
/// (to avoid division by zero).
///
/// # Example
/// ```no_run
/// use felex::nutrients::calculate::to_dm_basis;
///
/// // 120 g/kg as-fed with 80 % DM → 150 g/kg DM
/// let dm = to_dm_basis(120.0, 0.80);
/// assert!((dm - 150.0).abs() < 1e-9);
/// ```
pub fn to_dm_basis(as_fed_value: f64, dm_fraction: f64) -> f64 {
    if dm_fraction == 0.0 {
        return 0.0;
    }
    as_fed_value / dm_fraction
}

/// Convert a dry-matter basis concentration back to an as-fed basis.
///
/// # Parameters
/// - `dm_value`      – nutrient concentration expressed per kg DM.
/// - `dm_fraction`   – dry-matter content as a fraction in `[0.0, 1.0]`.
///
/// # Returns
/// Nutrient concentration per kg as-fed.
///
/// # Example
/// ```no_run
/// use felex::nutrients::calculate::from_dm_basis;
///
/// // 150 g/kg DM with 80 % DM → 120 g/kg as-fed
/// let as_fed = from_dm_basis(150.0, 0.80);
/// assert!((as_fed - 120.0).abs() < 1e-9);
/// ```
pub fn from_dm_basis(dm_value: f64, dm_fraction: f64) -> f64 {
    dm_value * dm_fraction
}

// ── Combined: daily nutrient from one feed ────────────────────────────────────

/// Compute the daily intake of a nutrient from a single feed ingredient.
///
/// This function chains unit normalisation and daily-amount calculation:
/// 1. Convert `stored_value` from `storage_unit` to the canonical base unit.
/// 2. Multiply by `feed_amount_kg` to get the total daily amount.
///
/// The result is expressed in the canonical base unit for the nutrient's
/// unit family (see [`Unit::to_base`]).
///
/// # Parameters
/// - `stored_value`   – nutrient concentration as stored in the database.
/// - `storage_unit`   – the [`Unit`] in which the value is stored.
/// - `feed_amount_kg` – daily mass of this feed offered to the animal (kg).
///
/// # Example
/// ```no_run
/// use felex::nutrients::{calculate::nutrient_from_feed, units::Unit};
///
/// // 500 mg Cu per kg × 3 kg feed = 1 500 mg → 1.5 g (base unit)
/// let daily_base = nutrient_from_feed(500.0, Unit::MgPerKg, 3.0);
/// assert!((daily_base - 1.5).abs() < 1e-9);
/// ```
pub fn nutrient_from_feed(stored_value: f64, storage_unit: Unit, feed_amount_kg: f64) -> f64 {
    let base_value = storage_unit.to_base(stored_value);
    to_daily_amount(base_value, feed_amount_kg)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── to_daily_amount ───────────────────────────────────────────────────────

    #[test]
    fn daily_amount_basic() {
        // 150 g/kg × 5 kg = 750 g
        assert!((to_daily_amount(150.0, 5.0) - 750.0).abs() < f64::EPSILON);
    }

    #[test]
    fn daily_amount_zero_feed() {
        assert!((to_daily_amount(150.0, 0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn daily_amount_zero_concentration() {
        assert!((to_daily_amount(0.0, 10.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn daily_amount_fractional_kg() {
        // 200 g/kg × 0.5 kg = 100 g
        assert!((to_daily_amount(200.0, 0.5) - 100.0).abs() < f64::EPSILON);
    }

    // ── to_dm_basis ───────────────────────────────────────────────────────────

    #[test]
    fn to_dm_basis_typical() {
        // 120 g/kg as-fed, 80 % DM → 150 g/kg DM
        let dm = to_dm_basis(120.0, 0.80);
        assert!((dm - 150.0).abs() < 1e-9, "dm={dm}");
    }

    #[test]
    fn to_dm_basis_full_dm() {
        // 100 % DM → value unchanged
        let dm = to_dm_basis(85.0, 1.0);
        assert!((dm - 85.0).abs() < f64::EPSILON);
    }

    #[test]
    fn to_dm_basis_zero_dm_returns_zero() {
        // Guard against division by zero
        let dm = to_dm_basis(100.0, 0.0);
        assert!((dm - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn to_dm_basis_round_trip_with_from_dm_basis() {
        let original_as_fed = 90.0_f64;
        let dm_fraction = 0.88_f64;

        let dm = to_dm_basis(original_as_fed, dm_fraction);
        let back = from_dm_basis(dm, dm_fraction);

        assert!(
            (back - original_as_fed).abs() < 1e-9,
            "round-trip failed: {back} != {original_as_fed}"
        );
    }

    // ── from_dm_basis ─────────────────────────────────────────────────────────

    #[test]
    fn from_dm_basis_typical() {
        // 150 g/kg DM, 80 % DM → 120 g/kg as-fed
        let as_fed = from_dm_basis(150.0, 0.80);
        assert!((as_fed - 120.0).abs() < 1e-9, "as_fed={as_fed}");
    }

    #[test]
    fn from_dm_basis_full_dm() {
        let as_fed = from_dm_basis(85.0, 1.0);
        assert!((as_fed - 85.0).abs() < f64::EPSILON);
    }

    #[test]
    fn from_dm_basis_zero_dm_returns_zero() {
        let as_fed = from_dm_basis(200.0, 0.0);
        assert!((as_fed - 0.0).abs() < f64::EPSILON);
    }

    // ── nutrient_from_feed ────────────────────────────────────────────────────

    #[test]
    fn nutrient_from_feed_mg_per_kg() {
        // 500 mg/kg × 3 kg → 1 500 mg → 1.5 g (base)
        let daily = nutrient_from_feed(500.0, Unit::MgPerKg, 3.0);
        assert!((daily - 1.5).abs() < 1e-9, "daily={daily}");
    }

    #[test]
    fn nutrient_from_feed_gram_per_kg() {
        // 150 g/kg × 5 kg → 750 g (base = g)
        let daily = nutrient_from_feed(150.0, Unit::GramPerKg, 5.0);
        assert!((daily - 750.0).abs() < f64::EPSILON);
    }

    #[test]
    fn nutrient_from_feed_percent() {
        // 10 % × 2 kg → base = 10 × 10 = 100 g/kg × 2 kg = 200 g
        let daily = nutrient_from_feed(10.0, Unit::Percent, 2.0);
        assert!((daily - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn nutrient_from_feed_mj_per_kg() {
        // 12.5 MJ/kg × 10 kg = 125 MJ
        let daily = nutrient_from_feed(12.5, Unit::MjPerKg, 10.0);
        assert!((daily - 125.0).abs() < f64::EPSILON);
    }

    #[test]
    fn nutrient_from_feed_iu_per_kg() {
        // 5000 IU/kg × 4 kg = 20 000 IU
        let daily = nutrient_from_feed(5000.0, Unit::IuPerKg, 4.0);
        assert!((daily - 20_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn nutrient_from_feed_mcg_per_kg() {
        // 1_000_000 µg/kg × 1 kg → base = 1.0 g/kg × 1 kg = 1.0 g
        let daily = nutrient_from_feed(1_000_000.0, Unit::McgPerKg, 1.0);
        assert!((daily - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn nutrient_from_feed_zero_feed_amount() {
        let daily = nutrient_from_feed(200.0, Unit::GramPerKg, 0.0);
        assert!((daily - 0.0).abs() < f64::EPSILON);
    }
}
