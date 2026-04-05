//! Database module - SQLite connection and schema management

pub mod animals;
pub mod feed_labels;
pub mod feed_quality;
pub mod feed_source_profiles;
pub mod feeds;
pub mod prices;
pub mod rations;
pub mod search;
pub mod schema;

use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

/// Database wrapper with thread-safe connection
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create a new database connection
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Run all migrations
    pub fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        schema::run_migrations(&conn)
    }

    /// Get a reference to the connection (for operations)
    pub fn with_conn<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.conn.lock().unwrap();
        f(&conn)
    }

    /// Execute with mutable connection
    pub fn with_conn_mut<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Connection) -> Result<T>,
    {
        let mut conn = self.conn.lock().unwrap();
        f(&mut conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::new(":memory:").unwrap();
        db.run_migrations().unwrap();
    }
}
