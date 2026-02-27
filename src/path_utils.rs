//! Path validation and utility functions

use anyhow::{Result as AnyhowResult, bail};
use std::path::{Path, PathBuf};

/// Validates that a path string is not empty or just whitespace
pub fn validate_path_str(path_str: &str) -> Result<(), String> {
    if path_str.trim().is_empty() {
        return Err("Path cannot be empty or contain only whitespace".to_string());
    }
    Ok(())
}

/// Creates a PathBuf from a string, validating it's not empty or whitespace
pub fn create_validated_pathbuf(path_str: &str) -> Result<PathBuf, String> {
    validate_path_str(path_str)?;
    Ok(PathBuf::from(path_str))
}

/// Safely gets the parent directory for creating directories.
/// Returns None for paths that don't need directory creation (like "config.yaml" in current dir)
pub fn safe_parent_for_creation(path: &Path) -> Option<&Path> {
    path.parent().filter(|p| !p.as_os_str().is_empty())
}

/// Checks if a path looks like a filename (no directory components)
pub fn is_just_filename(path: &Path) -> bool {
    path.parent().is_some_and(|p| p.as_os_str().is_empty())
}

/// Validates that a string is safe to use as a single path segment (e.g. an id or file stem).
///
/// Returns the trimmed segment on success.
pub fn validate_path_segment(segment: &str, what: &str) -> AnyhowResult<String> {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        bail!("{what} is required");
    }

    if trimmed.chars().count() > 128 {
        bail!("{what} is too long (max 128 characters)");
    }

    if trimmed == "." || trimmed == ".." {
        bail!("{what} must not be '.' or '..'");
    }

    if trimmed.contains('\0') {
        bail!("{what} must not contain NUL");
    }

    if trimmed.contains('/') || trimmed.contains('\\') {
        bail!("{what} must not contain path separators");
    }

    #[cfg(windows)]
    {
        if trimmed.ends_with('.') {
            bail!("{what} must not end with '.'");
        }

        const INVALID_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];
        if trimmed.chars().any(|ch| INVALID_CHARS.contains(&ch)) {
            bail!("{what} contains invalid characters");
        }

        let stem = trimmed.split('.').next().unwrap_or(trimmed);
        let stem_upper = stem.to_ascii_uppercase();

        const RESERVED: &[&str] = &["CON", "PRN", "AUX", "NUL"];
        if RESERVED.contains(&stem_upper.as_str()) {
            bail!("{what} uses a reserved device name");
        }

        if let Some(num) = stem_upper.strip_prefix("COM") {
            if matches!(num, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") {
                bail!("{what} uses a reserved device name");
            }
        }

        if let Some(num) = stem_upper.strip_prefix("LPT") {
            if matches!(num, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") {
                bail!("{what} uses a reserved device name");
            }
        }
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_str() {
        assert!(validate_path_str("").is_err());
        assert!(validate_path_str("   ").is_err());
        assert!(validate_path_str("\t").is_err());
        assert!(validate_path_str("valid/path").is_ok());
        assert!(validate_path_str("config.yaml").is_ok());
    }

    #[test]
    fn test_create_validated_pathbuf() {
        assert!(create_validated_pathbuf("").is_err());
        assert!(create_validated_pathbuf("   ").is_err());
        assert!(create_validated_pathbuf("valid/path").is_ok());
    }

    #[test]
    fn test_safe_parent_for_creation() {
        use std::path::Path;

        // Should return None for just filename
        assert!(safe_parent_for_creation(Path::new("config.yaml")).is_none());

        // Should return Some for paths with directories
        assert!(safe_parent_for_creation(Path::new("dir/config.yaml")).is_some());

        // Should return Some for absolute paths
        assert!(safe_parent_for_creation(Path::new("/tmp/config.yaml")).is_some());
    }

    #[test]
    fn test_is_just_filename() {
        use std::path::Path;

        assert!(is_just_filename(Path::new("config.yaml")));
        assert!(!is_just_filename(Path::new("dir/config.yaml")));
        assert!(!is_just_filename(Path::new("/tmp/config.yaml")));
    }

    #[test]
    fn test_validate_path_segment_basic() {
        assert!(validate_path_segment("", "name").is_err());
        assert!(validate_path_segment("   ", "name").is_err());
        assert!(validate_path_segment(".", "name").is_err());
        assert!(validate_path_segment("..", "name").is_err());
        assert!(validate_path_segment("a/b", "name").is_err());
        assert!(validate_path_segment("a\\b", "name").is_err());
        assert_eq!(validate_path_segment(" foo ", "name").unwrap(), "foo");
        assert_eq!(validate_path_segment("foo-bar", "name").unwrap(), "foo-bar");
        assert_eq!(validate_path_segment("draftpr", "name").unwrap(), "draftpr");
        assert_eq!(validate_path_segment("中文", "name").unwrap(), "中文");
    }

    #[cfg(windows)]
    #[test]
    fn test_validate_path_segment_windows_reserved_names() {
        assert!(validate_path_segment("con", "name").is_err());
        assert!(validate_path_segment("con.txt", "name").is_err());
        assert!(validate_path_segment("COM1", "name").is_err());
        assert!(validate_path_segment("LPT9.log", "name").is_err());
        assert!(validate_path_segment("bad:name", "name").is_err());
        assert!(validate_path_segment("trailing.", "name").is_err());
    }
}
