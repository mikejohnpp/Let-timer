//! Filesystem security utilities for safe path traversal prevention.

use std::path::{Component, Path, PathBuf};

/// Safely join a relative path to a root directory.
///
/// Prevents directory traversal attacks (`..`), absolute paths, control characters,
/// and symlinks that resolve outside of `root`.
///
/// Returns `Some(PathBuf)` containing the canonicalized path if and only if
/// the target path exists and resides strictly within `root`.
pub fn safe_join(root: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.trim_start_matches('/');
    if rel.is_empty() || rel.contains('\0') || rel.contains('\\') {
        return None;
    }
    let p = Path::new(rel);
    // Reject any component that is not a normal path segment (e.g., .., ., prefix, root)
    if p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let full = root.join(p).canonicalize().ok()?; // resolves symlinks
    full.starts_with(&root).then_some(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_allows_normal_nested_files() {
        let temp_dir = std::env::temp_dir().join(format!(
            "let_timer_safe_join_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sub = temp_dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let file = sub.join("hello.txt");
        std::fs::write(&file, b"world").unwrap();

        let joined = safe_join(&temp_dir, "sub/hello.txt");
        assert!(joined.is_some());
        assert_eq!(joined.unwrap(), file.canonicalize().unwrap());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn safe_join_rejects_parent_traversal() {
        let temp_dir = std::env::temp_dir().join(format!(
            "let_timer_safe_join_trav_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        assert!(safe_join(&temp_dir, "../outside.txt").is_none());
        assert!(safe_join(&temp_dir, "a/../../outside.txt").is_none());
        assert!(safe_join(&temp_dir, "..").is_none());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn safe_join_rejects_symlink_pointing_outside() {
        let temp_dir = std::env::temp_dir().join(format!(
            "let_timer_safe_join_sym_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = temp_dir.join("root");
        let outside = temp_dir.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        let secret = outside.join("secret.txt");
        std::fs::write(&secret, b"sensitive data").unwrap();

        let link = root.join("link_to_secret.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret, &link).unwrap();

        // safe_join should reject the symlink because its canonical target is outside root
        assert!(safe_join(&root, "link_to_secret.txt").is_none());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn safe_join_rejects_invalid_chars_and_absolute_paths() {
        let temp_dir = std::env::temp_dir().join(format!(
            "let_timer_safe_join_inv_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        assert!(safe_join(&temp_dir, "").is_none());
        assert!(safe_join(&temp_dir, "/").is_none());
        assert!(safe_join(&temp_dir, "foo\0bar").is_none());
        assert!(safe_join(&temp_dir, "foo\\bar").is_none());

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
