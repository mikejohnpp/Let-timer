use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::error::DbError;

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS workspaces (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS medias (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT,
    url             TEXT    NOT NULL,
    type            TEXT    NOT NULL
                    CHECK (type in (
                        'image',
                        'music',
                        'poem'
                    )),
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS media_list (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS media_media_list (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id        INTEGER NOT NULL,
    media_list_id   INTEGER NOT NULL,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),

    UNIQUE (media_id, media_list_id),
    FOREIGN KEY (media_id) REFERENCES medias(id) ON DELETE CASCADE,
    FOREIGN KEY (media_list_id) REFERENCES media_list(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tasks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id    INTEGER NOT NULL,
    media_list_id   INTEGER,
    name            TEXT    NOT NULL,
    description     TEXT,
    priority        INTEGER NOT NULL DEFAULT 0
                    CHECK (priority IN (
                            0,
                            1,
                            2
                            )),
    status          TEXT    NOT NULL DEFAULT 'pending'
                    CHECK (status IN (
                             'pending',
                             'in-progress',
                             'completed',
                             'cancelled'
                            )),
    estimated_mins  INTEGER,
    scheduled_on    TEXT,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (media_list_id) REFERENCES media_list(id) ON DELETE SET NULL
);
";

/// Current schema version. Bump this whenever `migrate` needs a new step.
const SCHEMA_VERSION: i64 = 1;

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
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        self.conn.execute_batch(SCHEMA_SQL)?;
        self.migrate()?;
        Ok(())
    }

    /// Bring an existing database file up to `SCHEMA_VERSION`.
    ///
    /// `SCHEMA_SQL` uses `CREATE TABLE IF NOT EXISTS`, so tables that already
    /// exist are left untouched; columns introduced by later versions have to
    /// be added explicitly here.
    fn migrate(&self) -> Result<(), DbError> {
        let current: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if current < 1 {
            // v1: `tasks.scheduled_on` (date only, ISO "YYYY-MM-DD").
            self.add_column_if_missing("tasks", "scheduled_on", "TEXT")?;
            self.conn.execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_tasks_scheduled_on ON tasks (scheduled_on);",
            )?;
        }

        self.conn
            .execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
        Ok(())
    }

    /// Add a column to a table unless the table already has it.
    fn add_column_if_missing(
        &self,
        table: &str,
        column: &str,
        declaration: &str,
    ) -> Result<(), DbError> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let existing: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        if existing.iter().any(|name| name == column) {
            return Ok(());
        }

        self.conn.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
        ))?;
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

    #[test]
    fn migration_adds_scheduled_on_to_legacy_database() {
        let dir = std::env::temp_dir().join(format!("let-timer-migration-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("legacy.db");

        // Simulate a database created before `scheduled_on` existed.
        {
            let conn = Connection::open(&path).expect("open legacy db");
            conn.execute_batch(
                "CREATE TABLE workspaces (
                     id         INTEGER PRIMARY KEY AUTOINCREMENT,
                     name       TEXT NOT NULL,
                     description TEXT,
                     created_at TEXT NOT NULL DEFAULT (datetime('now')),
                     updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                 );
                 CREATE TABLE media_list (
                     id         INTEGER PRIMARY KEY AUTOINCREMENT,
                     name       TEXT NOT NULL,
                     description TEXT,
                     created_at TEXT NOT NULL DEFAULT (datetime('now')),
                     updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                 );
                 CREATE TABLE tasks (
                     id              INTEGER PRIMARY KEY AUTOINCREMENT,
                     workspace_id    INTEGER NOT NULL,
                     media_list_id   INTEGER,
                     name            TEXT NOT NULL,
                     description     TEXT,
                     priority        INTEGER NOT NULL DEFAULT 0,
                     status          TEXT NOT NULL DEFAULT 'pending',
                     estimated_mins  INTEGER,
                     created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                     updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
                     FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
                     FOREIGN KEY (media_list_id) REFERENCES media_list(id) ON DELETE SET NULL
                 );
                 INSERT INTO workspaces (id, name) VALUES (1, 'Legacy WS');
                 INSERT INTO tasks (id, workspace_id, name)
                     VALUES (1, 1, 'Legacy task');",
            )
            .expect("seed legacy schema");
        }

        // Opening it must run the migration.
        let db = Database::open(&path).expect("open and migrate legacy db");
        let column: Option<String> = db
            .conn()
            .query_row("SELECT scheduled_on FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("scheduled_on should exist after migration");
        assert_eq!(column, None, "existing rows should migrate to NULL");

        let version: i64 = db
            .conn()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read user_version");
        assert_eq!(version, SCHEMA_VERSION);

        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_is_idempotent() {
        let dir =
            std::env::temp_dir().join(format!("let-timer-migration-twice-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("twice.db");

        for _ in 0..2 {
            let db = Database::open(&path).expect("open db");
            let count: i64 = db
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name = 'scheduled_on'",
                    [],
                    |row| row.get(0),
                )
                .expect("scheduled_on should exist");
            assert_eq!(count, 1);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
