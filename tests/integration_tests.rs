mod common;
use common::*;
use std::process::Command;

#[test]
fn test_command_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "tool", "clean-useless-comments", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Clean useless comments from source code"));
    assert!(stdout.contains("--dry-run"));
    assert!(stdout.contains("--interactive"));
}

#[test]
fn test_command_with_config() {
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
        min-comment-length: 30
"#;

    env.create_config(config_content);
    env.create_file(
        "test.py",
        r#"# This is a very long comment that should be removed
def hello():
    # TODO: This should be preserved
    pass
"#,
    );

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "tool",
            "clean-useless-comments",
            "--config",
            env.path()
                .join(".llman")
                .join("config.yaml")
                .to_str()
                .unwrap(),
            "--dry-run",
            "--verbose",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("LLMAN_CONFIG_DIR", env.path().to_str().unwrap())
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Processing files"));
    assert!(stdout.contains("Dry run mode enabled"));
}

#[test]
fn test_command_with_no_config() {
    let env = TestEnvironment::new();
    env.create_file(
        "test.py",
        r#"# Short comment
def hello():
    pass
"#,
    );

    let output = Command::new("cargo")
        .args(&["run", "--", "tool", "clean-useless-comments", "--dry-run"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("LLMAN_CONFIG_DIR", env.path().to_str().unwrap())
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Clean useless comments command"));
    assert!(stdout.contains("Dry run mode enabled"));
}

/// Tests that the tool handles missing configuration files gracefully
/// by falling back to default configuration or showing appropriate error messages
#[test]
fn test_command_file_not_found_falls_back_to_default_or_shows_error() {
    let env = TestEnvironment::new();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "tool",
            "clean-useless-comments",
            "--config",
            "nonexistent_config.yaml",
            "--dry-run",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("LLMAN_CONFIG_DIR", env.path().to_str().unwrap())
        .output()
        .expect("Failed to run command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The tool should either succeed with default config or show a clear error
    if output.status.success() {
        // If successful, should show it's using some form of processing
        assert!(
            stdout.contains("Clean useless comments command")
                || stdout.contains("Dry run mode enabled")
        );
    } else {
        // If failed, should show a meaningful error about missing config
        assert!(
            stderr.contains("not found")
                || stderr.contains("No such file")
                || stdout.contains("Error")
        );
    }
}
