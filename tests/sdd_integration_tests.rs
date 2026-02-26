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

```ison
{
  "version": "1.0.0",
  "kind": "llman.sdd.spec",
  "name": "sample",
  "purpose": "Describe sample behavior.",
  "requirements": [
    {
      "req_id": "existing-behavior",
      "title": "Existing behavior",
      "statement": "System MUST preserve existing behavior.",
      "scenarios": [
        {
          "id": "baseline",
          "text": "- **WHEN** running the sample\n- **THEN** behavior is preserved"
        }
      ]
    }
  ]
}
```
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
    let delta_spec = r#"```ison
{
  "version": "1.0.0",
  "kind": "llman.sdd.delta",
  "ops": [
    {
      "op": "add_requirement",
      "req_id": "added-behavior",
      "title": "Added behavior",
      "statement": "System MUST support the added behavior.",
      "scenarios": [
        {
          "id": "added",
          "text": "- **WHEN** a new action is taken\n- **THEN** the new behavior happens"
        }
      ]
    }
  ]
}
```
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
    assert!(updated_spec.contains("\"title\": \"Existing behavior\""));
    assert!(updated_spec.contains("\"title\": \"Added behavior\""));
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
    assert!(stdout.contains("run"));
    assert!(stdout.contains("freeze"));
    assert!(stdout.contains("thaw"));
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
    let delta_spec = r#"```ison
{
  "version": "1.0.0",
  "kind": "llman.sdd.delta",
  "ops": [
    {
      "op": "add_requirement",
      "req_id": "added-behavior",
      "title": "Added behavior",
      "statement": "System MUST support the added behavior.",
      "scenarios": [
        {
          "id": "added",
          "text": "- **WHEN** a new action is taken\n- **THEN** the new behavior happens"
        }
      ]
    }
  ]
}
```
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
    let delta_spec = r#"```ison
{
  "version": "1.0.0",
  "kind": "llman.sdd.delta",
  "ops": [
    {
      "op": "add_requirement",
      "req_id": "added-behavior",
      "title": "Added behavior",
      "statement": "System MUST support the added behavior.",
      "scenarios": [
        {
          "id": "added",
          "text": "- **WHEN** a new action is taken\n- **THEN** the new behavior happens"
        }
      ]
    }
  ]
}
```
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
fn test_sdd_validate_ab_report_json_orders_safety_priority_metrics() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let output = run_llman(
        &[
            "sdd",
            "validate",
            "--ab-report",
            "--json",
            "--no-interactive",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&output);

    let parsed: Value = serde_json::from_slice(&output.stdout).expect("ab report json");
    let metric_order = parsed["metricOrder"].as_array().expect("metric order");
    assert_eq!(metric_order[0], "quality");
    assert_eq!(metric_order[1], "safety");
    assert_eq!(metric_order[2], "token_estimate");
    assert_eq!(metric_order[3], "latency_ms");

    let styles = parsed["styles"].as_array().expect("styles");
    assert_eq!(styles.len(), 2);
    let style_names: Vec<String> = styles
        .iter()
        .filter_map(|s| s["style"].as_str().map(|v| v.to_string()))
        .collect();
    assert!(style_names.contains(&"new".to_string()));
    assert!(style_names.contains(&"legacy".to_string()));

    let priority = parsed["comparison"]["priority"]
        .as_array()
        .expect("priority array");
    assert_eq!(priority[0], "safety");
    assert_eq!(priority[1], "quality");
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

    let template_path = work_dir
        .join("llmanspec")
        .join("templates")
        .join("spec-driven")
        .join("new.md");
    fs::remove_file(&template_path).expect("remove template");
    assert!(!template_path.exists());

    let update_output = run_llman(&["sdd", "update"], work_dir, work_dir);
    assert_success(&update_output);

    assert!(template_path.exists());
    let content = fs::read_to_string(&template_path).expect("read template");
    assert!(content.contains("/llman-sdd:new"));
}

#[test]
fn test_sdd_update_legacy_style_routes_legacy_templates() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let template_path = work_dir
        .join("llmanspec")
        .join("templates")
        .join("spec-driven")
        .join("new.md");

    let before = fs::read_to_string(&template_path).expect("read default template");
    assert!(!before.contains("legacy-track"));

    let update_output = run_llman(&["sdd", "update", "--style", "legacy"], work_dir, work_dir);
    assert_success(&update_output);

    let after = fs::read_to_string(&template_path).expect("read legacy template");
    assert!(
        after.contains("legacy-track"),
        "legacy style update should render legacy marker"
    );
}

#[test]
fn test_sdd_update_skills_writes_codex_skills_without_workflow_prompts() {
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
    let compact_skill_path = output_dir.join("llman-sdd-specs-compact").join("SKILL.md");
    assert!(compact_skill_path.exists());
    let compact_skill = fs::read_to_string(&compact_skill_path).expect("read specs compact skill");
    assert!(!compact_skill.contains("legacy-track"));
    assert!(!compact_skill.contains("先调用某外部技能"));
    assert!(
        !compact_skill
            .to_lowercase()
            .contains("ison-prompt-optimizer-zh")
    );
    let archive_skill_path = output_dir.join("llman-sdd-archive").join("SKILL.md");
    assert!(archive_skill_path.exists());
    let archive_skill = fs::read_to_string(&archive_skill_path).expect("read archive skill");
    assert!(archive_skill.contains("llman sdd archive freeze"));
    assert!(archive_skill.contains("llman sdd archive thaw"));
    let explore_skill_path = output_dir.join("llman-sdd-explore").join("SKILL.md");
    assert!(explore_skill_path.exists());
    let explore_skill = fs::read_to_string(&explore_skill_path).expect("read explore skill");
    assert!(explore_skill.contains("Future-to-Execution Planning"));
    assert!(explore_skill.contains("llmanspec/changes/<id>/future.md"));

    let codex_prompts = work_dir.join(".codex/prompts");
    assert!(!codex_prompts.exists());
}

#[test]
fn test_sdd_update_skills_legacy_style_routes_legacy_templates() {
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
            "--style",
            "legacy",
            "--path",
            ".codex/skills",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&update_output);

    let compact_skill_path = output_dir.join("llman-sdd-specs-compact").join("SKILL.md");
    let compact_skill = fs::read_to_string(&compact_skill_path).expect("read legacy compact skill");
    assert!(
        compact_skill.contains("legacy-track"),
        "legacy style should render legacy unit marker"
    );
}

#[test]
fn test_sdd_update_skills_new_style_uses_markdown_override() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let override_path = work_dir.join("templates/sdd/en/skills/llman-sdd-onboard.md");
    fs::create_dir_all(override_path.parent().expect("parent")).expect("mkdir override");
    fs::write(
        &override_path,
        r#"---
name: "llman-sdd-onboard"
description: "markdown override"
metadata:
  llman-template-version: 1
---

# Markdown Override
MARKDOWN SOURCE MARKER

## Context
- from markdown
## Goal
- from markdown
## Constraints
- from markdown
## Workflow
- from markdown
## Decision Policy
- from markdown
## Output Contract
- from markdown
## Ethics Governance
- `ethics.risk_level`: x
- `ethics.prohibited_actions`: x
- `ethics.required_evidence`: x
- `ethics.refusal_contract`: x
- `ethics.escalation_policy`: x
"#,
    )
    .expect("write markdown override");

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

    let skill = fs::read_to_string(output_dir.join("llman-sdd-onboard").join("SKILL.md"))
        .expect("read skill");
    assert!(skill.contains("MARKDOWN SOURCE MARKER"));
}

#[test]
fn test_sdd_update_skills_new_style_fails_when_override_missing_ethics_key() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let override_path = work_dir.join("templates/sdd/en/skills/llman-sdd-onboard.md");
    fs::create_dir_all(override_path.parent().expect("parent")).expect("mkdir override");
    fs::write(
        &override_path,
        r#"---
name: "llman-sdd-onboard"
description: "missing ethics key"
metadata:
  llman-template-version: 1
---

## Context
- test
## Goal
- test
## Constraints
- test
## Workflow
- test
## Decision Policy
- test
## Output Contract
- test
## Ethics Governance
- `ethics.risk_level`: x
- `ethics.prohibited_actions`: x
- `ethics.required_evidence`: x
- `ethics.refusal_contract`: x
"#,
    )
    .expect("write invalid override");

    let output = run_llman(
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
    assert!(
        !output.status.success(),
        "missing ethics key in override should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ethics.escalation_policy"),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn test_sdd_archive_freeze_and_thaw_single_file_flow() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let archive_root = work_dir.join("llmanspec").join("changes").join("archive");
    let archived_a = archive_root.join("2026-01-01-alpha");
    let archived_b = archive_root.join("2026-01-02-beta");
    fs::create_dir_all(&archived_a).expect("create archived a");
    fs::create_dir_all(&archived_b).expect("create archived b");
    fs::write(archived_a.join("a.txt"), "alpha").expect("write a");
    fs::write(archived_b.join("b.txt"), "beta").expect("write b");

    let freeze_output = run_llman(&["sdd", "archive", "freeze"], work_dir, work_dir);
    assert_success(&freeze_output);

    let freeze_file = archive_root.join("freezed_changes.7z.archived");
    assert!(freeze_file.exists());
    assert!(!archived_a.exists());
    assert!(!archived_b.exists());

    let thaw_output = run_llman(&["sdd", "archive", "thaw"], work_dir, work_dir);
    assert_success(&thaw_output);
    assert!(archive_root.join(".thawed/2026-01-01-alpha/a.txt").exists());
    assert!(archive_root.join(".thawed/2026-01-02-beta/b.txt").exists());
}

#[test]
fn test_sdd_archive_thaw_supports_change_filter_and_dest() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let archive_root = work_dir.join("llmanspec").join("changes").join("archive");
    let archived_a = archive_root.join("2026-01-03-alpha");
    let archived_b = archive_root.join("2026-01-04-beta");
    fs::create_dir_all(&archived_a).expect("create archived a");
    fs::create_dir_all(&archived_b).expect("create archived b");
    fs::write(archived_a.join("a.txt"), "alpha").expect("write a");
    fs::write(archived_b.join("b.txt"), "beta").expect("write b");

    let freeze_output = run_llman(&["sdd", "archive", "freeze"], work_dir, work_dir);
    assert_success(&freeze_output);

    let custom_dest = work_dir.join("restore-target");
    let thaw_output = run_llman(
        &[
            "sdd",
            "archive",
            "thaw",
            "--change",
            "2026-01-03-alpha",
            "--dest",
            custom_dest.to_str().expect("dest"),
        ],
        work_dir,
        work_dir,
    );
    assert_success(&thaw_output);

    assert!(custom_dest.join("2026-01-03-alpha/a.txt").exists());
    assert!(!custom_dest.join("2026-01-04-beta").exists());
}

#[test]
fn test_sdd_archive_freeze_dry_run_does_not_write() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let archive_root = work_dir.join("llmanspec").join("changes").join("archive");
    let archived = archive_root.join("2026-01-05-alpha");
    fs::create_dir_all(&archived).expect("create archived dir");
    fs::write(archived.join("a.txt"), "alpha").expect("write file");

    let dry_run_output = run_llman(
        &["sdd", "archive", "freeze", "--dry-run"],
        work_dir,
        work_dir,
    );
    assert_success(&dry_run_output);

    assert!(archived.exists());
    assert!(!archive_root.join("freezed_changes.7z.archived").exists());
}

#[test]
fn test_sdd_archive_freeze_failure_keeps_source_dirs() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let archive_root = work_dir.join("llmanspec").join("changes").join("archive");
    let archived = archive_root.join("2026-01-06-alpha");
    fs::create_dir_all(&archived).expect("create archived dir");
    fs::write(archived.join("a.txt"), "alpha").expect("write file");

    let freeze_file_dir = archive_root.join("freezed_changes.7z.archived");
    fs::create_dir_all(&freeze_file_dir).expect("create conflicting freeze path");

    let freeze_output = run_llman(&["sdd", "archive", "freeze"], work_dir, work_dir);
    assert!(!freeze_output.status.success());
    assert!(archived.exists());
}

#[test]
fn test_sdd_archive_freeze_second_run_preserves_first_run_content() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let archive_root = work_dir.join("llmanspec").join("changes").join("archive");
    let first = archive_root.join("2026-01-07-first");
    fs::create_dir_all(&first).expect("create first dir");
    fs::write(first.join("one.txt"), "one").expect("write first file");
    assert_success(&run_llman(
        &["sdd", "archive", "freeze"],
        work_dir,
        work_dir,
    ));

    let second = archive_root.join("2026-01-08-second");
    fs::create_dir_all(&second).expect("create second dir");
    fs::write(second.join("two.txt"), "two").expect("write second file");
    assert_success(&run_llman(
        &["sdd", "archive", "freeze"],
        work_dir,
        work_dir,
    ));

    let thaw_dest = work_dir.join("thaw-all");
    assert_success(&run_llman(
        &[
            "sdd",
            "archive",
            "thaw",
            "--dest",
            thaw_dest.to_str().expect("dest"),
        ],
        work_dir,
        work_dir,
    ));

    assert!(thaw_dest.join("2026-01-07-first/one.txt").exists());
    assert!(thaw_dest.join("2026-01-08-second/two.txt").exists());
}

#[test]
fn test_sdd_validate_change_without_future_md_still_succeeds() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("no-future");
    let change_specs_dir = change_dir.join("specs").join("sample");
    fs::create_dir_all(&change_specs_dir).expect("create change spec dir");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nNeed a sample change.\n\n## What Changes\n- Add requirement.\n",
    )
    .expect("write proposal");
    fs::write(change_dir.join("tasks.md"), "## 1. Tasks\n- [ ] 1.1 Do\n").expect("write tasks");
    let delta_spec = r#"```ison
{
  "version": "1.0.0",
  "kind": "llman.sdd.delta",
  "ops": [
    {
      "op": "add_requirement",
      "req_id": "added-behavior",
      "title": "Added behavior",
      "statement": "System MUST support the added behavior.",
      "scenarios": [
        {
          "id": "added",
          "text": "- **WHEN** a new action is taken\n- **THEN** the new behavior happens"
        }
      ]
    }
  ]
}
```
"#;
    fs::write(change_specs_dir.join("spec.md"), delta_spec).expect("write delta spec");

    let validate_output = run_llman(
        &[
            "sdd",
            "validate",
            "no-future",
            "--type",
            "change",
            "--strict",
            "--no-interactive",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&validate_output);
}

#[test]
fn test_sdd_import_requires_style_flag() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let output = run_llman(&["sdd", "import"], work_dir, work_dir);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--style"));
}

#[test]
fn test_sdd_import_rejects_unsupported_style() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let output = run_llman(&["sdd", "import", "--style", "unknown"], work_dir, work_dir);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("openspec"));
}

#[test]
fn test_sdd_export_non_interactive_dry_run_only() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    fs::create_dir_all(work_dir.join("llmanspec/specs/sample")).expect("create source specs dir");
    fs::write(
        work_dir.join("llmanspec/specs/sample/spec.md"),
        "---\nllman_spec_valid_scope:\n  - src\nllman_spec_valid_commands:\n  - just test\nllman_spec_evidence:\n  - local\n---\n\n# Sample\n",
    )
    .expect("write source spec");

    let output = run_llman(
        &["sdd", "export", "--style", "openspec"],
        work_dir,
        work_dir,
    );
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Dry-run export plan"));
    assert!(stderr.contains("Non-interactive mode"));
    assert!(!work_dir.join("openspec/specs/sample/spec.md").exists());
}

#[test]
fn test_sdd_help_shows_import_export_and_hides_migrate() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let output = run_llman(&["sdd", "--help"], work_dir, work_dir);
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("import"));
    assert!(stdout.contains("export"));
    assert!(!stdout.contains("migrate"));
}

#[test]
fn test_sdd_migrate_to_ison_dry_run_does_not_write() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    fs::create_dir_all(work_dir.join("llmanspec/specs/sample")).expect("create spec dir");
    let source = r#"## Purpose
Describe sample behavior.

## Requirements
### Requirement: Existing behavior
System MUST preserve existing behavior.

#### Scenario: baseline
- **WHEN** running the sample
- **THEN** behavior is preserved
"#;
    let target = work_dir.join("llmanspec/specs/sample/spec.md");
    fs::write(&target, source).expect("write source spec");

    let output = run_llman(
        &["sdd", "migrate", "--to-ison", "--dry-run"],
        work_dir,
        work_dir,
    );
    assert_success(&output);

    let after = fs::read_to_string(&target).expect("read target");
    assert_eq!(after, source);
}

#[test]
fn test_sdd_migrate_to_ison_writes_spec_and_delta() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    fs::create_dir_all(work_dir.join("llmanspec/specs/sample")).expect("create spec dir");
    fs::write(
        work_dir.join("llmanspec/specs/sample/spec.md"),
        r#"---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - tests
---

## Purpose
Describe sample behavior.

## Requirements
### Requirement: Existing behavior
System MUST preserve existing behavior.

#### Scenario: baseline
- **WHEN** running the sample
- **THEN** behavior is preserved
"#,
    )
    .expect("write source spec");

    fs::create_dir_all(work_dir.join("llmanspec/changes/add-sample/specs/sample"))
        .expect("create delta dir");
    fs::write(
        work_dir.join("llmanspec/changes/add-sample/specs/sample/spec.md"),
        r#"## ADDED Requirements
### Requirement: Added behavior
System MUST support the added behavior.

#### Scenario: added
- **WHEN** a new action is taken
- **THEN** the new behavior happens
"#,
    )
    .expect("write source delta");

    let output = run_llman(&["sdd", "migrate", "--to-ison"], work_dir, work_dir);
    assert_success(&output);

    let spec_after = fs::read_to_string(work_dir.join("llmanspec/specs/sample/spec.md"))
        .expect("read migrated spec");
    assert!(spec_after.contains("```ison"));
    assert!(spec_after.contains("\"kind\": \"llman.sdd.spec\""));

    let delta_after =
        fs::read_to_string(work_dir.join("llmanspec/changes/add-sample/specs/sample/spec.md"))
            .expect("read migrated delta");
    assert!(delta_after.contains("```ison"));
    assert!(delta_after.contains("\"kind\": \"llman.sdd.delta\""));
    assert!(delta_after.contains("\"op\": \"add_requirement\""));
}
