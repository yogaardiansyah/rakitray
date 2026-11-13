use crate::store::StoreError;
use std::path::{Component, Path, PathBuf};

/// Validates that an entry relative path stays strictly inside staging_root.
/// Rejects:
/// 1. Absolute paths
/// 2. Parent dir traversals (`..`)
/// 3. Windows drive prefixes (`C:`)
pub fn validate_safe_relative_path(
    staging_root: &Path,
    entry_path: &Path,
) -> Result<PathBuf, StoreError> {
    for comp in entry_path.components() {
        match comp {
            Component::ParentDir => {
                return Err(StoreError::PathTraversal(format!(
                    "Path traversal '..' detected: {}",
                    entry_path.display()
                )));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(StoreError::PathTraversal(format!(
                    "Absolute or drive path detected: {}",
                    entry_path.display()
                )));
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }

    let joined = staging_root.join(entry_path);
    // Extra guard: check lexical containment
    if !joined.starts_with(staging_root) {
        return Err(StoreError::PathTraversal(format!(
            "Resolved path escapes staging root: {}",
            joined.display()
        )));
    }

    Ok(joined)
}

/// Ensures a symlink target stays strictly inside the staging root.
pub fn validate_symlink_target(
    staging_root: &Path,
    link_destination_path: &Path,
    raw_target: &Path,
) -> Result<(), StoreError> {
    let parent = link_destination_path.parent().unwrap_or(staging_root);
    let resolved = if raw_target.is_absolute() {
        return Err(StoreError::PathTraversal(format!(
            "Absolute symlink target rejected: {}",
            raw_target.display()
        )));
    } else {
        parent.join(raw_target)
    };

    // Normalize path components
    let mut normalized = PathBuf::new();
    for comp in resolved.components() {
        match comp {
            Component::Prefix(_) => {
                return Err(StoreError::PathTraversal(
                    "Symlink with prefix rejected".to_string(),
                ));
            }
            Component::RootDir => {
                normalized.push(std::path::MAIN_SEPARATOR.to_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(StoreError::PathTraversal(format!(
                        "Symlink escapes staging root: {}",
                        raw_target.display()
                    )));
                }
            }
            Component::Normal(c) => normalized.push(c),
        }
    }

    let mut staging_normalized = PathBuf::new();
    for comp in staging_root.components() {
        match comp {
            Component::RootDir => {
                staging_normalized.push(std::path::MAIN_SEPARATOR.to_string());
            }
            Component::CurDir => {}
            Component::Normal(c) => staging_normalized.push(c),
            c => staging_normalized.push(c.as_os_str()),
        }
    }

    if !normalized.starts_with(&staging_normalized) {
        return Err(StoreError::PathTraversal(format!(
            "Symlink target points outside staging directory: {}",
            raw_target.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_path_valid() {
        let root = Path::new("/tmp/staging_123");
        let safe = Path::new("bin/mariadbd");
        let res = validate_safe_relative_path(root, safe).unwrap();
        assert_eq!(res, Path::new("/tmp/staging_123/bin/mariadbd"));
    }

    #[test]
    fn test_safe_path_rejects_parent_dir() {
        let root = Path::new("/tmp/staging_123");
        let traversal = Path::new("../../etc/passwd");
        let res = validate_safe_relative_path(root, traversal);
        assert!(matches!(res, Err(StoreError::PathTraversal(_))));
    }

    #[test]
    fn test_safe_path_rejects_absolute() {
        let root = Path::new("/tmp/staging_123");
        let abs = Path::new("/etc/shadow");
        let res = validate_safe_relative_path(root, abs);
        assert!(matches!(res, Err(StoreError::PathTraversal(_))));
    }

    #[test]
    fn test_symlink_escape_rejected() {
        let root = Path::new("/tmp/staging_123");
        let link_dest = Path::new("/tmp/staging_123/bin/tool_link");
        let evil_target = Path::new("../../../../etc/passwd");
        let res = validate_symlink_target(root, link_dest, evil_target);
        assert!(matches!(res, Err(StoreError::PathTraversal(_))));
    }

    #[test]
    fn test_symlink_internal_valid() {
        let root = Path::new("/tmp/staging_123");
        let link_dest = Path::new("/tmp/staging_123/bin/php");
        let valid_target = Path::new("php-cgi");
        let res = validate_symlink_target(root, link_dest, valid_target);
        assert!(res.is_ok());
    }
}
