//! Ration database operations

use super::feeds::Feed;
use anyhow::Result;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

/// Ration model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ration {
    pub id: Option<i64>,
    pub name: String,
    pub animal_group_id: Option<String>,
    pub animal_count: i32,
    pub description: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Default for Ration {
    fn default() -> Self {
        Self {
            id: None,
            name: "New Ration".to_string(),
            animal_group_id: None,
            animal_count: 1,
            description: None,
            status: "draft".to_string(),
            created_at: None,
            updated_at: None,
        }
    }
}

/// Ration item (feed in ration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RationItem {
    pub id: Option<i64>,
    pub ration_id: i64,
    pub feed_id: i64,
    pub feed: Option<Feed>,
    pub amount_kg: f64,
    pub is_locked: bool,
    pub sort_order: i32,
}

impl RationItem {
    /// Calculate dry matter kg
    pub fn dm_kg(&self) -> f64 {
        if let Some(ref feed) = self.feed {
            self.amount_kg * feed.dry_matter.unwrap_or(86.0) / 100.0
        } else {
            0.0
        }
    }
}

/// Full ration with items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RationFull {
    pub ration: Ration,
    pub items: Vec<RationItem>,
}

impl Ration {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            animal_group_id: row.get("animal_group_id")?,
            animal_count: row.get("animal_count")?,
            description: row.get("description")?,
            status: row.get("status")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

/// List all rations
pub fn list_rations(conn: &Connection) -> Result<Vec<Ration>> {
    let mut stmt = conn.prepare("SELECT * FROM rations ORDER BY updated_at DESC")?;

    let rations = stmt
        .query_map([], Ration::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rations)
}

/// Get a ration by ID with all items
pub fn get_ration(conn: &Connection, id: i64) -> Result<Option<RationFull>> {
    let ration: Option<Ration> = conn
        .query_row("SELECT * FROM rations WHERE id = ?", [id], Ration::from_row)
        .ok();

    match ration {
        Some(r) => {
            let items = get_ration_items(conn, id)?;
            Ok(Some(RationFull { ration: r, items }))
        }
        None => Ok(None),
    }
}

/// Get ration items with feed data
pub fn get_ration_items(conn: &Connection, ration_id: i64) -> Result<Vec<RationItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, ration_id, feed_id, amount_kg, is_locked, sort_order
         FROM ration_items
         WHERE ration_id = ?
         ORDER BY sort_order",
    )?;

    let items = stmt
        .query_map([ration_id], |row| {
            Ok(RationItem {
                id: row.get("id")?,
                ration_id: row.get("ration_id")?,
                feed_id: row.get("feed_id")?,
                feed: None,
                amount_kg: row.get("amount_kg")?,
                is_locked: row.get::<_, i32>("is_locked")? != 0,
                sort_order: row.get("sort_order")?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut items_with_feeds = Vec::with_capacity(items.len());
    for mut item in items {
        item.feed = super::feeds::get_feed(conn, item.feed_id)?;
        items_with_feeds.push(item);
    }

    Ok(items_with_feeds)
}

/// Create a new ration
pub fn create_ration(conn: &Connection, ration: &Ration) -> Result<i64> {
    conn.execute(
        "INSERT INTO rations (name, animal_group_id, animal_count, description, status)
         VALUES (?, ?, ?, ?, ?)",
        params![
            ration.name,
            ration.animal_group_id,
            ration.animal_count,
            ration.description,
            ration.status
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Update a ration
pub fn update_ration(conn: &Connection, id: i64, ration: &Ration) -> Result<()> {
    conn.execute(
        "UPDATE rations SET
            name = ?, animal_group_id = ?, animal_count = ?,
            description = ?, status = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
        params![
            ration.name,
            ration.animal_group_id,
            ration.animal_count,
            ration.description,
            ration.status,
            id
        ],
    )?;

    Ok(())
}

/// Add item to ration
pub fn add_ration_item(
    conn: &Connection,
    ration_id: i64,
    feed_id: i64,
    amount_kg: f64,
    is_locked: bool,
) -> Result<i64> {
    // Get max sort order
    let max_order: i32 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) FROM ration_items WHERE ration_id = ?",
        [ration_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO ration_items (ration_id, feed_id, amount_kg, is_locked, sort_order)
         VALUES (?, ?, ?, ?, ?)",
        params![
            ration_id,
            feed_id,
            amount_kg,
            is_locked as i32,
            max_order + 1
        ],
    )?;

    // Update ration timestamp
    conn.execute(
        "UPDATE rations SET updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        [ration_id],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Update ration item amount
pub fn update_ration_item(
    conn: &Connection,
    item_id: i64,
    amount_kg: f64,
    is_locked: bool,
) -> Result<()> {
    conn.execute(
        "UPDATE ration_items SET amount_kg = ?, is_locked = ? WHERE id = ?",
        params![amount_kg, is_locked as i32, item_id],
    )?;

    Ok(())
}

/// Remove item from ration
pub fn remove_ration_item(conn: &Connection, item_id: i64) -> Result<()> {
    conn.execute("DELETE FROM ration_items WHERE id = ?", [item_id])?;
    Ok(())
}

/// Delete a ration and all its items
pub fn delete_ration(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM rations WHERE id = ?", [id])?;
    Ok(())
}
