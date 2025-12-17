//! Path validation and utility functions

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
}
