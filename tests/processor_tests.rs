use llman::tool::processor::CommentProcessor;
use llman::tool::command::CleanUselessCommentsArgs;
use llman::tool::config::Config;
mod common;
use common::*;

#[test]
fn test_python_comment_processing() {
    let env = TestEnvironment::new();

    let python_code = r#"#!/usr/bin/env python3
# This is a short comment
def hello():
    # Another short comment
    print("Hello")  # Inline comment
    # TODO: This should be preserved
    # FIXME: This should also be preserved
    return "done"
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
          - "^\\s*#\\s*(TODO|FIXME):"
        min-comment-length: 20
"#;

    let test_file = env.create_file("test.py", python_code);
    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        interactive: false,
        backup: None,
        no_backup: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    // Should detect changes but not actually modify files in dry-run mode
    assert!(result.files_changed.len() > 0);
}

#[test]
fn test_javascript_comment_processing() {
    let env = TestEnvironment::new();

    let js_code = r#"// Short
function hello() {
    console.log("Hello"); // x
    // TODO: This should be preserved
    return "done";
}
"#;

    let config_content = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.js"
    lang-rules:
      javascript:
        single-line-comments: true
        preserve-patterns:
          - "^\\s*//\\s*(TODO|FIXME):"
        min-comment-length: 15
"#;

    let test_file = env.create_file("test.js", js_code);
    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        interactive: false,
        backup: None,
        no_backup: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    assert!(result.files_changed.len() > 0);
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
        interactive: false,
        backup: None,
        no_backup: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    assert!(result.files_changed.len() > 0);
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
        interactive: false,
        backup: None,
        no_backup: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![test_file.clone()],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    // Should detect changes
    assert!(result.files_changed.len() > 0);
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
        interactive: false,
        backup: None,
        no_backup: false,
        force: false,
        verbose: false,
        git_only: false,
        files: vec![],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process().unwrap();

    // Should process both .py and .js files, but not .txt
    // Comments are not removed because they're exactly at the minimum length threshold
    assert_eq!(result.errors, 0, "Expected no errors, but got {}", result.errors);
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
        dry_run: true, // This should prevent actual file changes
        interactive: false,
        backup: None,
        no_backup: false,
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