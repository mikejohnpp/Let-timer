//! Storage backend abstraction for media assets (local disk & future cloud backends).

use crate::db::DbError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use std::fmt;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Db(DbError),
    InvalidPath(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "io error: {e}"),
            StorageError::Db(e) => write!(f, "database error: {e}"),
            StorageError::InvalidPath(p) => write!(f, "invalid media path: {p}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::Io(e) => Some(e),
            StorageError::Db(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::Io(err)
    }
}

impl From<DbError> for StorageError {
    fn from(err: DbError) -> Self {
        StorageError::Db(err)
    }
}

/// Abstract storage backend for media files (images, audio).
pub trait StorageBackend: Send + Sync {
    /// Save raw bytes to storage and return the public access URL / relative endpoint.
    fn put(&self, rel_dir: &str, filename: &str, data: &[u8]) -> Result<String, StorageError>;

    /// Fast local copy from source path on disk into the managed media storage.
    fn copy_local(&self, src_path: &Path, rel_dir: &str) -> Result<String, StorageError>;

    /// Delete file from storage if it exists (relative to media root).
    fn delete(&self, rel_path: &str) -> Result<(), StorageError>;

    /// Convert a stored relative path or key to a public URL.
    fn public_url(&self, rel_path: &str) -> String;
}

/// Default filesystem-based storage backend.
#[derive(Debug, Clone)]
pub struct LocalStorageBackend {
    base_dir: PathBuf,
}

impl LocalStorageBackend {
    /// Create a new LocalStorageBackend with the given root directory.
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Open or create default storage at standard location (`~/.local/share/let-timer/media`).
    pub fn default_storage() -> Result<Self, StorageError> {
        let dir = crate::default_media_dir()?;
        std::fs::create_dir_all(&dir)?;
        Ok(Self::new(dir))
    }

    /// Sanitize filename and generate unique name with content hash prefix.
    fn hashed_filename(filename: &str, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let hash_hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
        let short_hash = &hash_hex[..12];

        let sanitized: String = filename
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        format!("{}_{}", short_hash, sanitized)
    }

    /// Extract subpath from public URL (e.g. `/api/v1/media/files/images/xyz.jpg` -> `images/xyz.jpg`).
    pub fn rel_path_from_url(url: &str) -> Option<&str> {
        let prefix = "/api/v1/media/files/";
        if let Some(pos) = url.find(prefix) {
            Some(&url[pos + prefix.len()..])
        } else {
            None
        }
    }
}

impl StorageBackend for LocalStorageBackend {
    fn put(&self, rel_dir: &str, filename: &str, data: &[u8]) -> Result<String, StorageError> {
        let target_dir = self.base_dir.join(rel_dir);
        std::fs::create_dir_all(&target_dir)?;

        let final_filename = Self::hashed_filename(filename, data);
        let dest_path = target_dir.join(&final_filename);
        std::fs::write(&dest_path, data)?;

        let rel_path = format!("{}/{}", rel_dir.trim_matches('/'), final_filename);
        Ok(self.public_url(&rel_path))
    }

    fn copy_local(&self, src_path: &Path, rel_dir: &str) -> Result<String, StorageError> {
        if !src_path.exists() {
            return Err(StorageError::InvalidPath(format!(
                "source file does not exist: {}",
                src_path.display()
            )));
        }

        let data = std::fs::read(src_path)?;
        let file_name = src_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("media.bin");

        self.put(rel_dir, file_name, &data)
    }

    fn delete(&self, rel_path: &str) -> Result<(), StorageError> {
        // Prevent path traversal
        let clean_rel = rel_path.trim_start_matches('/');
        if clean_rel.contains("..") {
            return Err(StorageError::InvalidPath(rel_path.to_string()));
        }

        let full_path = self.base_dir.join(clean_rel);
        if full_path.exists() && full_path.is_file() {
            let _ = std::fs::remove_file(full_path);
        }
        Ok(())
    }

    fn public_url(&self, rel_path: &str) -> String {
        format!("/api/v1/media/files/{}", rel_path.trim_start_matches('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_storage_put_and_delete() {
        let temp_dir =
            std::env::temp_dir().join(format!("let_timer_storage_test_{}", rand::random::<u32>()));
        let storage = LocalStorageBackend::new(temp_dir.clone());

        let url = storage
            .put("images", "avatar.png", b"test content")
            .unwrap();
        assert!(url.starts_with("/api/v1/media/files/images/"));
        assert!(url.ends_with("avatar.png"));

        let rel = LocalStorageBackend::rel_path_from_url(&url).unwrap();
        assert!(temp_dir.join(rel).exists());

        storage.delete(rel).unwrap();
        assert!(!temp_dir.join(rel).exists());

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
