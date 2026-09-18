use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::error::DbError;

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS tasks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT,
    priority        INTEGER NOT NULL DEFAULT 0,
    status          TEXT    NOT NULL DEFAULT 'pending',
    estimated_mins  INTEGER,
    elapsed_secs    INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tasks_status   ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
";

/// Wrapper around a `rusqlite::Connection` that handles opening the database
/// file, creating the parent directory, running schema migrations, and
/// configuring recommended PRAGMAs.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the database at the default XDG data location:
    /// `~/.local/share/let-timer/let-timer.db`
    pub fn open_default() -> Result<Self, DbError> {
        let path = default_db_path();
        Self::open(&path)
    }

    /// Open (or create) the database at a custom path.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        // Ensure the parent directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DbError::Sqlite(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(format!("cannot create directory {}: {e}", parent.display())),
                ))
            })?;
        }

        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Open an in-memory database (useful for unit tests).
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Run one-time schema setup.
    fn initialize(&self) -> Result<(), DbError> {
        // Performance & safety PRAGMAs.
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        self.conn.execute_batch(SCHEMA_SQL)?;
        Ok(())
    }

    /// Borrow the underlying connection (for use by repositories).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// Resolve the default database path: `~/.local/share/let-timer/let-timer.db`.
fn default_db_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME env var must be set");
            PathBuf::from(home).join(".local/share")
        });
    base.join("let-timer").join("let-timer.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_creates_tables() {
        let db = Database::open_in_memory().expect("should open in-memory db");
        // Verify that the tasks table exists by querying it.
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .expect("tasks table should exist");
        assert_eq!(count, 0);
    }

    #[test]
    fn default_db_path_is_reasonable() {
        let path = default_db_path();
        assert!(path.ends_with("let-timer/let-timer.db"));
    }
}
