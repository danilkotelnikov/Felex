//! Canonical nutrient system for Felex.
//!
//! This module is the single source of truth for:
//! - Which nutrients exist and how they are categorised (`categories`)
//! - What units each nutrient is expressed in (`units`)
//!
//! # Planned sub-modules (not yet implemented)
//! - `manifest`    – full registry of all ~79 nutrients with metadata
//! - `conversions` – cross-unit conversion tables (e.g. IU → µg for vitamin A)
//! - `calculate`   – derived nutrient computation (e.g. NEL from ME)

pub mod calculate;
pub mod categories;
pub mod conversions;
pub mod manifest;
pub mod units;

pub use calculate::{from_dm_basis, nutrient_from_feed, to_daily_amount, to_dm_basis};
pub use categories::NutrientCategory;
pub use conversions::{apply_conversions, ConversionMode, Conversion, Species, CONVERSIONS};
pub use manifest::{get_nutrient, get_nutrient_by_column, NutrientDef, NUTRIENTS};
pub use units::Unit;
