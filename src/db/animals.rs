//! Animal groups database operations

use anyhow::Result;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

/// Animal group model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalGroup {
    pub id: String,
    pub species: String,
    pub production_type: Option<String>,
    pub name_ru: String,
    pub name_en: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>,
}

impl AnimalGroup {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            species: row.get("species")?,
            production_type: row.get("production_type")?,
            name_ru: row.get("name_ru")?,
            name_en: row.get("name_en")?,
            description: row.get("description")?,
            created_at: row.get("created_at")?,
        })
    }
}

/// List all animal groups
pub fn list_animal_groups(conn: &Connection) -> Result<Vec<AnimalGroup>> {
    let mut stmt = conn.prepare("SELECT * FROM animal_groups ORDER BY species, name_ru")?;

    let groups = stmt
        .query_map([], AnimalGroup::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(groups)
}

/// List animal groups by species
pub fn list_by_species(conn: &Connection, species: &str) -> Result<Vec<AnimalGroup>> {
    let mut stmt =
        conn.prepare("SELECT * FROM animal_groups WHERE species = ? ORDER BY name_ru")?;

    let groups = stmt
        .query_map([species], AnimalGroup::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(groups)
}

/// Get animal group by ID
pub fn get_animal_group(conn: &Connection, id: &str) -> Result<Option<AnimalGroup>> {
    let group = conn
        .query_row(
            "SELECT * FROM animal_groups WHERE id = ?",
            [id],
            AnimalGroup::from_row,
        )
        .ok();

    Ok(group)
}

/// Create animal group
pub fn create_animal_group(conn: &Connection, group: &AnimalGroup) -> Result<()> {
    conn.execute(
        "INSERT INTO animal_groups (id, species, production_type, name_ru, name_en, description)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![
            group.id,
            group.species,
            group.production_type,
            group.name_ru,
            group.name_en,
            group.description
        ],
    )?;

    Ok(())
}

/// Update animal group
pub fn update_animal_group(conn: &Connection, id: &str, group: &AnimalGroup) -> Result<()> {
    conn.execute(
        "UPDATE animal_groups SET
            species = ?, production_type = ?, name_ru = ?, name_en = ?, description = ?
         WHERE id = ?",
        params![
            group.species,
            group.production_type,
            group.name_ru,
            group.name_en,
            group.description,
            id
        ],
    )?;

    Ok(())
}

/// Delete animal group
pub fn delete_animal_group(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM animal_groups WHERE id = ?", [id])?;
    Ok(())
}

/// Seed default animal groups
pub fn seed_default_groups(conn: &Connection) -> Result<()> {
    let groups = vec![
        // Dairy cattle
        AnimalGroup {
            id: "cattle_dairy_dry_early".to_string(),
            species: "cattle".to_string(),
            production_type: Some("dairy".to_string()),
            name_ru: "Сухостойные (1-25 дн.)".to_string(),
            name_en: Some("Dry early (1-25 d)".to_string()),
            description: None,
            created_at: None,
        },
        AnimalGroup {
            id: "cattle_dairy_fresh".to_string(),
            species: "cattle".to_string(),
            production_type: Some("dairy".to_string()),
            name_ru: "Раздой (0-60 дн. лактации)".to_string(),
            name_en: Some("Fresh (0-60 d lactation)".to_string()),
            description: None,
            created_at: None,
        },
        AnimalGroup {
            id: "cattle_dairy_early_lact".to_string(),
            species: "cattle".to_string(),
            production_type: Some("dairy".to_string()),
            name_ru: "Ранняя лактация (60-150 дн.)".to_string(),
            name_en: Some("Early lactation (60-150 d)".to_string()),
            description: None,
            created_at: None,
        },
        // Swine
        AnimalGroup {
            id: "swine_piglet_nursery".to_string(),
            species: "swine".to_string(),
            production_type: Some("fattening".to_string()),
            name_ru: "Поросята-отъёмыши (28-70 дн.)".to_string(),
            name_en: Some("Nursery piglets (28-70 d)".to_string()),
            description: None,
            created_at: None,
        },
        AnimalGroup {
            id: "swine_finisher".to_string(),
            species: "swine".to_string(),
            production_type: Some("fattening".to_string()),
            name_ru: "Откорм (120-170 дн.)".to_string(),
            name_en: Some("Finisher (120-170 d)".to_string()),
            description: None,
            created_at: None,
        },
        AnimalGroup {
            id: "swine_sow_lactating".to_string(),
            species: "swine".to_string(),
            production_type: Some("breeding".to_string()),
            name_ru: "Подсосные свиноматки".to_string(),
            name_en: Some("Lactating sows".to_string()),
            description: None,
            created_at: None,
        },
        // Poultry
        AnimalGroup {
            id: "poultry_broiler_starter".to_string(),
            species: "poultry".to_string(),
            production_type: Some("broiler".to_string()),
            name_ru: "Бройлер стартер (0-10 дн.)".to_string(),
            name_en: Some("Broiler starter (0-10 d)".to_string()),
            description: None,
            created_at: None,
        },
        AnimalGroup {
            id: "poultry_broiler_finisher".to_string(),
            species: "poultry".to_string(),
            production_type: Some("broiler".to_string()),
            name_ru: "Бройлер финишер (26-42 дн.)".to_string(),
            name_en: Some("Broiler finisher (26-42 d)".to_string()),
            description: None,
            created_at: None,
        },
        AnimalGroup {
            id: "poultry_layer_peak".to_string(),
            species: "poultry".to_string(),
            production_type: Some("layer".to_string()),
            name_ru: "Несушки, пик (до 40 нед.)".to_string(),
            name_en: Some("Layers, peak (to 40 wk)".to_string()),
            description: None,
            created_at: None,
        },
    ];

    for group in groups {
        // Skip if already exists
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM animal_groups WHERE id = ?",
            [&group.id],
            |row| row.get(0),
        )?;

        if !exists {
            create_animal_group(conn, &group)?;
        }
    }

    Ok(())
}
