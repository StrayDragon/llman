mod common;

use common::{TestEnvironment, assert_success, git_head, llman_command};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run_llman(args: &[&str], work_dir: &Path, config_dir: &Path) -> Output {
    let mut cmd = llman_command(config_dir);
    cmd.args(args).current_dir(work_dir);
    if let Some(base_ref) = git_head(work_dir) {
        cmd.env("LLMANSPEC_BASE_REF", base_ref);
    }
    cmd.output().expect("Failed to run llman command")
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

fn assert_no_disallowed_prompt_markers(path: &Path, content: &str) {
    const DISALLOWED: &[&str] = &[
        "Options:",
        "<option",
        "What would you like to do?",
        "/llman-sdd:",
        "{{ unit(",
    ];
    for snippet in DISALLOWED {
        assert!(
            !content.contains(snippet),
            "generated content contains disallowed snippet {snippet:?}: {}",
            path.display()
        );
    }
}

fn author_sample_spec(work_dir: &Path) {
    assert_success(&run_llman(
        &["sdd", "spec", "skeleton", "sample"],
        work_dir,
        work_dir,
    ));
    assert_success(&run_llman(
        &[
            "sdd",
            "spec",
            "add-requirement",
            "sample",
            "r1",
            "--title",
            "R1",
            "--statement",
            "System MUST support R1.",
        ],
        work_dir,
        work_dir,
    ));
    assert_success(&run_llman(
        &[
            "sdd",
            "spec",
            "add-scenario",
            "sample",
            "r1",
            "happy",
            "--when",
            "a trigger happens",
            "--then",
            "the outcome is observed",
        ],
        work_dir,
        work_dir,
    ));
}

fn author_sample_change(work_dir: &Path, change_id: &str) {
    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join(change_id);
    fs::create_dir_all(&change_dir).expect("create change dir");
    let proposal = "## Why\nNeed a sample change.\n\n## What Changes\n- Add requirement.\n";
    fs::write(change_dir.join("proposal.md"), proposal).expect("write proposal");

    assert_success(&run_llman(
        &["sdd", "delta", "skeleton", change_id, "sample"],
        work_dir,
        work_dir,
    ));
    assert_success(&run_llman(
        &[
            "sdd",
            "delta",
            "add-req",
            change_id,
            "sample",
            "r2",
            "--title",
            "R2",
            "--statement",
            "System MUST support R2.",
        ],
        work_dir,
        work_dir,
    ));
    assert_success(&run_llman(
        &[
            "sdd",
            "delta",
            "add-scenario",
            change_id,
            "sample",
            "r2",
            "happy",
            "--when",
            "r2 is used",
            "--then",
            "it works",
        ],
        work_dir,
        work_dir,
    ));
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

    let config_path = work_dir.join("llmanspec").join("config.yaml");
    assert!(config_path.exists());

    let list_output = run_llman(&["sdd", "list", "--specs", "--json"], work_dir, work_dir);
    assert_success(&list_output);

    let stdout = String::from_utf8_lossy(&list_output.stdout);
    let parsed: Value = serde_json::from_str(stdout.trim()).expect("spec list json");
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 0);
}

#[test]
fn test_sdd_init_writes_schema_spec_driven() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let config_path = work_dir.join("llmanspec").join("config.yaml");
    let config = fs::read_to_string(&config_path).expect("read config");
    assert!(
        config.contains("schema: spec-driven"),
        "expected init config to include schema: spec-driven; got:\n{config}"
    );
    assert!(
        !config.contains("spec_style:"),
        "config must not contain spec_style field; got:\n{config}"
    );
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
    let spec_content = "kind: llman.sdd.spec\nname: sample\npurpose: \"Describe sample behavior.\"\nvalid_scope[1]: src\nrequirements[1]{req_id,title,statement}:\n  existing,Existing behavior,System MUST preserve existing behavior.\nscenarios[1]{req_id,id,given,when,then}:\n  existing,baseline,,running the sample,behavior is preserved\n";
    fs::write(spec_dir.join("spec.toon"), spec_content).expect("write spec");

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
    let delta_spec = "kind: llman.sdd.delta\nops[1]{op,req_id,title,statement,from,to,name}:\n  add_requirement,added,Added behavior,System MUST support the added behavior.,null,null,null\nop_scenarios[1]{req_id,id,given,when,then}:\n  added,added,,a new action is taken,the new behavior happens\n";
    fs::write(change_specs_dir.join("spec.toon"), delta_spec).expect("write delta spec");

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

    let archive_output = run_llman(&["sdd", "archive", "run", "add-sample"], work_dir, work_dir);
    assert_success(&archive_output);

    let archive_root = llmanspec_dir.join("changes").join("archive");
    let entries: Vec<_> = fs::read_dir(&archive_root)
        .expect("read archive dir")
        .filter_map(|entry| entry.ok())
        .filter(|e| e.file_name() != ".gitkeep")
        .collect();
    assert_eq!(entries.len(), 1);
    let archive_name = entries[0].file_name().to_string_lossy().to_string();
    assert!(archive_name.ends_with("-add-sample"));

    let updated_spec = fs::read_to_string(spec_dir.join("spec.toon")).expect("read updated spec");
    assert!(updated_spec.contains("requirements["));
    assert!(updated_spec.contains("existing"));
    assert!(updated_spec.contains("added"));
    assert!(updated_spec.contains("Existing behavior"));
    assert!(updated_spec.contains("Added behavior"));
}

#[test]
fn test_sdd_archive_flow_works_in_toon_project() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    assert_success(&run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    ));

    // Seed existing main spec.
    author_sample_spec(work_dir);

    // Seed change deltas.
    author_sample_change(work_dir, "add-sample");
    git_commit_all(work_dir, "seed toon spec and change");

    let validate_spec = run_llman(
        &[
            "sdd",
            "validate",
            "sample",
            "--type",
            "spec",
            "--strict",
            "--no-interactive",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&validate_spec);

    let archive_output = run_llman(&["sdd", "archive", "run", "add-sample"], work_dir, work_dir);
    assert_success(&archive_output);

    let updated = fs::read_to_string(work_dir.join("llmanspec/specs/sample/spec.toon"))
        .expect("read updated spec");
    assert!(updated.contains("valid_scope"));
    assert!(updated.contains("r2"));
    assert!(updated.contains("System MUST support R2."));
}

#[test]
fn test_sdd_single_toon_block_show_and_validate_spec() {
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
    let spec_content = "kind: llman.sdd.spec\nname: sample\npurpose: \"Describe sample behavior.\"\nvalid_scope[1]: src\nrequirements[1]{req_id,title,statement}:\n  r1,First requirement,System MUST do the first thing.\nscenarios[1]{req_id,id,given,when,then}:\n  r1,s1,,doing the first thing,it works\n";
    fs::write(spec_dir.join("spec.toon"), spec_content).expect("write spec");

    git_commit_all(work_dir, "add toon spec");

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
}

#[test]
fn test_sdd_given_mapping_to_raw_text_is_deterministic() {
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
    let spec_content = "kind: llman.sdd.spec\nname: sample\npurpose: \"Describe sample behavior.\"\nvalid_scope[1]: src\nrequirements[1]{req_id,title,statement}:\n  r1,First requirement,System MUST do the first thing.\nscenarios[2]{req_id,id,given,when,then}:\n  r1,s1,,run the flow,it works\n  r1,s2,user exists,run the flow,it works\n";
    fs::write(spec_dir.join("spec.toon"), spec_content).expect("write spec");

    let show_output = run_llman(
        &["sdd", "show", "sample", "--type", "spec", "--json"],
        work_dir,
        work_dir,
    );
    assert_success(&show_output);

    let show_json: Value = serde_json::from_slice(&show_output.stdout).expect("show spec json");
    let scenarios = show_json["requirements"][0]["scenarios"]
        .as_array()
        .expect("scenarios array");
    let raw_1 = scenarios[0]["rawText"].as_str().expect("rawText 1");
    let raw_2 = scenarios[1]["rawText"].as_str().expect("rawText 2");
    assert_eq!(raw_1, "WHEN: run the flow\nTHEN: it works");
    assert_eq!(
        raw_2,
        "GIVEN: user exists\nWHEN: run the flow\nTHEN: it works"
    );
}

#[test]
fn test_sdd_authoring_helpers_produce_strict_valid_spec_and_change() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);
    git_commit_all(work_dir, "init");

    let spec_skel = run_llman(&["sdd", "spec", "skeleton", "sample"], work_dir, work_dir);
    assert_success(&spec_skel);

    let add_req = run_llman(
        &[
            "sdd",
            "spec",
            "add-requirement",
            "sample",
            "r1",
            "--title",
            "First requirement",
            "--statement",
            "System MUST do the first thing.",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&add_req);

    let add_scenario = run_llman(
        &[
            "sdd",
            "spec",
            "add-scenario",
            "sample",
            "r1",
            "s2",
            "--when",
            "running the flow",
            "--then",
            "the first thing happens",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&add_scenario);

    git_commit_all(work_dir, "add spec content");

    let validate_spec = run_llman(
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
    assert_success(&validate_spec);
    let validate_spec_json: Value =
        serde_json::from_slice(&validate_spec.stdout).expect("validate spec json");
    assert_eq!(validate_spec_json["items"][0]["valid"], true);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("add-sample");
    fs::create_dir_all(&change_dir).expect("create change dir");
    let proposal = "## Why\nNeed a sample change.\n\n## What Changes\n- Add requirement.\n";
    fs::write(change_dir.join("proposal.md"), proposal).expect("write proposal");
    fs::write(
        change_dir.join("design.md"),
        "# Design\n\nSimple change, no trade-offs.\n",
    )
    .expect("write design");
    fs::write(change_dir.join("tasks.md"), "- [x] Implement the change\n").expect("write tasks");

    let delta_skel = run_llman(
        &["sdd", "delta", "skeleton", "add-sample", "sample"],
        work_dir,
        work_dir,
    );
    assert_success(&delta_skel);

    let add_op = run_llman(
        &[
            "sdd",
            "delta",
            "add-req",
            "add-sample",
            "sample",
            "r2",
            "--title",
            "R2",
            "--statement",
            "System MUST support R2.",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&add_op);

    let add_delta_scenario = run_llman(
        &[
            "sdd",
            "delta",
            "add-scenario",
            "add-sample",
            "sample",
            "r2",
            "s1",
            "--when",
            "r2 is used",
            "--then",
            "it works",
        ],
        work_dir,
        work_dir,
    );
    assert_success(&add_delta_scenario);

    git_commit_all(work_dir, "add delta content");

    let validate_change = run_llman(
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
    assert_success(&validate_change);
    let validate_change_json: Value =
        serde_json::from_slice(&validate_change.stdout).expect("validate change json");
    assert_eq!(validate_change_json["items"][0]["valid"], true);
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
    let delta_spec = "kind: llman.sdd.delta\nops[1]{op,req_id,title,statement,from,to,name}:\n  add_requirement,added,Added behavior,System MUST support the added behavior.,null,null,null\nop_scenarios[1]{req_id,id,given,when,then}:\n  added,added,,a new action is taken,the new behavior happens\n";
    fs::write(change_specs_dir.join("spec.toon"), delta_spec).expect("write delta spec");

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

    // stage is inferred from artifacts (proposal + specs = specified, no
    // design/tasks yet → not ready to implement). See sdd-workflow r46.
    assert_eq!(show_json["stage"], "specified");
    assert_eq!(show_json["readyToImplement"], false);
    let artifacts = show_json["artifacts"]
        .as_array()
        .expect("artifacts is an array");
    assert!(
        artifacts.iter().any(|v| v == "proposal.md"),
        "artifacts should include proposal.md, got: {artifacts:?}"
    );
    assert!(
        artifacts.iter().any(|v| v == "specs"),
        "artifacts should include specs, got: {artifacts:?}"
    );
    assert!(
        !artifacts.iter().any(|v| v == "design.md"),
        "artifacts should not include design.md yet"
    );
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
    fs::write(
        change_dir.join("design.md"),
        "# Design\n\nSimple change, no trade-offs.\n",
    )
    .expect("write design");
    fs::write(change_dir.join("tasks.md"), "- [x] Implement the change\n").expect("write tasks");
    let delta_spec = "kind: llman.sdd.delta\nops[1]{op,req_id,title,statement,from,to,name}:\n  add_requirement,added,Added behavior,System MUST support the added behavior.,null,null,null\nop_scenarios[1]{req_id,id,given,when,then}:\n  added,added,,a new action is taken,the new behavior happens\n";
    fs::write(change_specs_dir.join("spec.toon"), delta_spec).expect("write delta spec");

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
fn test_sdd_update_recreates_root_agents_md() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let agents_path = work_dir.join("AGENTS.md");
    fs::remove_file(&agents_path).expect("remove root AGENTS.md");
    assert!(!agents_path.exists());

    let update_output = run_llman(&["sdd", "init", "--update"], work_dir, work_dir);
    assert_success(&update_output);

    assert!(agents_path.exists());
    let content = fs::read_to_string(&agents_path).expect("read root AGENTS.md");
    assert!(content.contains("<!-- LLMANSPEC:START -->"));
    assert!(content.contains("LLMAN Spec-Driven Development"));
    assert!(content.contains("/llman-sdd-explore"));
}

#[test]
fn test_sdd_update_skills_writes_agents_skills() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let output_dir = work_dir.join(".agents/skills");
    let update_output = run_llman(&["sdd", "init", "--update"], work_dir, work_dir);
    assert_success(&update_output);

    let skill_path = output_dir.join("llman-sdd-explore").join("SKILL.md");
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

    for entry in fs::read_dir(&output_dir).expect("read skills output dir") {
        let entry = entry.expect("skills entry");
        let file_type = entry.file_type().expect("skills entry file type");
        if !file_type.is_dir() {
            continue;
        }
        let skill_md = entry.path().join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let content = fs::read_to_string(&skill_md).expect("read generated SKILL.md");
        assert_no_disallowed_prompt_markers(&skill_md, &content);
    }
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

    let override_path = work_dir.join("templates/sdd/en/skills/llman-sdd-explore.md");
    fs::create_dir_all(override_path.parent().expect("parent")).expect("mkdir override");
    fs::write(
        &override_path,
        r#"---
name: "llman-sdd-explore"
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

    let output_dir = work_dir.join(".agents/skills");
    let update_output = run_llman(&["sdd", "init", "--update"], work_dir, work_dir);
    assert_success(&update_output);

    let skill = fs::read_to_string(output_dir.join("llman-sdd-explore").join("SKILL.md"))
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

    let override_path = work_dir.join("templates/sdd/en/skills/llman-sdd-explore.md");
    fs::create_dir_all(override_path.parent().expect("parent")).expect("mkdir override");
    fs::write(
        &override_path,
        r#"---
name: "llman-sdd-explore"
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

    let output = run_llman(&["sdd", "init", "--update"], work_dir, work_dir);
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
fn test_sdd_validate_tasks_without_design_is_error() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("tasks-no-design");
    let change_specs_dir = change_dir.join("specs").join("sample");
    fs::create_dir_all(&change_specs_dir).expect("create change spec dir");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nTest constraint.\n\n## What Changes\n- Test.\n",
    )
    .expect("write proposal");
    fs::write(change_dir.join("tasks.md"), "## Tasks\n- [x] Done\n").expect("write tasks");
    let delta_spec = "kind: llman.sdd.delta\nops[1]{op,req_id,title,statement,from,to,name}:\n  add_requirement,r1,Test,System MUST test.,null,null,null\nop_scenarios[1]{req_id,id,given,when,then}:\n  r1,happy,\"\",trigger,outcome\n";
    fs::write(change_specs_dir.join("spec.toon"), delta_spec).expect("write delta spec");

    let validate_output = run_llman(
        &[
            "sdd",
            "validate",
            "tasks-no-design",
            "--type",
            "change",
            "--no-interactive",
        ],
        work_dir,
        work_dir,
    );
    let stderr = String::from_utf8_lossy(&validate_output.stderr);
    assert!(
        !validate_output.status.success(),
        "Should fail when tasks.md exists without design.md"
    );
    assert!(
        stderr.contains("tasks.md") && stderr.contains("design.md"),
        "Error should mention both tasks.md and design.md, got: {stderr}"
    );
}

#[test]
fn test_sdd_validate_completeness_stage_in_output() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("draft-only");
    fs::create_dir_all(&change_dir).expect("create change dir");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nJust a draft.\n\n## What Changes\n- Nothing yet.\n",
    )
    .expect("write proposal");

    let validate_output = run_llman(
        &[
            "sdd",
            "validate",
            "draft-only",
            "--type",
            "change",
            "--no-interactive",
        ],
        work_dir,
        work_dir,
    );
    let stdout = String::from_utf8_lossy(&validate_output.stdout);
    assert_success(&validate_output);
    assert!(
        stdout.contains("draft"),
        "Output should contain stage 'draft', got: {stdout}"
    );
}

#[test]
fn test_sdd_list_shows_stage_column() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("stage-test");
    fs::create_dir_all(&change_dir).expect("create change dir");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nTest stage display.\n",
    )
    .expect("write proposal");

    let list_output = run_llman(&["sdd", "list"], work_dir, work_dir);
    assert_success(&list_output);
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        stdout.contains("draft"),
        "List output should show stage 'draft', got: {stdout}"
    );

    let json_output = run_llman(&["sdd", "list", "--json"], work_dir, work_dir);
    assert_success(&json_output);
    let json: Value = serde_json::from_slice(&json_output.stdout).expect("parse json");
    let stage = json["changes"][0]["stage"].as_str().unwrap_or("");
    assert_eq!(stage, "draft", "JSON output should contain stage field");
}

/// A full change (proposal + specs + design + tasks) MUST report
/// `stage: full` and `readyToImplement: true` (sdd-workflow r46).
#[test]
fn test_sdd_show_change_full_stage_ready_to_implement() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("full-change");
    let change_specs_dir = change_dir.join("specs").join("sample");
    fs::create_dir_all(&change_specs_dir).expect("create change spec dir");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nFull change.\n\n## What Changes\n- Add behavior.\n",
    )
    .expect("write proposal");
    let delta_spec = "kind: llman.sdd.delta\nops[1]{op,req_id,title,statement,from,to,name}:\n  add_requirement,added,Added behavior,System MUST support the added behavior.,null,null,null\nop_scenarios[1]{req_id,id,given,when,then}:\n  added,added,,a new action is taken,the new behavior happens\n";
    fs::write(change_specs_dir.join("spec.toon"), delta_spec).expect("write delta spec");
    fs::write(change_dir.join("design.md"), "# Design\nTrivial.\n").expect("write design");
    fs::write(change_dir.join("tasks.md"), "- [ ] implement\n").expect("write tasks");

    let show_output = run_llman(
        &["sdd", "show", "full-change", "--type", "change", "--json"],
        work_dir,
        work_dir,
    );
    assert_success(&show_output);
    let show_json: Value = serde_json::from_slice(&show_output.stdout).expect("show change json");
    assert_eq!(show_json["stage"], "full");
    assert_eq!(show_json["readyToImplement"], true);
}

/// A draft change (proposal-only) under non-strict validate MUST surface the
/// stage hint as INFO instead of swallowing it on the valid short-circuit
/// (sdd-workflow r45 drift fix).
#[test]
fn test_sdd_validate_draft_non_stract_shows_stage_info() {
    let env = TestEnvironment::new();
    let work_dir = env.path();

    let init_output = run_llman(
        &["sdd", "init", work_dir.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&init_output);

    let llmanspec_dir = work_dir.join("llmanspec");
    let change_dir = llmanspec_dir.join("changes").join("draft-proposal");
    fs::create_dir_all(&change_dir).expect("create change dir");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nDraft.\n\n## What Changes\n- Nothing yet.\n",
    )
    .expect("write proposal");

    let validate_output = run_llman(
        &[
            "sdd",
            "validate",
            "draft-proposal",
            "--type",
            "change",
            "--no-interactive",
        ],
        work_dir,
        work_dir,
    );
    // draft is valid under non-strict (no errors)
    assert_success(&validate_output);
    // the stage INFO hint is now surfaced (was previously swallowed)
    let stderr = String::from_utf8_lossy(&validate_output.stderr);
    assert!(
        stderr.contains("draft"),
        "non-strict validate should surface the draft stage INFO, got stderr: {stderr}"
    );
}
