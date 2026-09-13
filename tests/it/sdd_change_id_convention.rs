//! Integration tests for the machine-readable change id convention
//! (sdd-workflow r29): `change_id.pattern` validate gate, `change_id.template`
//! rendering with `--dry-run`, and the read-only `change next-id` preview.

use crate::common::{TestEnvironment, assert_success, llman_command};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run_llman(args: &[&str], work_dir: &Path, config_dir: &Path) -> Output {
    llman_command(config_dir)
        .args(args)
        .current_dir(work_dir)
        .output()
        .expect("run llman")
}

fn git_commit_all(work_dir: &Path, message: &str) {
    Command::new("git")
        .args(["add", "."])
        .current_dir(work_dir)
        .output()
        .expect("git add");
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@x",
            "commit",
            "-m",
            message,
        ])
        .current_dir(work_dir)
        .output()
        .expect("git commit");
    assert_success(&out);
}

/// Seed an initialized sdd project (git repo, clean tree) with `change_id:`
/// config and a sample spec so validate has content.
fn seed_project(env: &TestEnvironment, pattern: Option<&str>, template: Option<&str>) {
    let work = env.path();
    run_llman(&["sdd", "init", work.to_str().unwrap()], work, work);
    let spec_dir = work.join("llmanspec/specs/sample");
    fs::create_dir_all(&spec_dir).expect("mkdir spec");
    fs::write(
        spec_dir.join("sample.feature"),
        "# language: en\n# capability: sample\n# purpose: sample\n# scope: llmanspec/specs/sample\n\nFeature: sample\n  @req:r1 @human\n  Scenario: r1 rule\n    - System MUST do X.\n",
    )
    .expect("write spec");
    let mut config = "schema: spec-driven\nlocale: en\n".to_string();
    if pattern.is_some() || template.is_some() {
        config.push_str("\nchange_id:\n");
        if let Some(pattern) = pattern {
            config.push_str(&format!("  pattern: '{pattern}'\n"));
        }
        if let Some(template) = template {
            config.push_str(&format!("  template: '{template}'\n"));
        }
    }
    fs::write(work.join("llmanspec/config.yaml"), config).expect("write config");
    run_llman(&["sdd", "init", "--update"], work, work);
    git_commit_all(work, "seed");
}

fn write_change(work: &Path, id: &str) {
    let dir = work.join("llmanspec/changes").join(id);
    fs::create_dir_all(&dir).expect("mkdir change");
    fs::write(
        dir.join("proposal.md"),
        "---\ndepends_on: []\n---\n\n## Why\nw\n\n## What Changes\n- c\n",
    )
    .expect("write proposal");
}

#[test]
fn pattern_violation_errors_active_only() {
    let env = TestEnvironment::new();
    let work = env.path();
    seed_project(&env, Some(r"^c[0-9]+-(add|fix)-[a-z0-9-]+$"), None);
    write_change(work, "Weird Id!");
    // Archive shape also violates the pattern — never back-checked (r124/r29).
    let archived = work.join("llmanspec/changes/archive/2026-09-13-c20-legacy");
    fs::create_dir_all(&archived).expect("mkdir archived");
    fs::write(
        archived.join("proposal.md"),
        "---\ndepends_on: []\n---\n\n## Why\nw\n\n## What Changes\n- c\n",
    )
    .expect("write archived");

    let output = run_llman(
        &[
            "sdd",
            "validate",
            "--all",
            "--strict",
            "--no-check",
            "--no-interactive",
        ],
        work,
        work,
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !output.status.success(),
        "pattern violation must fail: {stdout}{stderr}"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("Weird Id!"),
        "must name the violating id: {combined}"
    );
    assert!(
        !combined.contains("2026-09-13-c20-legacy"),
        "archive shapes are never back-checked: {combined}"
    );
}

#[test]
fn unconfigured_section_keeps_validate_green() {
    let env = TestEnvironment::new();
    let work = env.path();
    seed_project(&env, None, None);
    write_change(work, "any-style-id");

    let output = run_llman(
        &[
            "sdd",
            "validate",
            "--all",
            "--strict",
            "--no-check",
            "--no-interactive",
        ],
        work,
        work,
    );
    assert!(
        output.status.success(),
        "no change_id section = legacy behavior: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn template_dry_run_is_readonly_and_renders_id() {
    let env = TestEnvironment::new();
    let work = env.path();
    seed_project(
        &env,
        None,
        Some("c{{ llman_sdd_unique_id }}-{{ verb }}-{{ subject }}"),
    );

    let output = run_llman(
        &[
            "sdd",
            "change",
            "new",
            "--from",
            "add user login",
            "--dry-run",
        ],
        work,
        work,
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "dry-run must succeed: {stdout}");
    assert!(
        stdout.contains("c1-add-user-login"),
        "rendered id: {stdout}"
    );
    assert!(
        !work.join("llmanspec/changes/c1-add-user-login").exists(),
        "dry-run must not create the change dir"
    );

    // Without --dry-run the same description lands at the rendered id.
    let output = run_llman(
        &["sdd", "change", "new", "--from", "add user login"],
        work,
        work,
    );
    assert!(output.status.success());
    assert!(
        work.join("llmanspec/changes/c1-add-user-login/proposal.md")
            .exists()
    );
}

#[test]
fn next_id_previews_whole_tree_max_readonly() {
    let env = TestEnvironment::new();
    let work = env.path();
    seed_project(&env, None, None);
    // Issue #20 shape: active max < nested delayed max.
    fs::create_dir_all(work.join("llmanspec/changes/c10-active")).expect("mkdir active");
    fs::create_dir_all(work.join("llmanspec/delayed-changes/tools/c2620-tool-x"))
        .expect("mkdir delayed");

    let output = run_llman(&["sdd", "change", "next-id"], work, work);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("2620"), "whole-tree max: {stdout}");
    assert!(stdout.contains("2621"), "next free number: {stdout}");
    assert!(
        !work.join("llmanspec/changes/c2621-anything").exists(),
        "next-id must not create anything"
    );

    let json = run_llman(&["sdd", "change", "next-id", "--json"], work, work);
    let stdout = String::from_utf8_lossy(&json.stdout);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(value["nextNumber"], 2621);
}
