use base64::Engine;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use rand::RngExt;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

use super::error::DbError;
use super::models::{ApiToken, CreatedToken};

/// Repository for managing API tokens.
pub struct TokenRepository<'a> {
    conn: &'a Connection,
}

impl<'a> TokenRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Hash a raw token string with SHA-256 (hex-encoded).
    pub fn hash_token(raw_token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw_token.as_bytes());
        let result = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for byte in result {
            use std::fmt::Write;
            let _ = write!(&mut hex, "{:02x}", byte);
        }
        hex
    }

    /// Generate a new API token with format `lt_<32 base64url bytes>`.
    /// Stores the SHA-256 hash in the database and returns the plaintext token.
    pub fn create(&self, name: &str) -> Result<CreatedToken, DbError> {
        let mut random_bytes = [0u8; 32];
        rand::rng().fill(&mut random_bytes);
        let encoded = BASE64_URL_SAFE_NO_PAD.encode(random_bytes);
        let plaintext_token = format!("lt_{encoded}");
        let hash = Self::hash_token(&plaintext_token);

        self.conn.execute(
            "INSERT INTO api_tokens (name, token_hash) VALUES (?1, ?2)",
            params![name, hash],
        )?;

        let id = self.conn.last_insert_rowid();
        let created_at: String = self.conn.query_row(
            "SELECT created_at FROM api_tokens WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        Ok(CreatedToken {
            id,
            name: name.to_string(),
            token: plaintext_token,
            created_at,
        })
    }

    /// Create or ensure a token with a pre-defined raw token value (e.g. bootstrap token).
    pub fn create_with_raw_token(
        &self,
        name: &str,
        raw_token: &str,
    ) -> Result<CreatedToken, DbError> {
        let hash = Self::hash_token(raw_token);
        self.conn.execute(
            "INSERT INTO api_tokens (name, token_hash) VALUES (?1, ?2)
             ON CONFLICT(name) DO UPDATE SET token_hash = excluded.token_hash, revoked_at = NULL",
            params![name, hash],
        )?;

        let (id, created_at): (i64, String) = self.conn.query_row(
            "SELECT id, created_at FROM api_tokens WHERE name = ?1",
            params![name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok(CreatedToken {
            id,
            name: name.to_string(),
            token: raw_token.to_string(),
            created_at,
        })
    }

    /// Rotate an existing token by name. Replaces the hash with a new random secret.
    pub fn rotate(&self, name: &str) -> Result<CreatedToken, DbError> {
        let mut random_bytes = [0u8; 32];
        rand::rng().fill(&mut random_bytes);
        let encoded = BASE64_URL_SAFE_NO_PAD.encode(random_bytes);
        let plaintext_token = format!("lt_{encoded}");
        let hash = Self::hash_token(&plaintext_token);

        let rows = self.conn.execute(
            "UPDATE api_tokens SET token_hash = ?1, created_at = datetime('now'), last_used_at = NULL, revoked_at = NULL WHERE name = ?2",
            params![hash, name],
        )?;

        if rows == 0 {
            return Err(DbError::NotFound {
                entity: "api_token",
                id: 0,
            });
        }

        let (id, created_at): (i64, String) = self.conn.query_row(
            "SELECT id, created_at FROM api_tokens WHERE name = ?1",
            params![name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok(CreatedToken {
            id,
            name: name.to_string(),
            token: plaintext_token,
            created_at,
        })
    }

    /// Find an active (non-revoked) token by its raw plaintext value.
    /// If valid, updates its `last_used_at` timestamp.
    pub fn authenticate(&self, raw_token: &str) -> Result<Option<ApiToken>, DbError> {
        let hash = Self::hash_token(raw_token);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, token_hash, created_at, last_used_at, revoked_at
             FROM api_tokens
             WHERE token_hash = ?1 AND revoked_at IS NULL",
        )?;

        let mut rows = stmt.query(params![hash])?;
        if let Some(row) = rows.next()? {
            let token = ApiToken {
                id: row.get(0)?,
                name: row.get(1)?,
                token_hash: row.get(2)?,
                created_at: row.get(3)?,
                last_used_at: row.get(4)?,
                revoked_at: row.get(5)?,
            };

            // Touch last_used_at
            let _ = self.conn.execute(
                "UPDATE api_tokens SET last_used_at = datetime('now') WHERE id = ?1",
                params![token.id],
            );

            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    /// List all tokens (with hashes, never plaintext).
    pub fn list_all(&self) -> Result<Vec<ApiToken>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, token_hash, created_at, last_used_at, revoked_at
             FROM api_tokens
             ORDER BY id ASC",
        )?;

        let tokens = stmt
            .query_map([], |row| {
                Ok(ApiToken {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    token_hash: row.get(2)?,
                    created_at: row.get(3)?,
                    last_used_at: row.get(4)?,
                    revoked_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tokens)
    }

    /// Revoke a token by name.
    pub fn revoke(&self, name: &str) -> Result<bool, DbError> {
        let rows = self.conn.execute(
            "UPDATE api_tokens SET revoked_at = datetime('now') WHERE name = ?1 AND revoked_at IS NULL",
            params![name],
        )?;
        Ok(rows > 0)
    }

    /// Delete a token permanently by name.
    pub fn delete(&self, name: &str) -> Result<bool, DbError> {
        let rows = self
            .conn
            .execute("DELETE FROM api_tokens WHERE name = ?1", params![name])?;
        Ok(rows > 0)
    }

    /// Check if any active tokens exist.
    pub fn has_active_tokens(&self) -> Result<bool, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM api_tokens WHERE revoked_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[test]
    fn create_authenticate_and_revoke_token() {
        let db = Database::open_in_memory().unwrap();
        let repo = TokenRepository::new(db.conn());

        assert!(!repo.has_active_tokens().unwrap());

        // Create token
        let created = repo.create("cli-laptop").unwrap();
        assert!(created.token.starts_with("lt_"));
        assert_eq!(created.name, "cli-laptop");

        assert!(repo.has_active_tokens().unwrap());

        // Authenticate with correct token
        let authed = repo.authenticate(&created.token).unwrap();
        assert!(authed.is_some());
        let token = authed.unwrap();
        assert_eq!(token.name, "cli-laptop");
        assert!(token.revoked_at.is_none());

        // Authenticate with invalid token
        let bad = repo.authenticate("lt_invalid").unwrap();
        assert!(bad.is_none());

        // Revoke token
        let revoked = repo.revoke("cli-laptop").unwrap();
        assert!(revoked);

        // Authenticate with revoked token fails
        let authed_after = repo.authenticate(&created.token).unwrap();
        assert!(authed_after.is_none());

        assert!(!repo.has_active_tokens().unwrap());
    }

    #[test]
    fn rotate_and_bootstrap_tokens() {
        let db = Database::open_in_memory().unwrap();
        let repo = TokenRepository::new(db.conn());

        // Bootstrap
        let boot = repo
            .create_with_raw_token("bootstrap", "lt_secret_admin_token")
            .unwrap();
        assert_eq!(boot.name, "bootstrap");
        assert_eq!(boot.token, "lt_secret_admin_token");
        assert!(
            repo.authenticate("lt_secret_admin_token")
                .unwrap()
                .is_some()
        );

        // Rotate
        let rotated = repo.rotate("bootstrap").unwrap();
        assert_ne!(rotated.token, "lt_secret_admin_token");
        assert!(
            repo.authenticate("lt_secret_admin_token")
                .unwrap()
                .is_none()
        );
        assert!(repo.authenticate(&rotated.token).unwrap().is_some());
    }
}
