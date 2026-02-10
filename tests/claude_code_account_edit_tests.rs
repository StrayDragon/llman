#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn run_llman(args: &[&str], work_dir: &Path, config_dir: &Path, editor_raw: &str) -> Output {
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
        .env_remove("VISUAL")
        .env("EDITOR", editor_raw)
        .current_dir(work_dir)
        .output()
        .expect("Failed to run llman command")
}

fn chmod_executable(path: &Path) {
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

#[test]
fn claude_code_account_edit_creates_config_file() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let config_dir = work_dir.join("config");

    let editor_path = work_dir.join("ok-editor.sh");
    fs::write(&editor_path, "#!/bin/sh\nexit 0\n").expect("write editor");
    chmod_executable(&editor_path);

    let output = run_llman(
        &["x", "claude-code", "account", "edit"],
        work_dir,
        &config_dir,
        editor_path.to_str().expect("editor path"),
    );
    assert!(
        output.status.success(),
        "command failed: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = config_dir.join("claude-code.toml");
    assert!(config_path.exists(), "config file should be created");
    let content = fs::read_to_string(&config_path).expect("read config");
    assert!(content.contains("[groups]"));
}

#[test]
fn claude_code_account_edit_appends_config_path_last() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let config_dir = work_dir.join("config");
    let args_out = work_dir.join("editor-args.txt");

    let editor_path = work_dir.join("capture-editor.sh");
    fs::write(
        &editor_path,
        "#!/bin/sh\nprintf \"%s\\n\" \"$@\" > \"$LLMAN_TEST_EDITOR_OUT\"\nexit 0\n",
    )
    .expect("write editor");
    chmod_executable(&editor_path);

    let editor_raw = format!("{} --wait", editor_path.display());
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--manifest-path",
            manifest_path().to_str().expect("manifest path"),
            "--",
            "--config-dir",
            config_dir.to_str().expect("config dir"),
        ])
        .args(["x", "claude-code", "account", "edit"])
        .env_remove("VISUAL")
        .env("EDITOR", &editor_raw)
        .env("LLMAN_TEST_EDITOR_OUT", &args_out)
        .current_dir(work_dir)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "command failed: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let args_text = fs::read_to_string(&args_out).expect("read args");
    let args: Vec<&str> = args_text.lines().collect();
    assert_eq!(args.first().copied(), Some("--wait"));
    assert_eq!(
        args.last().copied(),
        Some(config_dir.join("claude-code.toml").to_str().unwrap())
    );
}

#[test]
fn claude_code_account_edit_non_zero_exit_fails() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let config_dir = work_dir.join("config");

    let editor_path = work_dir.join("fail-editor.sh");
    fs::write(&editor_path, "#!/bin/sh\nexit 42\n").expect("write editor");
    chmod_executable(&editor_path);

    let output = run_llman(
        &["x", "claude-code", "account", "edit"],
        work_dir,
        &config_dir,
        editor_path.to_str().expect("editor path"),
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Editor exited with status"));
    assert!(stderr.contains("42"));
}
