use std::path::{Component, Path, PathBuf};

/// Checks if an entry relative path should be stripped to save disk space.
/// Patterns applied after `strip_components`.
pub fn should_strip_path(rel_path: &Path, strip_profile: &str) -> bool {
    if strip_profile == "none" {
        return false;
    }

    let path_str = rel_path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('\\', "/");

    // 1. Debug & static libs
    if path_str.ends_with(".pdb")
        || path_str.ends_with(".lib")
        || path_str.ends_with(".exp")
        || path_str.ends_with(".a")
    {
        return true;
    }

    // 2. DB test / bench suites
    if path_str.starts_with("mysql-test/")
        || path_str.contains("/mysql-test/")
        || path_str.starts_with("sql-bench/")
        || path_str.contains("/sql-bench/")
        || path_str.starts_with("test/")
        || path_str.contains("/test/")
    {
        return true;
    }

    // 3. Offline docs & manuals
    if path_str.starts_with("doc/")
        || path_str.contains("/doc/")
        || path_str.starts_with("docs/")
        || path_str.contains("/docs/")
        || path_str.starts_with("manual/")
        || path_str.contains("/manual/")
        || path_str.starts_with("man/")
        || path_str.contains("/man/")
    {
        return true;
    }

    false
}

/// Strips `count` leading directory components from a path.
/// Returns None if the path has fewer components than `count`.
pub fn strip_components(path: &Path, count: u8) -> Option<PathBuf> {
    if count == 0 {
        return Some(path.to_path_buf());
    }

    let mut components = path.components();
    for _ in 0..count {
        components.next()?;
    }

    let mut stripped = PathBuf::new();
    let mut has_any = false;
    for comp in components {
        if let Component::Normal(c) = comp {
            stripped.push(c);
            has_any = true;
        }
    }

    if has_any {
        Some(stripped)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_debug_and_lib() {
        assert!(should_strip_path(Path::new("bin/mariadbd.pdb"), "default"));
        assert!(should_strip_path(
            Path::new("lib/libmariadbclient.a"),
            "default"
        ));
        assert!(should_strip_path(Path::new("lib/mariadb.lib"), "default"));
        assert!(!should_strip_path(Path::new("bin/mariadbd"), "default"));
    }

    #[test]
    fn test_strip_test_and_doc() {
        assert!(should_strip_path(
            Path::new("mysql-test/suite/test.pl"),
            "default"
        ));
        assert!(should_strip_path(Path::new("docs/manual.html"), "default"));
        assert!(should_strip_path(
            Path::new("man/man1/mariadb.1"),
            "default"
        ));
        assert!(!should_strip_path(
            Path::new("share/english/errmsg.sys"),
            "default"
        ));
    }

    #[test]
    fn test_strip_components() {
        let p = Path::new("mariadb-11.4.3-linux-x64/bin/mariadbd");
        let stripped = strip_components(p, 1).unwrap();
        assert_eq!(stripped, Path::new("bin/mariadbd"));

        let single = Path::new("mariadb-11.4.3-linux-x64");
        assert!(strip_components(single, 1).is_none());
    }
}
