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

    /// Get the configured timezone name (defaulting to "UTC").
    pub fn get_timezone(&self) -> Result<String, DbError> {
        let tz = self.get("timezone")?.unwrap_or_else(|| "UTC".to_string());
        Ok(tz)
    }

    /// Set and validate the configured timezone.
    pub fn set_timezone(&self, tz: &str) -> Result<(), DbError> {
        if tz.parse::<chrono_tz::Tz>().is_err() {
            return Err(DbError::Configuration(format!(
                "invalid IANA timezone: {tz}"
            )));
        }
        self.set("timezone", tz)
    }

    /// Get or generate a 32-byte hexadecimal secret for signing media URLs.
    pub fn get_or_create_media_signing_secret(&self) -> Result<String, DbError> {
        if let Some(val) = self.get("media_signing_secret")? {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }

        let bytes: [u8; 32] = rand::random();
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        self.set("media_signing_secret", &hex)?;
        Ok(hex)
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

    #[test]
    fn timezone_setting_get_and_set_validation() {
        let db = Database::open_in_memory().unwrap();
        let repo = SettingsRepository::new(db.conn());

        assert_eq!(repo.get_timezone().unwrap(), "UTC");
        assert!(repo.set_timezone("Asia/Ho_Chi_Minh").is_ok());
        assert_eq!(repo.get_timezone().unwrap(), "Asia/Ho_Chi_Minh");
        assert!(repo.set_timezone("Invalid/Timezone").is_err());
    }

    #[test]
    fn media_signing_secret_generation_and_persistence() {
        let db = Database::open_in_memory().unwrap();
        let repo = SettingsRepository::new(db.conn());

        let secret1 = repo.get_or_create_media_signing_secret().unwrap();
        assert_eq!(secret1.len(), 64);

        let secret2 = repo.get_or_create_media_signing_secret().unwrap();
        assert_eq!(secret1, secret2);
    }
}
