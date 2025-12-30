mod common;
use clap::Parser;
use clap::error::ErrorKind;
use common::*;
use llman::cli::Cli;
use std::process::Command;

#[test]
fn test_command_help_is_display_help() {
    let err = match Cli::try_parse_from(["llman", "tool", "clean-useless-comments", "--help"]) {
        Ok(_) => panic!("Expected clap to short-circuit with help"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), ErrorKind::DisplayHelp);
}

#[test]
fn test_command_with_config_applies_changes() {
    let env = TestEnvironment::new();

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

    let config_path = env.create_config(config_content);
    let test_file = env.create_file(
        "test.py",
        r#"# Short comment
def hello():
    # TODO: This should be preserved
    pass
"#,
    );
    let config_dir = env.path();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--config-dir",
            config_dir.to_str().unwrap(),
            "tool",
            "clean-useless-comments",
            "--config",
            config_path.to_str().unwrap(),
            "--yes",
            "--force",
            test_file.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let updated = std::fs::read_to_string(&test_file).expect("Failed to read updated file");
    assert!(!updated.contains("Short comment"));
    assert!(updated.contains("TODO: This should be preserved"));
}

#[test]
fn test_command_with_no_config_uses_default_and_leaves_file_unchanged() {
    let env = TestEnvironment::new();
    let test_file = env.create_file(
        "test.py",
        r#"# Short comment
def hello():
    pass
"#,
    );
    let original = std::fs::read_to_string(&test_file).expect("Failed to read original file");
    let config_dir = env.path();
    let missing_config = env.path().join("missing_config.yaml");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--config-dir",
            config_dir.to_str().unwrap(),
            "tool",
            "clean-useless-comments",
            "--config",
            missing_config.to_str().unwrap(),
            "--yes",
            "--force",
            test_file.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let updated = std::fs::read_to_string(&test_file).expect("Failed to read updated file");
    assert_eq!(updated, original);
}

#[test]
fn test_command_missing_explicit_config_falls_back_to_default() {
    let env = TestEnvironment::new();

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

    let test_file = env.create_file(
        "test.py",
        r#"# Short comment
def hello():
    pass
"#,
    );
    let original = std::fs::read_to_string(&test_file).expect("Failed to read original file");
    let config_dir = env.path();
    let missing_config = env.path().join("missing_config.yaml");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--config-dir",
            config_dir.to_str().unwrap(),
            "tool",
            "clean-useless-comments",
            "--config",
            missing_config.to_str().unwrap(),
            "--yes",
            "--force",
            test_file.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let updated = std::fs::read_to_string(&test_file).expect("Failed to read updated file");
    assert_eq!(updated, original);
}
