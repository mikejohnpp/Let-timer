use rusqlite::{Connection, params};

use super::error::DbError;

/// Repository for key-value configuration stored in the `settings` table.
pub struct SettingsRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SettingsRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Retrieve a setting value by key.
    pub fn get(&self, key: &str) -> Result<Option<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Insert or update a setting value.
    pub fn set(&self, key: &str, value: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get the active workspace ID, falling back to the first available workspace.
    pub fn active_workspace_id(&self) -> Result<i64, DbError> {
        if let Some(val) = self.get("active_workspace_id")?
            && let Ok(id) = val.parse::<i64>()
        {
            // Verify that this workspace actually exists
            let exists: bool = self
                .conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = ?1)",
                    params![id],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if exists {
                return Ok(id);
            }
        }

        // Fallback to first available workspace
        let fallback_id: i64 = self
            .conn
            .query_row(
                "SELECT id FROM workspaces ORDER BY id ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entity: "workspace",
                    id: 0,
                },
                other => DbError::Sqlite(other),
            })?;

        // Update settings cache with valid fallback
        let _ = self.set("active_workspace_id", &fallback_id.to_string());
        Ok(fallback_id)
    }

    /// Set the active workspace ID.
    pub fn set_active_workspace_id(&self, id: i64) -> Result<(), DbError> {
        // Ensure workspace exists
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = ?1)",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !exists {
            return Err(DbError::NotFound {
                entity: "workspace",
                id,
            });
        }

        self.set("active_workspace_id", &id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;

    #[test]
    fn get_and_set_settings() {
        let db = Database::open_in_memory().unwrap();
        let repo = SettingsRepository::new(db.conn());

        assert!(repo.get("nonexistent").unwrap().is_none());
        repo.set("foo", "bar").unwrap();
        assert_eq!(repo.get("foo").unwrap().as_deref(), Some("bar"));
        repo.set("foo", "baz").unwrap();
        assert_eq!(repo.get("foo").unwrap().as_deref(), Some("baz"));
    }

    #[test]
    fn active_workspace_id_resolves_and_updates() {
        let db = Database::open_in_memory().unwrap();
        let repo = SettingsRepository::new(db.conn());

        let active_id = repo.active_workspace_id().unwrap();
        assert_eq!(active_id, 1);

        // Setting a nonexistent workspace errors
        assert!(repo.set_active_workspace_id(9999).is_err());
    }
}
