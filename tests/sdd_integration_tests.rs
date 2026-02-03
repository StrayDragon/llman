mod common;

use common::TestEnvironment;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
        .env("LLMANSPEC_BASE_REF", "HEAD")
        .output()
        .expect("Failed to run llman command")
}

fn assert_success(output: &Output) {
    if output.status.success() {
        return;
    }
    panic!(
        "Command failed: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_commit_all(work_dir: &Path, message: &str) {
    let add_output = Command::new("git")
        .args(["add", "."])
        .current_dir(work_dir)
        .output()
        .expect("Failed to git add");
    assert_success(&add_output);

    let commit_output = Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            message,
        ])
        .current_dir(work_dir)
        .output()
        .expect("Failed to git commit");
    assert_success(&commit_output);
}

#[test]
fn test_sdd_init_and_list_specs_json() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let agents_path = work_dir.join("llmanspec").join("AGENTS.md");
    assert!(agents_path.exists());

    let list_output = run_llman(&["sdd", "list", "--specs", "--json"], work_dir, work_dir);
    assert_success(&list_output);

    let stdout = String::from_utf8_lossy(&list_output.stdout);
    let parsed: Value = serde_json::from_str(stdout.trim()).expect("spec list json");
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 0);
}

#[test]
fn test_sdd_show_validate_archive_flow() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let spec_dir = llmanspec_dir.join("specs").join("sample");
    fs::create_dir_all(&spec_dir).expect("create spec dir");
    let spec_content = r#"---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - tests/sdd_integration_tests.rs
---

# Sample Specification

## Purpose
Describe sample behavior.

## Requirements
### Requirement: Existing behavior
System MUST preserve existing behavior.

#### Scenario: baseline
- **WHEN** running the sample
- **THEN** behavior is preserved
"#;
    fs::write(spec_dir.join("spec.md"), spec_content).expect("write spec");

    let change_dir = llmanspec_dir.join("changes").join("add-sample");
    let change_specs_dir = change_dir.join("specs").join("sample");
    fs::create_dir_all(&change_specs_dir).expect("create change spec dir");
    let proposal = r#"## Why
Need a sample change.

## What Changes
- Add a requirement to sample spec.
"#;
    fs::write(change_dir.join("proposal.md"), proposal).expect("write proposal");
    fs::write(
        change_dir.join("tasks.md"),
        "## 1. Done\n- [x] 1.1 Completed\n",
    )
    .expect("write tasks");
    let delta_spec = r#"## ADDED Requirements
### Requirement: Added behavior
System MUST support the added behavior.

#### Scenario: added
- **WHEN** a new action is taken
- **THEN** the new behavior happens
"#;
    fs::write(change_specs_dir.join("spec.md"), delta_spec).expect("write delta spec");

    git_commit_all(work_dir, "init sdd sample");

    let show_output = run_llman(
        &["sdd", "show", "sample", "--type", "spec", "--json"],
        work_dir,
        work_dir,
    );
    assert_success(&show_output);
    let show_json: Value = serde_json::from_slice(&show_output.stdout).expect("show spec json");
    assert_eq!(show_json["id"], "sample");
    assert_eq!(show_json["requirementCount"], 1);

    let validate_output = run_llman(
        &[
            "sdd",
            "validate",
            "sample",
            "--type",
            "spec",
            "--strict",
            "--no-interactive",
            "--json",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&validate_output);
    let validate_json: Value =
        serde_json::from_slice(&validate_output.stdout).expect("validate json");
    assert_eq!(validate_json["items"][0]["valid"], true);

    let archive_output = run_llman(&["sdd", "archive", "add-sample"], work_dir, work_dir);
    assert_success(&archive_output);

    let archive_root = llmanspec_dir.join("changes").join("archive");
    let entries: Vec<_> = fs::read_dir(&archive_root)
        .expect("read archive dir")
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(entries.len(), 1);
    let archive_name = entries[0].file_name().to_string_lossy().to_string();
    assert!(archive_name.ends_with("-add-sample"));

    let updated_spec = fs::read_to_string(spec_dir.join("spec.md")).expect("read updated spec");
    assert!(updated_spec.contains("Requirement: Existing behavior"));
    assert!(updated_spec.contains("Requirement: Added behavior"));
}

#[test]
fn test_sdd_archive_help_hides_force() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let help_output = run_llman(&["sdd", "archive", "--help"], work_dir, work_dir);
    assert_success(&help_output);

    let stdout = String::from_utf8_lossy(&help_output.stdout);
    let stderr = String::from_utf8_lossy(&help_output.stderr);
    assert!(!stdout.contains("--force"));
    assert!(!stderr.contains("--force"));
}

#[test]
fn test_sdd_list_changes_json_reports_status() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("add-sample");
    fs::create_dir_all(&change_dir).expect("create change dir");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nNeed a sample change.\n\n## What Changes\n- Add sample.\n",
    )
    .expect("write proposal");
    fs::write(
        change_dir.join("tasks.md"),
        "## 1. Tasks\n- [x] 1.1 Done\n- [ ] 1.2 Pending\n",
    )
    .expect("write tasks");

    let list_output = run_llman(&["sdd", "list", "--json"], work_dir, work_dir);
    assert_success(&list_output);

    let parsed: Value =
        serde_json::from_slice(&list_output.stdout).expect("parse list changes json");
    let changes = parsed["changes"].as_array().expect("changes array");
    assert_eq!(changes.len(), 1);
    let change = &changes[0];
    assert_eq!(change["name"], "add-sample");
    assert_eq!(change["completedTasks"], 1);
    assert_eq!(change["totalTasks"], 2);
    assert_eq!(change["status"], "in-progress");
}

#[test]
fn test_sdd_show_change_json_uses_delta_specs() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("add-sample");
    let change_specs_dir = change_dir.join("specs").join("sample");
    fs::create_dir_all(&change_specs_dir).expect("create change spec dir");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nNeed a sample change.\n\n## What Changes\n- Add requirement.\n",
    )
    .expect("write proposal");
    let delta_spec = r#"## ADDED Requirements
### Requirement: Added behavior
System MUST support the added behavior.

#### Scenario: added
- **WHEN** a new action is taken
- **THEN** the new behavior happens
"#;
    fs::write(change_specs_dir.join("spec.md"), delta_spec).expect("write delta spec");

    let show_output = run_llman(
        &["sdd", "show", "add-sample", "--type", "change", "--json"],
        work_dir,
        work_dir,
    );
    assert_success(&show_output);

    let show_json: Value = serde_json::from_slice(&show_output.stdout).expect("show change json");
    assert_eq!(show_json["id"], "add-sample");
    assert_eq!(show_json["deltaCount"], 1);
    assert_eq!(show_json["deltas"][0]["operation"], "ADDED");
    assert_eq!(show_json["deltas"][0]["spec"], "sample");
}

#[test]
fn test_sdd_validate_change_json_succeeds() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("add-sample");
    let change_specs_dir = change_dir.join("specs").join("sample");
    fs::create_dir_all(&change_specs_dir).expect("create change spec dir");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nNeed a sample change.\n\n## What Changes\n- Add requirement.\n",
    )
    .expect("write proposal");
    let delta_spec = r#"## ADDED Requirements
### Requirement: Added behavior
System MUST support the added behavior.

#### Scenario: added
- **WHEN** a new action is taken
- **THEN** the new behavior happens
"#;
    fs::write(change_specs_dir.join("spec.md"), delta_spec).expect("write delta spec");

    let validate_output = run_llman(
        &[
            "sdd",
            "validate",
            "add-sample",
            "--type",
            "change",
            "--strict",
            "--no-interactive",
            "--json",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&validate_output);

    let validate_json: Value =
        serde_json::from_slice(&validate_output.stdout).expect("validate change json");
    assert_eq!(validate_json["items"][0]["type"], "change");
    assert_eq!(validate_json["items"][0]["valid"], true);
}

#[test]
fn test_sdd_update_recreates_templates() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let proposal_path = work_dir
        .join("llmanspec")
        .join("templates")
        .join("spec-driven")
        .join("proposal.md");
    fs::remove_file(&proposal_path).expect("remove proposal template");
    assert!(!proposal_path.exists());

    let update_output = run_llman(&["sdd", "update"], work_dir, work_dir);
    assert_success(&update_output);

    assert!(proposal_path.exists());
    let content = fs::read_to_string(&proposal_path).expect("read proposal template");
    assert!(content.contains("## Why"));
}

#[test]
fn test_sdd_update_skills_writes_codex_templates() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let output_dir = work_dir.join(".codex/skills");
    let update_output = run_llman(
        &[
            "sdd",
            "update-skills",
            "--tool",
            "codex",
            "--no-interactive",
            "--path",
            ".codex/skills",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&update_output);

    let skill_path = output_dir.join("llman-sdd-onboard").join("SKILL.md");
    assert!(skill_path.exists());
}
