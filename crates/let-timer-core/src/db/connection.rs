use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

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
                             'paused',
                             'completed',
                             'cancelled'
                            )),
    estimated_mins  INTEGER,
    scheduled_on    TEXT,
    run_no          INTEGER NOT NULL DEFAULT 0,
    notified_at     TEXT,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (media_list_id) REFERENCES media_list(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_scheduled_on ON tasks (scheduled_on);

CREATE TABLE IF NOT EXISTS task_sessions (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id    INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    run_no     INTEGER NOT NULL,
    started_at TEXT    NOT NULL,
    ended_at   TEXT,
    end_reason TEXT CHECK (end_reason IN
               ('paused','stopped','completed','cancelled','restarted'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_task ON task_sessions (task_id, run_no);

CREATE UNIQUE INDEX IF NOT EXISTS idx_one_open_session
    ON task_sessions ((1)) WHERE ended_at IS NULL;

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS api_tokens (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL UNIQUE,
    token_hash   TEXT NOT NULL UNIQUE,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT,
    revoked_at   TEXT
);
";

/// Current schema version. Bump this whenever `migrate` needs a new step.
const SCHEMA_VERSION: i64 = 3;

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
        let path = default_db_path()?;
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
        self.migrate()?;
        self.ensure_default_workspace_and_settings()?;
        Ok(())
    }

    /// Bring an existing database file up to `SCHEMA_VERSION`.
    fn migrate(&self) -> Result<(), DbError> {
        let current: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if current < 2 {
            if current < 1 {
                // v1: `tasks.scheduled_on` (date only, ISO "YYYY-MM-DD").
                self.add_column_if_missing("tasks", "scheduled_on", "TEXT")?;
                self.conn.execute_batch(
                    "CREATE INDEX IF NOT EXISTS idx_tasks_scheduled_on ON tasks (scheduled_on);",
                )?;
            }

            // v2: Rebuild `tasks` table if `run_no` column is missing (handles legacy v0/v1 databases)
            if !self.has_column("tasks", "run_no")? {
                self.conn.execute_batch("PRAGMA foreign_keys = OFF;")?;

                self.conn.execute_batch(
                    "BEGIN;
                     CREATE TABLE tasks_new (
                         id              INTEGER PRIMARY KEY AUTOINCREMENT,
                         workspace_id    INTEGER NOT NULL,
                         media_list_id   INTEGER,
                         name            TEXT    NOT NULL,
                         description     TEXT,
                         priority        INTEGER NOT NULL DEFAULT 0
                                         CHECK (priority IN (0, 1, 2)),
                         status          TEXT    NOT NULL DEFAULT 'pending'
                                         CHECK (status IN (
                                                  'pending',
                                                  'in-progress',
                                                  'paused',
                                                  'completed',
                                                  'cancelled'
                                                 )),
                         estimated_mins  INTEGER,
                         scheduled_on    TEXT,
                         run_no          INTEGER NOT NULL DEFAULT 0,
                         notified_at     TEXT,
                         created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
                         updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),

                         FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
                         FOREIGN KEY (media_list_id) REFERENCES media_list(id) ON DELETE SET NULL
                     );

                     INSERT INTO tasks_new (
                         id, workspace_id, media_list_id, name, description,
                         priority, status, estimated_mins, scheduled_on,
                         run_no, notified_at, created_at, updated_at
                     )
                     SELECT
                         id, workspace_id, media_list_id, name, description,
                         priority, status, estimated_mins, scheduled_on,
                         0, NULL, created_at, updated_at
                     FROM tasks;

                     DROP TABLE tasks;
                     ALTER TABLE tasks_new RENAME TO tasks;
                     CREATE INDEX IF NOT EXISTS idx_tasks_scheduled_on ON tasks (scheduled_on);

                     CREATE TABLE IF NOT EXISTS task_sessions (
                         id         INTEGER PRIMARY KEY AUTOINCREMENT,
                         task_id    INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                         run_no     INTEGER NOT NULL,
                         started_at TEXT    NOT NULL,
                         ended_at   TEXT,
                         end_reason TEXT CHECK (end_reason IN
                                    ('paused','stopped','completed','cancelled','restarted'))
                     );
                     CREATE INDEX IF NOT EXISTS idx_sessions_task ON task_sessions (task_id, run_no);
                     CREATE UNIQUE INDEX IF NOT EXISTS idx_one_open_session
                         ON task_sessions ((1)) WHERE ended_at IS NULL;

                     CREATE TABLE IF NOT EXISTS settings (
                         key   TEXT PRIMARY KEY,
                         value TEXT NOT NULL
                     );
                     COMMIT;"
                )?;

                self.conn.execute_batch("PRAGMA foreign_keys = ON;")?;

                let fk_errors: i64 = self.conn.query_row(
                    "SELECT COUNT(*) FROM pragma_foreign_key_check()",
                    [],
                    |row| row.get(0),
                )?;
                if fk_errors > 0 {
                    return Err(DbError::Sqlite(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CORRUPT),
                        Some(format!("foreign key check failed: {fk_errors} violations")),
                    )));
                }
            }
        }

        if current < 3 {
            // v3: api_tokens table for HTTP API authentication
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS api_tokens (
                    id           INTEGER PRIMARY KEY AUTOINCREMENT,
                    name         TEXT NOT NULL UNIQUE,
                    token_hash   TEXT NOT NULL UNIQUE,
                    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
                    last_used_at TEXT,
                    revoked_at   TEXT
                );",
            )?;
        }

        self.conn
            .execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
        Ok(())
    }

    /// Check if a table has a specific column.
    fn has_column(&self, table: &str, column: &str) -> Result<bool, DbError> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let existing: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(existing.iter().any(|name| name == column))
    }

    /// Add a column to a table unless the table already has it.
    fn add_column_if_missing(
        &self,
        table: &str,
        column: &str,
        declaration: &str,
    ) -> Result<(), DbError> {
        if self.has_column(table, column)? {
            return Ok(());
        }

        self.conn.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
        ))?;
        Ok(())
    }

    /// Ensure at least one Default workspace exists and settings.active_workspace_id is set.
    fn ensure_default_workspace_and_settings(&self) -> Result<(), DbError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM workspaces", [], |row| row.get(0))?;
        if count == 0 {
            self.conn.execute(
                "INSERT INTO workspaces (name, description) VALUES ('Default', 'Default workspace')",
                [],
            )?;
        }

        let first_ws_id: i64 = self.conn.query_row(
            "SELECT id FROM workspaces ORDER BY id ASC LIMIT 1",
            [],
            |row| row.get(0),
        )?;

        let active_ws_valid: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM settings s
                    JOIN workspaces w ON s.value = CAST(w.id AS TEXT)
                    WHERE s.key = 'active_workspace_id'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !active_ws_valid {
            self.conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('active_workspace_id', ?1)",
                params![first_ws_id.to_string()],
            )?;
        }

        Ok(())
    }

    /// Borrow the underlying connection (for use by repositories).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// Resolve the default database path: `~/.local/share/let-timer/let-timer.db`.
///
/// Can be overridden with the `LET_TIMER_DB` environment variable.
pub fn default_db_path() -> Result<PathBuf, DbError> {
    if let Some(override_path) = std::env::var_os("LET_TIMER_DB")
        && !override_path.is_empty()
    {
        return Ok(PathBuf::from(override_path));
    }
    if let Some(base) = std::env::var_os("XDG_DATA_HOME")
        && !base.is_empty()
    {
        return Ok(PathBuf::from(base).join("let-timer").join("let-timer.db"));
    }
    if let Some(home) = std::env::var_os("HOME")
        && !home.is_empty()
    {
        return Ok(PathBuf::from(home)
            .join(".local/share")
            .join("let-timer")
            .join("let-timer.db"));
    }
    Err(DbError::Configuration(
        "neither LET_TIMER_DB, XDG_DATA_HOME nor HOME is set".to_string(),
    ))
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

        // Verify Default workspace and active_workspace_id setting exist
        let ws_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM workspaces WHERE name = 'Default'",
                [],
                |row| row.get(0),
            )
            .expect("default workspace should exist");
        assert_eq!(ws_count, 1);

        let active_ws: String = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = 'active_workspace_id'",
                [],
                |row| row.get(0),
            )
            .expect("active_workspace_id should exist");
        assert!(!active_ws.is_empty());
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn default_db_path_is_reasonable() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("LET_TIMER_DB") };
        let path = default_db_path().expect("default db path should resolve");
        assert!(path.ends_with("let-timer/let-timer.db"));
    }

    #[test]
    fn default_db_path_respects_let_timer_db_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("LET_TIMER_DB", "/custom/test/db.sqlite") };
        let path = default_db_path().expect("should resolve with env override");
        assert_eq!(path, PathBuf::from("/custom/test/db.sqlite"));
        unsafe { std::env::remove_var("LET_TIMER_DB") };
    }

    #[test]
    fn migration_v1_to_v2_on_file_db() {
        let dir = std::env::temp_dir().join(format!(
            "let-timer-migration-v1-to-v2-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("v1.db");

        // 1. Seed a legacy v1 database (with user_version = 1)
        {
            let conn = Connection::open(&path).expect("open raw v1 db");
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;
                 CREATE TABLE workspaces (
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
                     priority        INTEGER NOT NULL DEFAULT 0 CHECK (priority IN (0, 1, 2)),
                     status          TEXT NOT NULL DEFAULT 'pending'
                                     CHECK (status IN ('pending', 'in-progress', 'completed', 'cancelled')),
                     estimated_mins  INTEGER,
                     scheduled_on    TEXT,
                     created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                     updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
                     FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
                     FOREIGN KEY (media_list_id) REFERENCES media_list(id) ON DELETE SET NULL
                 );
                 CREATE INDEX idx_tasks_scheduled_on ON tasks (scheduled_on);
                 PRAGMA user_version = 1;

                 INSERT INTO workspaces (id, name) VALUES (1, 'Study WS');
                 INSERT INTO tasks (id, workspace_id, name, status, priority, estimated_mins, scheduled_on)
                     VALUES (1, 1, 'Task pending', 'pending', 0, 15, '2026-10-15');
                 INSERT INTO tasks (id, workspace_id, name, status, priority, estimated_mins, scheduled_on)
                     VALUES (2, 1, 'Task in-progress', 'in-progress', 1, 30, '2026-10-15');
                 INSERT INTO tasks (id, workspace_id, name, status, priority, estimated_mins, scheduled_on)
                     VALUES (3, 1, 'Task completed', 'completed', 2, NULL, NULL);
                 INSERT INTO tasks (id, workspace_id, name, status, priority, estimated_mins, scheduled_on)
                     VALUES (4, 1, 'Task cancelled', 'cancelled', 0, NULL, NULL);",
            )
            .expect("seed v1 db");
        }

        // 2. Open with Database::open to trigger v2 migration
        let db = Database::open(&path).expect("open and migrate v1 db");

        // Verify rows preserved
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 4);

        let t1_run_no: i64 = db
            .conn()
            .query_row("SELECT run_no FROM tasks WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(t1_run_no, 0);

        let t1_notified: Option<String> = db
            .conn()
            .query_row("SELECT notified_at FROM tasks WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(t1_notified, None);

        // Verify paused status can now be inserted
        let inserted = db.conn().execute(
            "INSERT INTO tasks (workspace_id, name, status) VALUES (1, 'Task paused', 'paused')",
            [],
        );
        assert!(
            inserted.is_ok(),
            "inserting paused status should succeed after v2 migration"
        );

        // Verify foreign key integrity
        let fk_errors: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check()", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(fk_errors, 0, "foreign keys must remain valid");

        // Verify task_sessions and settings exist
        let sessions_count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM task_sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sessions_count, 0);

        let active_ws: String = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = 'active_workspace_id'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(active_ws, "1");

        // Verify user_version is SCHEMA_VERSION (3)
        let version: i64 = db
            .conn()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fresh_db_vs_migrated_db_table_info_equal() {
        let dir = std::env::temp_dir().join(format!("let-timer-compare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");

        let fresh_path = dir.join("fresh.db");
        let migrated_path = dir.join("migrated.db");

        // Seed legacy v1 in migrated_path
        {
            let conn = Connection::open(&migrated_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE workspaces (id INTEGER PRIMARY KEY, name TEXT NOT NULL, description TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')));
                 CREATE TABLE media_list (id INTEGER PRIMARY KEY, name TEXT NOT NULL, description TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')));
                 CREATE TABLE tasks (id INTEGER PRIMARY KEY, workspace_id INTEGER NOT NULL REFERENCES workspaces(id), media_list_id INTEGER REFERENCES media_list(id), name TEXT NOT NULL, description TEXT, priority INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'pending', estimated_mins INTEGER, scheduled_on TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')));
                 PRAGMA user_version = 1;",
            ).unwrap();
        }

        let fresh_db = Database::open(&fresh_path).unwrap();
        let migrated_db = Database::open(&migrated_path).unwrap();

        for table in [
            "tasks",
            "task_sessions",
            "settings",
            "workspaces",
            "api_tokens",
        ] {
            let get_info = |db: &Database| -> Vec<(i64, String, String, i64, Option<String>, i64)> {
                let mut stmt = db
                    .conn()
                    .prepare(&format!("PRAGMA table_info('{table}')"))
                    .unwrap();
                stmt.query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
            };

            let fresh_info = get_info(&fresh_db);
            let migrated_info = get_info(&migrated_db);
            assert_eq!(fresh_info, migrated_info, "table_info mismatch on {table}");
        }

        drop(fresh_db);
        drop(migrated_db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn task_sessions_unique_open_session_enforced() {
        let db = Database::open_in_memory().unwrap();

        // Workspace id 1 already exists (Default workspace). Create a task.
        db.conn()
            .execute(
                "INSERT INTO tasks (id, workspace_id, name) VALUES (1, 1, 'Task')",
                [],
            )
            .unwrap();

        // 1. Insert open session
        db.conn()
            .execute(
                "INSERT INTO task_sessions (task_id, run_no, started_at, ended_at)
             VALUES (1, 0, '2026-10-12T10:00:00Z', NULL)",
                [],
            )
            .unwrap();

        // 2. Inserting a second open session must fail (unique partial index violation)
        let second_open = db.conn().execute(
            "INSERT INTO task_sessions (task_id, run_no, started_at, ended_at)
             VALUES (1, 0, '2026-10-12T10:05:00Z', NULL)",
            [],
        );
        assert!(
            second_open.is_err(),
            "cannot have two open sessions at the same time"
        );

        // 3. Inserting a closed session succeeds
        let closed = db.conn().execute(
            "INSERT INTO task_sessions (task_id, run_no, started_at, ended_at, end_reason)
             VALUES (1, 0, '2026-10-12T09:00:00Z', '2026-10-12T09:30:00Z', 'paused')",
            [],
        );
        assert!(
            closed.is_ok(),
            "closed sessions do not violate the open session index"
        );
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
                    "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name = 'run_no'",
                    [],
                    |row| row.get(0),
                )
                .expect("run_no should exist");
            assert_eq!(count, 1);
            let version: i64 = db
                .conn()
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            assert_eq!(version, SCHEMA_VERSION);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
