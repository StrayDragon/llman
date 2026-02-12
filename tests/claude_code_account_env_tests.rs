use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn run_llman(args: &[&str], work_dir: &Path, config_dir: &Path) -> Output {
    Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--manifest-path",
            manifest_path().to_str().expect("manifest path"),
            "--",
            "--config-dir",
            config_dir.to_str().expect("config dir"),
        ])
        .args(args)
        .current_dir(work_dir)
        .output()
        .expect("Failed to run llman command")
}

fn write_claude_code_config(config_dir: &Path, content: &str) {
    fs::create_dir_all(config_dir).expect("create config dir");
    let config_path = config_dir.join("claude-code.toml");
    fs::write(config_path, content).expect("write claude-code.toml");
}

#[test]
fn claude_code_account_env_emits_sorted_injection_statements_with_escaping() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let config_dir = work_dir.join("config");

    write_claude_code_config(
        &config_dir,
        r#"
[groups]

[groups.g]
B = "2"
A = "1"
QUOTE = "a'b"
SPACE = "hello world"
"#,
    );

    let output = run_llman(
        &["x", "claude-code", "account", "env", "g"],
        work_dir,
        &config_dir,
    );

    assert!(
        output.status.success(),
        "command failed: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if cfg!(windows) {
        assert_eq!(
            lines,
            vec![
                "# PowerShell: llman x claude-code account env g | Out-String | Invoke-Expression",
                "$env:A='1'",
                "$env:B='2'",
                "$env:QUOTE='a''b'",
                "$env:SPACE='hello world'",
            ]
        );
    } else {
        assert_eq!(
            lines,
            vec![
                "# Bash/Zsh: source <(llman x claude-code account env g) && ...",
                "export A='1'",
                "export B='2'",
                "export QUOTE='a'\\''b'",
                "export SPACE='hello world'",
            ]
        );
    }
}

#[test]
fn claude_code_account_env_invalid_key_fails_without_output() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let config_dir = work_dir.join("config");

    write_claude_code_config(
        &config_dir,
        r#"
[groups]

[groups.g]
BAD-KEY = "1"
"#,
    );

    let output = run_llman(
        &["x", "claude-code", "account", "env", "g"],
        work_dir,
        &config_dir,
    );

    assert!(!output.status.success(), "expected failure");
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid environment variable key"));
}

#[test]
fn claude_code_account_env_group_not_found_fails_without_output() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let config_dir = work_dir.join("config");

    write_claude_code_config(
        &config_dir,
        r#"
[groups]

[groups.g]
FOO = "bar"
"#,
    );

    let output = run_llman(
        &["x", "claude-code", "account", "env", "does-not-exist"],
        work_dir,
        &config_dir,
    );

    assert!(!output.status.success(), "expected failure");
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Configuration group"));
    assert!(stderr.contains("not found"));
}
