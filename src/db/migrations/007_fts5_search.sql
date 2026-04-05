-- Feed search index
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

-- Rebuild from the current feeds table so existing databases get indexed.
INSERT INTO feeds_fts(feeds_fts) VALUES ('rebuild');
