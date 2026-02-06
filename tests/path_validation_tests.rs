use anyhow::Result;
use llman::path_utils::{create_validated_pathbuf, safe_parent_for_creation, validate_path_str};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
mod env_lock;

#[test]
fn test_prevent_empty_path_creation() -> Result<()> {
    // Test that empty paths are rejected
    assert!(validate_path_str("").is_err());
    assert!(validate_path_str("   ").is_err());
    assert!(validate_path_str("\t").is_err());

    // Valid paths should work
    assert!(validate_path_str("config").is_ok());
    assert!(validate_path_str("/tmp/config").is_ok());

    Ok(())
}

#[test]
fn test_safe_parent_prevents_empty_directory_creation() -> Result<()> {
    use std::path::Path;

    // Test behavior with relative filename - this is where the bug occurs!
    let config_file = Path::new("config.yaml");
    // parent() returns Some("") for relative filenames, which is the bug source
    assert!(config_file.parent().is_some());
    assert!(config_file.parent().unwrap().as_os_str().is_empty());

    // Our safe_parent_for_creation should filter this out
    assert!(safe_parent_for_creation(config_file).is_none());

    let config_file_path = PathBuf::from("config.yaml");
    assert!(safe_parent_for_creation(&config_file_path).is_none());

    let temp_dir = TempDir::new()?;

    // For an absolute path with just filename, safe_parent should return the parent directory
    let config_file = temp_dir.path().join("config.yaml");
    let parent = safe_parent_for_creation(&config_file);
    assert!(parent.is_some());
    assert_eq!(parent.unwrap(), temp_dir.path());

    // For a path with directories, it should return the parent
    let nested_config = temp_dir.path().join("dir").join("config.yaml");
    assert!(safe_parent_for_creation(&nested_config).is_some());

    Ok(())
}

#[test]
fn test_path_creation_with_validation() -> Result<()> {
    // Valid paths should create PathBuf
    assert!(create_validated_pathbuf("config.yaml").is_ok());
    assert!(create_validated_pathbuf("/tmp/config").is_ok());

    // Invalid paths should fail
    assert!(create_validated_pathbuf("").is_err());
    assert!(create_validated_pathbuf("   ").is_err());

    Ok(())
}

#[test]
fn test_environment_variable_validation() -> Result<()> {
    // This test simulates what happens when LLMAN_CONFIG_DIR is empty
    let empty_env_var = "";

    // The validation should catch this
    let result = validate_path_str(empty_env_var);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_no_space_directory_creation() -> Result<()> {
    let _guard = env_lock::lock_env();
    let temp_dir = TempDir::new()?;
    let _cwd = env_lock::CwdGuard::set(temp_dir.path())?;

    // Try to create a file with just a filename (no parent directories)
    let config_path = PathBuf::from("test_config.yaml");

    // This should not create any directories
    if let Some(parent) = safe_parent_for_creation(&config_path) {
        fs::create_dir_all(parent)?;
    }

    // Write the file
    fs::write(&config_path, "test: config")?;

    // Check that no weird directories were created
    for entry in std::fs::read_dir(&temp_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Should not have empty or space-only directory names
            assert!(
                !filename.trim().is_empty(),
                "Found directory with empty/space name: {:?}",
                path
            );
        }
    }

    Ok(())
}
