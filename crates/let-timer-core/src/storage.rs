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
        let clean_dir = rel_dir.trim_start_matches('/');
        let p = std::path::Path::new(clean_dir);
        if clean_dir.is_empty()
            || clean_dir.contains('\0')
            || clean_dir.contains('\\')
            || p.components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(StorageError::InvalidPath(rel_dir.to_string()));
        }

        let target_dir = self.base_dir.join(clean_dir);
        std::fs::create_dir_all(&target_dir)?;

        let final_filename = Self::hashed_filename(filename, data);
        let dest_path = target_dir.join(&final_filename);
        std::fs::write(&dest_path, data)?;

        let rel_path = format!("{}/{}", clean_dir.trim_matches('/'), final_filename);
        Ok(self.public_url(&rel_path))
    }

    fn copy_local(&self, src_path: &Path, rel_dir: &str) -> Result<String, StorageError> {
        // 1. Reject non-regular files (symlinks, devices, FIFOs, directories)
        let sym_meta = std::fs::symlink_metadata(src_path)?;
        if !sym_meta.is_file() {
            return Err(StorageError::InvalidPath(format!(
                "source file must be a regular file (symlinks, devices, and directories rejected): {}",
                src_path.display()
            )));
        }

        // 2. Canonicalize first
        let canonical_src = src_path.canonicalize()?;
        if !canonical_src.is_file() {
            return Err(StorageError::InvalidPath(format!(
                "source file must be a regular file: {}",
                canonical_src.display()
            )));
        }

        // 3. Refuse paths inside daemon's own data directory except the media directory
        if let Ok(db_path) = crate::default_db_path()
            && let Some(data_dir) = db_path.parent()
            && let Ok(canonical_data_dir) = data_dir.canonicalize()
        {
            let in_data_dir = canonical_src.starts_with(&canonical_data_dir);
            let in_media_dir = self
                .base_dir
                .canonicalize()
                .map(|b| canonical_src.starts_with(&b))
                .unwrap_or(false);

            if in_data_dir && !in_media_dir {
                return Err(StorageError::InvalidPath(format!(
                    "cannot import files from daemon data directory: {}",
                    src_path.display()
                )));
            }
        }

        // 4. Cap size (default 200 MB, configurable via LET_TIMER_MAX_UPLOAD_MB)
        let max_mb = std::env::var("LET_TIMER_MAX_UPLOAD_MB")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(200);
        let max_bytes = max_mb * 1024 * 1024;

        let meta = std::fs::metadata(&canonical_src)?;
        if meta.len() > max_bytes {
            return Err(StorageError::InvalidPath(format!(
                "file exceeds maximum size limit ({} MB)",
                max_mb
            )));
        }

        let data = std::fs::read(&canonical_src)?;
        let file_name = canonical_src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("media.bin");

        self.put(rel_dir, file_name, &data)
    }

    fn delete(&self, rel_path: &str) -> Result<(), StorageError> {
        let full_path = match crate::safe_join(&self.base_dir, rel_path) {
            Some(p) => p,
            None => {
                let clean = rel_path.trim_start_matches('/');
                if clean.contains("..") || clean.contains('\0') || clean.contains('\\') {
                    return Err(StorageError::InvalidPath(rel_path.to_string()));
                }
                return Ok(());
            }
        };

        if full_path.is_file() {
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

    #[test]
    fn copy_local_rejects_symlink() {
        let temp_dir =
            std::env::temp_dir().join(format!("let_timer_storage_sym_{}", rand::random::<u32>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let storage = LocalStorageBackend::new(temp_dir.join("media"));

        let target_file = temp_dir.join("target.txt");
        std::fs::write(&target_file, b"content").unwrap();

        let symlink_file = temp_dir.join("symlink.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target_file, &symlink_file).unwrap();

        let res = storage.copy_local(&symlink_file, "images");
        assert!(matches!(res, Err(StorageError::InvalidPath(_))));

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn copy_local_rejects_directory() {
        let temp_dir =
            std::env::temp_dir().join(format!("let_timer_storage_dir_{}", rand::random::<u32>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let storage = LocalStorageBackend::new(temp_dir.join("media"));

        let sub_dir = temp_dir.join("somedir");
        std::fs::create_dir_all(&sub_dir).unwrap();

        let res = storage.copy_local(&sub_dir, "images");
        assert!(matches!(res, Err(StorageError::InvalidPath(_))));

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
