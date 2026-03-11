#![cfg(unix)]

mod common;

use common::{
    assert_success, llman_command_with_editor, run_llman_with_editor, write_executable_script,
};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[test]
fn claude_code_account_edit_creates_config_file() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let config_dir = work_dir.join("config");

    let editor_path = work_dir.join("ok-editor.sh");
    write_executable_script(&editor_path, "#!/bin/sh\nexit 0\n");

    let output = run_llman_with_editor(
        &["x", "claude-code", "account", "edit"],
        work_dir,
        &config_dir,
        editor_path.to_str().expect("editor path"),
    );
    assert_success(&output);

    let config_path = config_dir.join("claude-code.toml");
    assert!(config_path.exists(), "config file should be created");
    let content = fs::read_to_string(&config_path).expect("read config");
    assert!(content.contains("[groups]"));
}

#[test]
fn claude_code_account_edit_creates_user_only_config_permissions() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let config_dir = work_dir.join("config");

    let editor_path = work_dir.join("ok-editor.sh");
    write_executable_script(&editor_path, "#!/bin/sh\nexit 0\n");

    let output = run_llman_with_editor(
        &["x", "claude-code", "account", "edit"],
        work_dir,
        &config_dir,
        editor_path.to_str().expect("editor path"),
    );
    assert_success(&output);

    let mode = fs::metadata(config_dir.join("claude-code.toml"))
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn claude_code_account_edit_appends_config_path_last() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let config_dir = work_dir.join("config");
    let args_out = work_dir.join("editor-args.txt");

    let editor_path = work_dir.join("capture-editor.sh");
    write_executable_script(
        &editor_path,
        "#!/bin/sh\nprintf \"%s\\n\" \"$@\" > \"$LLMAN_TEST_EDITOR_OUT\"\nexit 0\n",
    );

    let editor_raw = format!("{} --wait", editor_path.display());
    let output = llman_command_with_editor(&config_dir, &editor_raw)
        .args(["x", "claude-code", "account", "edit"])
        .env("LLMAN_TEST_EDITOR_OUT", &args_out)
        .current_dir(work_dir)
        .output()
        .expect("run");

    assert_success(&output);

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
    write_executable_script(&editor_path, "#!/bin/sh\nexit 42\n");

    let output = run_llman_with_editor(
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
