//! Database schema and migrations

use anyhow::Result;
use rusqlite::Connection;

/// Run all database migrations
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Create migrations table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            applied_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Run each migration if not already applied
    let migrations = vec![
        ("001_initial", MIGRATION_001_INITIAL),
        ("002_feeds", MIGRATION_002_FEEDS),
        ("003_norms", MIGRATION_003_NORMS),
        ("004_rations", MIGRATION_004_RATIONS),
        ("005_prices", MIGRATION_005_PRICES),
        ("006_feed_carotene", MIGRATION_006_FEED_CAROTENE),
        ("007_feeds_fts5_search", MIGRATION_007_FEEDS_FTS5_SEARCH),
        ("008_trim_feed_nutrients", MIGRATION_008_TRIM_FEED_NUTRIENTS),
        ("009_scraped_prices", MIGRATION_009_SCRAPED_PRICES),
        ("010_add_vitamins_minerals", MIGRATION_010_ADD_VITAMINS_MINERALS),
    ];

    for (name, sql) in migrations {
        let applied: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM migrations WHERE name = ?",
            [name],
            |row| row.get(0),
        )?;

        if !applied {
            tracing::info!("Applying migration: {}", name);
            conn.execute_batch(sql)?;
            conn.execute("INSERT INTO migrations (name) VALUES (?)", [name])?;
        }
    }

    Ok(())
}

const MIGRATION_001_INITIAL: &str = r#"
-- Initial schema setup
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO app_settings (key, value) VALUES ('schema_version', '1');
"#;

const MIGRATION_002_FEEDS: &str = r#"
-- Feeds table
CREATE TABLE IF NOT EXISTS feeds (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id       TEXT,
    source_url      TEXT,
    name_ru         TEXT NOT NULL,
    name_en         TEXT,
    category        TEXT NOT NULL,
    subcategory     TEXT,

    -- ENERGY
    dry_matter       REAL,
    energy_oe_cattle REAL,
    energy_oe_pig    REAL,
    energy_oe_poultry REAL,
    koe              REAL,

    -- PROTEIN
    crude_protein        REAL,
    dig_protein_cattle   REAL,
    dig_protein_pig      REAL,
    dig_protein_poultry  REAL,
    lysine               REAL,
    methionine_cystine   REAL,

    -- FAT AND FIBER
    crude_fat   REAL,
    crude_fiber REAL,

    -- CARBOHYDRATES
    starch REAL,
    sugar  REAL,

    -- ASH AND MINERALS
    calcium          REAL,
    phosphorus       REAL,
    magnesium        REAL,
    potassium        REAL,
    sodium           REAL,
    sulfur           REAL,
    iron             REAL,
    copper           REAL,
    zinc             REAL,
    manganese        REAL,
    cobalt           REAL,
    iodine           REAL,

    -- VITAMINS
    vit_d3  REAL,
    vit_e   REAL,

    -- OTHER
    moisture             REAL,
    feed_conversion      REAL,
    palatability         INTEGER,
    max_inclusion_cattle REAL,
    max_inclusion_pig    REAL,
    max_inclusion_poultry REAL,

    -- ECONOMICS
    price_per_ton    REAL,
    price_updated_at TEXT,
    region           TEXT,

    -- METADATA
    is_custom   INTEGER DEFAULT 0,
    verified    INTEGER DEFAULT 0,
    notes       TEXT,
    created_at  TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at  TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_feeds_category ON feeds(category);
CREATE INDEX IF NOT EXISTS idx_feeds_name ON feeds(name_ru);
"#;

const MIGRATION_003_NORMS: &str = r#"
-- Animal norms table
CREATE TABLE IF NOT EXISTS animal_groups (
    id TEXT PRIMARY KEY,
    species TEXT NOT NULL,
    production_type TEXT,
    name_ru TEXT NOT NULL,
    name_en TEXT,
    description TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS animal_norms (
    id               INTEGER PRIMARY KEY,
    species          TEXT NOT NULL,
    production_type  TEXT,
    breed_group      TEXT,
    sex              TEXT,
    age_from_days    INTEGER,
    age_to_days      INTEGER,
    weight_from_kg   REAL,
    weight_to_kg     REAL,
    milk_yield_kg    REAL,
    milk_fat_pct     REAL,
    milk_protein_pct REAL,
    daily_gain_g     INTEGER,
    nutrients_min    TEXT,
    nutrients_max    TEXT,
    nutrients_target TEXT,
    feed_intake_min  REAL,
    feed_intake_max  REAL,
    notes            TEXT,
    source           TEXT,
    created_at       TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_norms_species ON animal_norms(species);
"#;

const MIGRATION_004_RATIONS: &str = r#"
-- Rations table
CREATE TABLE IF NOT EXISTS rations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    animal_group_id TEXT,
    animal_count INTEGER DEFAULT 1,
    description TEXT,
    status TEXT DEFAULT 'draft',
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ration_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ration_id INTEGER NOT NULL REFERENCES rations(id) ON DELETE CASCADE,
    feed_id INTEGER NOT NULL REFERENCES feeds(id),
    amount_kg REAL NOT NULL DEFAULT 0,
    is_locked INTEGER DEFAULT 0,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ration_items_ration ON ration_items(ration_id);
"#;

const MIGRATION_005_PRICES: &str = r#"
-- Price history table
CREATE TABLE IF NOT EXISTS feed_prices (
    id                    INTEGER PRIMARY KEY,
    feed_id               INTEGER REFERENCES feeds(id),
    region                TEXT,
    price_rubles_per_ton  REAL,
    price_date            TEXT,
    source                TEXT,
    notes                 TEXT,
    created_at            TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS feed_price_history (
    feed_id     INTEGER,
    region      TEXT,
    price       REAL,
    recorded_at TEXT,
    PRIMARY KEY (feed_id, region, recorded_at)
);

CREATE INDEX IF NOT EXISTS idx_prices_feed ON feed_prices(feed_id);
"#;

const MIGRATION_006_FEED_CAROTENE: &str = r#"
ALTER TABLE feeds ADD COLUMN carotene REAL;
"#;

const MIGRATION_007_FEEDS_FTS5_SEARCH: &str = include_str!("migrations/007_fts5_search.sql");

const MIGRATION_008_TRIM_FEED_NUTRIENTS: &str = r#"
DROP TRIGGER IF EXISTS feeds_ai;
DROP TRIGGER IF EXISTS feeds_ad;
DROP TRIGGER IF EXISTS feeds_au;
DROP TABLE IF EXISTS feeds_fts;

CREATE TABLE feeds_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id TEXT,
    source_url TEXT,
    name_ru TEXT NOT NULL,
    name_en TEXT,
    category TEXT NOT NULL,
    subcategory TEXT,
    dry_matter REAL,
    energy_oe_cattle REAL,
    energy_oe_pig REAL,
    energy_oe_poultry REAL,
    koe REAL,
    crude_protein REAL,
    dig_protein_cattle REAL,
    dig_protein_pig REAL,
    dig_protein_poultry REAL,
    lysine REAL,
    methionine_cystine REAL,
    crude_fat REAL,
    crude_fiber REAL,
    starch REAL,
    sugar REAL,
    calcium REAL,
    phosphorus REAL,
    magnesium REAL,
    potassium REAL,
    sodium REAL,
    sulfur REAL,
    iron REAL,
    copper REAL,
    zinc REAL,
    manganese REAL,
    cobalt REAL,
    iodine REAL,
    carotene REAL,
    vit_d3 REAL,
    vit_e REAL,
    moisture REAL,
    feed_conversion REAL,
    palatability INTEGER,
    max_inclusion_cattle REAL,
    max_inclusion_pig REAL,
    max_inclusion_poultry REAL,
    price_per_ton REAL,
    price_updated_at TEXT,
    region TEXT,
    is_custom INTEGER DEFAULT 0,
    verified INTEGER DEFAULT 0,
    notes TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO feeds_new (
    id, source_id, source_url, name_ru, name_en, category, subcategory,
    dry_matter, energy_oe_cattle, energy_oe_pig, energy_oe_poultry, koe,
    crude_protein, dig_protein_cattle, dig_protein_pig, dig_protein_poultry,
    lysine, methionine_cystine, crude_fat, crude_fiber, starch, sugar,
    calcium, phosphorus, magnesium, potassium, sodium, sulfur,
    iron, copper, zinc, manganese, cobalt, iodine, carotene, vit_d3, vit_e,
    moisture, feed_conversion, palatability, max_inclusion_cattle, max_inclusion_pig, max_inclusion_poultry,
    price_per_ton, price_updated_at, region, is_custom, verified, notes, created_at, updated_at
)
SELECT
    id, source_id, source_url, name_ru, name_en, category, subcategory,
    dry_matter, energy_oe_cattle, energy_oe_pig, energy_oe_poultry, koe,
    crude_protein, dig_protein_cattle, dig_protein_pig, dig_protein_poultry,
    lysine, methionine_cystine, crude_fat, crude_fiber, starch, sugar,
    calcium, phosphorus, magnesium, potassium, sodium, sulfur,
    iron, copper, zinc, manganese, cobalt, iodine, carotene, vit_d3, vit_e,
    moisture, feed_conversion, palatability, max_inclusion_cattle, max_inclusion_pig, max_inclusion_poultry,
    price_per_ton, price_updated_at, region, is_custom, verified, notes, created_at, updated_at
FROM feeds;

DROP TABLE feeds;
ALTER TABLE feeds_new RENAME TO feeds;

CREATE INDEX IF NOT EXISTS idx_feeds_category ON feeds(category);
CREATE INDEX IF NOT EXISTS idx_feeds_name ON feeds(name_ru);

CREATE VIRTUAL TABLE IF NOT EXISTS feeds_fts USING fts5(
    name_ru,
    name_en,
    subcategory,
    content='feeds',
    content_rowid='id'
);

CREATE TRIGGER IF NOT EXISTS feeds_ai AFTER INSERT ON feeds BEGIN
    INSERT INTO feeds_fts(rowid, name_ru, name_en, subcategory)
    VALUES (NEW.id, NEW.name_ru, COALESCE(NEW.name_en, ''), COALESCE(NEW.subcategory, ''));
END;

CREATE TRIGGER IF NOT EXISTS feeds_ad AFTER DELETE ON feeds BEGIN
    INSERT INTO feeds_fts(feeds_fts, rowid, name_ru, name_en, subcategory)
    VALUES ('delete', OLD.id, OLD.name_ru, COALESCE(OLD.name_en, ''), COALESCE(OLD.subcategory, ''));
END;

CREATE TRIGGER IF NOT EXISTS feeds_au AFTER UPDATE ON feeds BEGIN
    INSERT INTO feeds_fts(feeds_fts, rowid, name_ru, name_en, subcategory)
    VALUES ('delete', OLD.id, OLD.name_ru, COALESCE(OLD.name_en, ''), COALESCE(OLD.subcategory, ''));
    INSERT INTO feeds_fts(rowid, name_ru, name_en, subcategory)
    VALUES (NEW.id, NEW.name_ru, COALESCE(NEW.name_en, ''), COALESCE(NEW.subcategory, ''));
END;

INSERT INTO feeds_fts(feeds_fts) VALUES ('rebuild');
"#;

const MIGRATION_009_SCRAPED_PRICES: &str = r#"
-- Scraped price history table
CREATE TABLE IF NOT EXISTS scraped_feed_prices (
    feed_id INTEGER NOT NULL,
    price REAL NOT NULL,
    currency TEXT NOT NULL,
    source_url TEXT,
    confidence_score INTEGER,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    locale TEXT,
    PRIMARY KEY (feed_id, locale)
);
"#;

const MIGRATION_010_ADD_VITAMINS_MINERALS: &str = include_str!("migrations/010_add_vitamins_minerals.sql");

