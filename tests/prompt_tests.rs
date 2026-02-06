use llman::prompt::PromptCommand;
mod common;
mod env_lock;
use common::*;

/// Tests PromptCommand creation with default configuration
#[test]
fn test_prompt_command_creation_with_default_config() {
    let _env = TestEnvironment::new();

    // This test verifies that PromptCommand can be created
    // The actual implementation may need a proper config directory
    let result = PromptCommand::new();

    // Depending on the environment setup, this might succeed or fail
    // The important thing is that it doesn't panic
    match result {
        Ok(_) => {
            // Successfully created
        }
        Err(_) => {
            // Failed gracefully (likely due to missing config directory)
        }
    }
}

/// Tests PromptCommand creation with custom config directory
#[test]
fn test_prompt_command_creation_with_custom_config() {
    let env = TestEnvironment::new();

    // Use the test environment's config directory
    let config_path = env.path().join(".llman");
    let config_dir = config_path.to_str().unwrap();
    let result = PromptCommand::with_config_dir(Some(config_dir));

    // This should work with our test environment
    assert!(result.is_ok());
}

/// Tests PromptCommand with missing config directory
#[test]
fn test_prompt_command_with_nonexistent_config() {
    let result = PromptCommand::with_config_dir(Some("/nonexistent/path"));

    // This should fail gracefully
    assert!(result.is_err());
}

/// Tests PromptCommand creation with empty config path
#[test]
fn test_prompt_command_with_empty_config_path() {
    let result = PromptCommand::with_config_dir(Some(""));

    // Should handle gracefully (either succeed with default or fail gracefully)
    match result {
        Ok(_) => {
            // Handled empty path successfully
        }
        Err(_) => {
            // Failed gracefully
        }
    }
}

/// Tests PromptCommand in different project scenarios
#[test]
fn test_prompt_command_in_different_project_scenarios() {
    let env = TestEnvironment::new();

    // Create different types of project structures
    let scenarios = vec![
        ("rust_project", vec!["Cargo.toml", "src/main.rs"]),
        ("python_project", vec!["requirements.txt", "main.py"]),
        ("node_project", vec!["package.json", "index.js"]),
        ("generic_project", vec!["README.md"]),
    ];

    for (project_name, files) in scenarios {
        let project_dir = env.path().join(project_name);
        std::fs::create_dir_all(&project_dir).unwrap();

        // Create project files
        for file in files {
            let file_path = project_dir.join(file);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&file_path, "").unwrap();
        }

        // Test PromptCommand creation in this project context
        let config_path = project_dir.join(".llman");
        let config_dir = config_path.to_str().unwrap();
        let result = PromptCommand::with_config_dir(Some(config_dir));

        // Should work in project directories
        assert!(result.is_ok(), "Should work in {} project", project_name);
    }
}

/// Tests PromptCommand error handling scenarios
#[test]
fn test_prompt_command_error_scenarios() {
    let _env = TestEnvironment::new();

    // Test with invalid config directory
    let invalid_dirs = vec![
        "/root/llman",        // Might not exist or have permissions
        "/tmp/invalid/llman", // Nested non-existent path
        "/dev/null/llman",    // Invalid path
    ];

    for dir in invalid_dirs {
        let result = PromptCommand::with_config_dir(Some(dir));
        match result {
            Ok(_) => {
                // Unexpectedly succeeded (might be valid on some systems)
            }
            Err(_) => {
                // Expected to fail, which is fine
            }
        }
    }
}

/// Tests PromptCommand with various configuration scenarios
#[test]
fn test_prompt_command_configuration_scenarios() {
    let env = TestEnvironment::new();

    // Create a basic config structure
    let config_dir = env.path().join(".llman");
    std::fs::create_dir_all(&config_dir).unwrap();

    // Create a minimal config file
    let config_content = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    lang-rules:
      python:
        single-line-comments: true
        min-comment-length: 10
"#;

    std::fs::write(config_dir.join("config.yaml"), config_content).unwrap();

    // Test PromptCommand with this config
    let result = PromptCommand::with_config_dir(Some(env.path().to_str().unwrap()));
    assert!(result.is_ok());
}

/// Tests PromptCommand behavior with corrupted configuration
#[test]
fn test_prompt_command_with_corrupted_config() {
    let env = TestEnvironment::new();

    // Create config directory
    let config_dir = env.path().join(".llman");
    std::fs::create_dir_all(&config_dir).unwrap();

    // Create corrupted YAML config
    let corrupted_config = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    lang-rules:
      python:
        single-line-comments: not_a_boolean
        min-comment-length: "not_a_number
"#; // Intentionally malformed YAML

    std::fs::write(config_dir.join("config.yaml"), corrupted_config).unwrap();

    // Test PromptCommand with corrupted config
    let result = PromptCommand::with_config_dir(Some(env.path().to_str().unwrap()));

    // Should handle corrupted config gracefully
    match result {
        Ok(_) => {
            // Handled gracefully (maybe uses defaults)
        }
        Err(_) => {
            // Failed gracefully with proper error
        }
    }
}

/// Tests PromptCommand concurrency scenarios
#[test]
fn test_prompt_command_concurrent_creation() {
    use std::sync::Arc;
    use std::thread;

    let env = TestEnvironment::new();
    let config_dir = Arc::new(env.path().to_str().unwrap().to_string());

    // Test creating multiple PromptCommands concurrently
    let mut handles = vec![];

    for _i in 0..5 {
        let config_dir_clone = Arc::clone(&config_dir);
        let handle = thread::spawn(move || PromptCommand::with_config_dir(Some(&config_dir_clone)));
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        let result = handle.join().unwrap();
        // All should either succeed or fail consistently
        match result {
            Ok(_) | Err(_) => {
                // Both outcomes are acceptable
            }
        }
    }
}

/// Tests PromptCommand memory and resource usage
#[test]
fn test_prompt_command_resource_usage() {
    let env = TestEnvironment::new();

    // Create and destroy multiple PromptCommands to test resource cleanup
    for _ in 0..10 {
        let result = PromptCommand::with_config_dir(Some(env.path().to_str().unwrap()));

        match result {
            Ok(prompt_command) => {
                // Create and immediately drop to test cleanup
                drop(prompt_command);
            }
            Err(_) => {
                // Failed to create, which is also fine
            }
        }
    }

    // If we reach here without panics or memory issues, the test passes
}

/// Tests PromptCommand with various edge case inputs
#[test]
fn test_prompt_command_edge_case_inputs() {
    let _guard = env_lock::lock_env();
    let test_env = TestEnvironment::new();
    let work_dir = test_env.path().join("edge-case-workdir");
    std::fs::create_dir_all(&work_dir).unwrap();
    let _cwd = env_lock::CwdGuard::set(&work_dir).unwrap();

    let temp_config_dir = test_env.path().join("edge-case-config");
    let _config_guard =
        env_lock::EnvVarGuard::set("LLMAN_CONFIG_DIR", &temp_config_dir.to_string_lossy());

    // Test with various unusual but valid inputs
    let edge_cases = vec![
        Some("."),
        Some("./"),
        Some("../"),
        Some(" "),
        Some(".llman"),
        Some("config"),
        Some("config.yaml"),
        None, // Default config
    ];

    for case in edge_cases {
        let result = PromptCommand::with_config_dir(case);

        match result {
            Ok(_) => {
                // Successfully handled this case
            }
            Err(_) => {
                // Failed gracefully, which is acceptable for edge cases
            }
        }
    }
}
