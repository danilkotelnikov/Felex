//! Feed import and synchronization adapters.

pub mod cap_ru;
pub mod multi_source;
pub mod normalized_feed_db;
pub mod price_fetcher;
pub mod marketplace;

pub use cap_ru::CapRuScraper;
pub use multi_source::{refresh_inferred_prices, seed_from_json_if_empty, sync_all, SyncReport};
pub use normalized_feed_db::load_workspace_seed_feeds;
pub use price_fetcher::PriceFetcher;
