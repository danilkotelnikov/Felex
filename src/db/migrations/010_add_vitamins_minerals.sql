-- Migration 010: Add Vitamin A and Selenium
-- Date: 2026-03-26
-- Related: Felex_Legacy_Remediation_Log.md
-- Purpose: Add essential nutrients missing from original schema

-- Add Vitamin A column (IU/kg)
-- Vitamin A is essential for vision, immune function, and reproduction
-- 1 mg carotene converts to approximately 400 IU vitamin A in cattle
ALTER TABLE feeds ADD COLUMN vit_a REAL;

-- Add Selenium column (mg/kg)
-- Selenium is essential trace mineral for antioxidant function
-- Required for glutathione peroxidase enzyme activity
-- NASEM 2016: 0.1-0.3 mg/kg DM for cattle
-- FDA limit: Maximum 0.3 mg/kg complete feed
ALTER TABLE feeds ADD COLUMN selenium REAL;

-- Update feeds_fts5 search index to include new columns
DROP TABLE IF EXISTS feeds_fts;

CREATE VIRTUAL TABLE feeds_fts USING fts5(
    name_ru,
    name_en,
    subcategory,
    content='feeds',
    content_rowid='id'
);

-- Rebuild search index with updated schema
INSERT INTO feeds_fts(feeds_fts) VALUES ('rebuild');

-- Update schema version
UPDATE app_settings SET value = '10' WHERE key = 'schema_version';
