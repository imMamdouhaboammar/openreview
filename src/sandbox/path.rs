use crate::error::{AppError, Result};
use std::path::{Component, Path, PathBuf};

/// Safely resolves a user-provided path within a base directory.
/// Prevents directory traversal (`..`), symlink escapes, and access outside `base`.
pub fn safe_resolve_path(base: &Path, user_path: &str) -> Result<PathBuf> {
    if user_path.is_empty() {
        return Err(AppError::PathTraversal("Empty path is invalid".to_string()));
    }

    let path_obj = Path::new(user_path);
    if path_obj.is_absolute() {
        return Err(AppError::PathTraversal(
            "Absolute paths are not permitted".to_string(),
        ));
    }

    // Reject explicit parent directory navigation components
    for comp in path_obj.components() {
        match comp {
            Component::ParentDir => {
                return Err(AppError::PathTraversal(format!(
                    "Path traversal with '..' detected in '{user_path}'"
                )));
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(AppError::PathTraversal(format!(
                    "Prefixes not permitted in '{user_path}'"
                )));
            }
            _ => {}
        }
    }

    // Canonicalize base directory
    let canonical_base = base
        .canonicalize()
        .map_err(|e| AppError::Sandbox(format!("Failed to canonicalize base directory: {e}")))?;

    let mut target = canonical_base.clone();
    for component in path_obj.components() {
        if let Component::Normal(name) = component {
            target.push(name);
            match std::fs::symlink_metadata(&target) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(AppError::PathTraversal(format!(
                        "Symlinks are not permitted in '{user_path}'"
                    )));
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(AppError::Sandbox(format!(
                        "Failed to inspect path '{user_path}': {e}"
                    )));
                }
            }
        }
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_valid_relative_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("foo/bar.txt");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "content").unwrap();

        let resolved = safe_resolve_path(dir.path(), "foo/bar.txt").unwrap();
        assert_eq!(resolved, file_path.canonicalize().unwrap());
    }

    #[test]
    fn test_valid_new_file_path() {
        let dir = tempdir().unwrap();
        let resolved = safe_resolve_path(dir.path(), "new_dir/new_file.txt").unwrap();
        assert!(resolved.starts_with(dir.path().canonicalize().unwrap()));
    }

    #[test]
    fn test_rejects_parent_traversal() {
        let dir = tempdir().unwrap();
        let err = safe_resolve_path(dir.path(), "../outside.txt").unwrap_err();
        assert!(matches!(err, AppError::PathTraversal(_)));

        let err2 = safe_resolve_path(dir.path(), "foo/../../outside.txt").unwrap_err();
        assert!(matches!(err2, AppError::PathTraversal(_)));
    }

    #[test]
    fn test_rejects_empty_path() {
        let dir = tempdir().unwrap();
        let err = safe_resolve_path(dir.path(), "").unwrap_err();
        assert!(matches!(err, AppError::PathTraversal(_)));
    }

    #[cfg(unix)]
    #[test]
    fn test_rejects_dangling_symlink_and_absolute_path() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        symlink(
            "/tmp/nonexistent-openreview-target",
            dir.path().join("dangling"),
        )
        .unwrap();
        assert!(matches!(
            safe_resolve_path(dir.path(), "dangling"),
            Err(AppError::PathTraversal(_))
        ));
        assert!(matches!(
            safe_resolve_path(dir.path(), "/etc/passwd"),
            Err(AppError::PathTraversal(_))
        ));
    }

    #[test]
    fn test_rejects_symlink_escape() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let secret = outside.path().join("secret.env");
        std::fs::write(&secret, "SECRET=123").unwrap();

        // Create symlink inside dir pointing to outside file
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link = dir.path().join("link_to_secret");
            symlink(&secret, &link).unwrap();

            let err = safe_resolve_path(dir.path(), "link_to_secret").unwrap_err();
            assert!(matches!(err, AppError::PathTraversal(_)));
        }
    }
}
