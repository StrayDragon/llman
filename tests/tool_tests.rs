use llman::tool::clean_comments;
use llman::tool::command::CleanUselessCommentsArgs;
use llman::tool::config::Config;
use llman::tool::processor::CommentProcessor;
mod common;
use common::*;

/// Tests the clean_comments module entry point
/// This tests the main command runner
#[test]
fn test_clean_comments_command_with_valid_args() {
    let env = TestEnvironment::new();

    // Create a test file and config
    let test_file = env.create_file("test.py", "# Short comment\ndef test(): pass");
    env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH);

    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: true,
        verbose: false,
        git_only: false,
        files: vec![test_file],
    };

    // This should not panic or return an error
    let result = clean_comments::run(&args);
    assert!(result.is_ok());
}

/// Tests clean_comments command with default config
#[test]
fn test_clean_comments_command_with_default_config() {
    let env = TestEnvironment::new();
    let test_file = env.create_file("test.py", "# Some comment\ndef test(): pass");

    let args = CleanUselessCommentsArgs {
        config: None, // Use default
        dry_run: true,
        yes: false,
        interactive: false,
        force: true,
        verbose: false,
        git_only: false,
        files: vec![test_file],
    };

    let result = clean_comments::run(&args);
    assert!(result.is_ok());
}

/// Tests clean_comments command with missing config file
#[test]
fn test_clean_comments_command_with_missing_config() {
    let env = TestEnvironment::new();
    let test_file = env.create_file("test.py", "# Comment\ndef test(): pass");

    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join("nonexistent.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: true,
        verbose: false,
        git_only: false,
        files: vec![test_file],
    };

    let result = clean_comments::run(&args);
    assert!(result.is_ok()); // Should fall back to default config
}

/// Tests CommentProcessor creation with different configurations
#[test]
fn test_comment_processor_creation() {
    let env = TestEnvironment::new();
    env.create_python_clean_config(test_constants::DEFAULT_MIN_COMMENT_LENGTH);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: true,
        verbose: false,
        git_only: false,
        files: vec![],
    };

    let _processor = CommentProcessor::new(config, args);
    // Processor should be created successfully (even if TreeSitter fails)
    // If we reach here, creation succeeded
}

/// Tests CommentProcessor with various file scenarios
#[test]
fn test_comment_processor_with_empty_files_list() {
    let env = TestEnvironment::new();
    env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: true,
        verbose: false,
        git_only: false,
        files: vec![], // Empty files list
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process();
    assert!(result.is_ok());

    let processing_result = result.unwrap();
    assert_eq!(processing_result.errors, 0);
}

/// Tests CommentProcessor with non-existent files
#[test]
fn test_comment_processor_with_nonexistent_files() {
    let env = TestEnvironment::new();
    env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let nonexistent_file = env.path().join("nonexistent.py");

    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: true,
        verbose: false,
        git_only: false,
        files: vec![nonexistent_file],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process();

    // Should handle gracefully - either process successfully with 0 files or return a specific error
    assert!(result.is_ok());
}

/// Tests CommentProcessor with invalid configuration
#[test]
fn test_comment_processor_with_invalid_config() {
    let env = TestEnvironment::new();

    // Create invalid YAML config
    let invalid_config = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    lang-rules:
      python:
        single-line-comments: "not_a_boolean"  # Invalid type
"#;
    env.create_config(invalid_config);

    let config_result = Config::load(env.path().join(".llman").join("config.yaml"));
    assert!(config_result.is_err());
}

/// Tests verbose mode in clean_comments
#[test]
fn test_clean_comments_verbose_mode() {
    let env = TestEnvironment::new();
    let test_file = env.create_file("test.py", "# Comment\ndef test(): pass");
    env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH);

    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: true,
        verbose: true, // Enable verbose mode
        git_only: false,
        files: vec![test_file],
    };

    let result = clean_comments::run(&args);
    assert!(result.is_ok());
}

/// Tests interactive mode flag
#[test]
fn test_clean_comments_interactive_mode() {
    let env = TestEnvironment::new();
    let test_file = env.create_file("test.py", "# Comment\ndef test(): pass");
    env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH);

    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: true, // Enable interactive mode
        force: true,
        verbose: false,
        git_only: false,
        files: vec![test_file],
    };

    let result = clean_comments::run(&args);
    assert!(result.is_ok());
}

/// Tests git-only mode
#[test]
fn test_clean_comments_git_only_mode() {
    let env = TestEnvironment::new();
    let test_file = env.create_file("test.py", "# Comment\ndef test(): pass");
    env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH);

    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: true,
        verbose: false,
        git_only: true, // Only git-tracked files
        files: vec![test_file],
    };

    let result = clean_comments::run(&args);
    assert!(result.is_ok());
}

/// Tests different dry-run configurations
#[test]
fn test_clean_comments_dry_run_modes() {
    let env = TestEnvironment::new();
    let test_file = env.create_file("test.py", "# Short comment\ndef test(): pass");
    env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH);

    // Test with dry-run enabled
    let args_dry = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: true,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let result_dry = clean_comments::run(&args_dry);
    assert!(result_dry.is_ok());

    // Test with dry-run disabled (live mode)
    let args_live = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: false,
        yes: true,
        interactive: false,
        force: true,
        verbose: false,
        git_only: false,
        files: vec![test_file],
    };

    let result_live = clean_comments::run(&args_live);
    assert!(result_live.is_ok());
}

#[test]
fn test_clean_comments_dry_run_first_safety() {
    let env = TestEnvironment::new();
    let original = "# Short comment\ndef test(): pass";
    let test_file = env.create_file("test.py", original);

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
        min-comment-length: 20
    safety:
      dry-run-first: true
"#;

    env.create_config(config_content);

    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: false,
        yes: true,
        interactive: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let result = clean_comments::run(&args);
    assert!(result.is_ok());

    let actual = std::fs::read_to_string(&test_file).unwrap();
    assert_eq!(actual, original);
}
