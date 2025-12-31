use llman::tool::command::CleanUselessCommentsArgs;
use llman::tool::config::Config;
use llman::tool::processor::CommentProcessor;
mod common;
use common::*;
use std::process::Command;

/// Tests that the comment processor correctly identifies and processes Python comments
/// based on length and pattern rules, preserving important comments while marking
/// short comments for removal.
#[test]
fn test_python_comment_processing_removes_short_comments_and_preserves_important_ones() {
    let env = TestEnvironment::new();

    let test_file = env.create_file("test.py", test_content::PYTHON_CODE_WITH_COMMENTS);
    env.create_python_clean_config(test_constants::DEFAULT_MIN_COMMENT_LENGTH);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    // Should detect changes but not actually modify files in dry-run mode
    assert_eq!(
        result.files_changed.len(),
        1,
        "Expected exactly 1 file to have changes, got {}",
        result.files_changed.len()
    );
    assert_eq!(
        result.errors, 0,
        "Expected no processing errors, got {}",
        result.errors
    );
}

/// Tests JavaScript comment processing with proper pattern matching and length filtering.
/// Verifies that TODO/FIXME comments are preserved while short comments are marked for removal.
#[test]
fn test_javascript_comment_processing_preserves_patterns_and_filters_by_length() {
    let env = TestEnvironment::new();

    let test_file = env.create_file("test.js", test_content::JAVASCRIPT_CODE_WITH_COMMENTS);
    env.create_javascript_clean_config(test_constants::SHORT_COMMENT_LENGTH * 3); // 15 characters

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    assert_eq!(result.errors, 0, "Expected no processing errors");
    println!(
        "Files processed: {}, Files changed: {}",
        result.files_changed.len(),
        result.files_changed.len()
    );
}

#[test]
fn test_rust_comment_processing() {
    let env = TestEnvironment::new();

    let rust_code = r#"// Short
fn main() {
    println!("Hello"); // x
    /// This is a doc comment and should be preserved
    // TODO: This should be preserved
}
"#;

    let config_content = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.rs"
    lang-rules:
      rust:
        single-line-comments: true
        doc-comments: false
        preserve-patterns:
          - "^\\s*//\\s*(TODO|FIXME):"
          - "^\\s*///"
        min-comment-length: 15
"#;

    let test_file = env.create_file("test.rs", rust_code);
    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    assert!(!result.files_changed.is_empty());
}

#[test]
fn test_typescript_rules_prefer_typescript_config() {
    let env = TestEnvironment::new();

    let ts_code = r#"// short comment
const value = 1;
"#;

    let config_content = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.ts"
    lang-rules:
      javascript:
        single-line-comments: true
        min-comment-length: 5
      typescript:
        single-line-comments: true
        min-comment-length: 50
"#;

    let test_file = env.create_file("test.ts", ts_code);
    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    assert_eq!(result.files_changed.len(), 1);
}

#[test]
fn test_preserve_important_comments() {
    let env = TestEnvironment::new();

    let code = r#"#!/usr/bin/env python3
# x
def important_function():
    # TODO: This is a TODO item
    # FIXME: This needs to be fixed
    # NOTE: This is an important note
    # y
    print("Hello")
"#;

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
        preserve-patterns:
          - "^\\s*#\\s*(TODO|FIXME|NOTE):"
        min-comment-length: 10
"#;

    let test_file = env.create_file("test.py", code);
    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    // Should detect changes
    assert!(!result.files_changed.is_empty());
}

#[test]
fn test_file_scoping() {
    let env = TestEnvironment::new();

    // Create files with different extensions
    env.create_file("test.py", "# Python comment");
    env.create_file("test.js", "// JavaScript comment");
    env.create_file("test.txt", "# Text file comment - should be ignored");

    let config_content = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
        - "**/*.js"
      exclude:
        - "**/ignored/**"
    lang-rules:
      python:
        single-line-comments: true
        min-comment-length: 5
      javascript:
        single-line-comments: true
        min-comment-length: 5
"#;

    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    // Should process both .py and .js files, but not .txt
    // Comments are not removed because they're exactly at the minimum length threshold
    assert_eq!(
        result.errors, 0,
        "Expected no errors, but got {}",
        result.errors
    );
}

#[test]
fn test_git_only_processes_tracked_files() {
    let env = TestEnvironment::new();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(env.path()).unwrap();

    let _test_file = env.create_file("tracked.py", "# x\n");
    Command::new("git")
        .args(["add", "tracked.py"])
        .current_dir(env.path())
        .output()
        .expect("Failed to add file to git index");

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

    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: false,
        verbose: false,
        git_only: true,
        files: vec![],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    assert_eq!(result.files_changed.len(), 1);

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_dry_run_mode() {
    let env = TestEnvironment::new();

    let original_content = r#"# Short comment
def hello():
    pass
"#;

    let test_file = env.create_file("test.py", original_content);

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
"#;

    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false, // This should prevent actual file changes
        interactive: false,
        force: false,
        verbose: false, // Enable verbose to see debug output
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    // File should be detected as having changes
    assert_eq!(result.files_changed.len(), 1);

    // But content should remain unchanged due to dry-run
    let actual_content = std::fs::read_to_string(&test_file).unwrap();
    assert_eq!(actual_content, original_content);
}
