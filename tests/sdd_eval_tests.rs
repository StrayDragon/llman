#![cfg(unix)]

use ignore::WalkBuilder;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn llman_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_llman"))
}

fn fake_agent_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_llman-fake-acp-agent"))
}

fn run_llman(args: &[&str], work_dir: &Path, config_dir: &Path) -> Output {
    Command::new(llman_bin())
        .args(["--config-dir", config_dir.to_str().expect("config dir")])
        .args(args)
        .current_dir(work_dir)
        .output()
        .expect("run llman")
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

fn write_claude_code_group_config(config_dir: &Path, group: &str, secret: &str) {
    fs::create_dir_all(config_dir).expect("create config dir");
    fs::write(
        config_dir.join("claude-code.toml"),
        format!(
            r#"
[groups.{group}]
ANTHROPIC_AUTH_TOKEN = "{secret}"
"#
        ),
    )
    .expect("write claude-code.toml");
}

fn write_playbook(project_root: &Path, agent_command: &Path, group: &str) -> PathBuf {
    let path = project_root.join("playbook.yaml");
    fs::write(
        &path,
        format!(
            r#"version: 1
task:
  title: "demo"
  prompt: |
    Create a file in the repo root called demo.txt.

sdd_loop:
  max_iterations: 1

variants:
  - name: v1
    style: sdd
    agent:
      kind: claude-code-acp
      preset: {group}
      command: "{cmd}"

report:
  ai_judge:
    enabled: false
    model: gpt-4.1
  human:
    enabled: true
"#,
            group = group,
            cmd = agent_command.display()
        ),
    )
    .expect("write playbook");
    path
}

fn find_secret_in_tree(root: &Path, secret: &str) -> Vec<PathBuf> {
    let mut hits = Vec::new();
    for entry in WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .build()
    {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let Some(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(path) else { continue };
        // Skip large files.
        if bytes.len() > 2_000_000 {
            continue;
        }
        let hay = String::from_utf8_lossy(&bytes);
        if hay.contains(secret) {
            hits.push(path.to_path_buf());
        }
    }
    hits
}

#[derive(Debug, Deserialize)]
struct ManifestVariant {
    name: String,
    injected_env_keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    run_id: String,
    variants: Vec<ManifestVariant>,
}

#[derive(Debug, Deserialize)]
struct Metrics {
    denied_operations: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ReportVariantHumanScore {
    score: f64,
}

#[derive(Debug, Deserialize)]
struct ReportVariant {
    name: String,
    human_score: Option<ReportVariantHumanScore>,
}

#[derive(Debug, Deserialize)]
struct Report {
    variants: Vec<ReportVariant>,
}

#[test]
fn sdd_eval_run_is_sandboxed_and_redacts_secrets() {
    let temp = TempDir::new().expect("temp dir");
    let project_root = temp.path();
    fs::write(project_root.join("README.md"), "demo\n").expect("write readme");

    // Initialize git repo so project root resolution is deterministic.
    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(project_root)
        .output()
        .expect("git init");

    let config_temp = TempDir::new().expect("config temp dir");
    let config_dir = config_temp.path();
    let secret = "super-secret-token-123";
    write_claude_code_group_config(config_dir, "test", secret);

    let playbook = write_playbook(project_root, &fake_agent_bin(), "test");

    let out = run_llman(
        &[
            "x",
            "sdd-eval",
            "run",
            "--playbook",
            playbook.to_str().unwrap(),
        ],
        project_root,
        config_dir,
    );
    assert_success(&out);

    let run_dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(
        !run_dir.is_empty(),
        "expected run dir path in stdout, got empty"
    );
    let run_dir = PathBuf::from(run_dir);
    assert!(run_dir.exists(), "expected {}", run_dir.display());

    // Manifest contains env keys but never values.
    let manifest: Manifest = serde_json::from_str(
        &fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest"),
    )
    .expect("parse manifest");
    assert_eq!(manifest.variants.len(), 1);
    assert_eq!(manifest.variants[0].name, "v1");
    assert!(
        manifest.variants[0]
            .injected_env_keys
            .contains(&"ANTHROPIC_AUTH_TOKEN".to_string()),
        "expected injected env key recorded"
    );

    // Secret MUST NOT appear anywhere under the run dir.
    let hits = find_secret_in_tree(&run_dir, secret);
    assert!(hits.is_empty(), "secret leaked into run dir: {hits:?}");

    // Sandbox should deny at least one operation (/etc/passwd probe).
    let metrics: Metrics = serde_json::from_str(
        &fs::read_to_string(
            run_dir
                .join("variants")
                .join("v1")
                .join("artifacts")
                .join("acp-metrics.json"),
        )
        .expect("read metrics"),
    )
    .expect("parse metrics");
    assert!(
        !metrics.denied_operations.is_empty(),
        "expected at least one denied operation"
    );

    // Report generation should succeed and produce artifacts.
    let report_out = run_llman(
        &["x", "sdd-eval", "report", "--run", &manifest.run_id],
        project_root,
        config_dir,
    );
    assert_success(&report_out);
    assert!(run_dir.join("report.json").exists(), "expected report.json");
    assert!(run_dir.join("report.md").exists(), "expected report.md");
    assert!(
        run_dir.join("human-pack.json").exists(),
        "expected human-pack.json"
    );
    assert!(
        run_dir.join("human-scores.template.json").exists(),
        "expected human-scores.template.json"
    );

    // Import human scores and ensure report includes them.
    let scores_path = project_root.join("scores.json");
    fs::write(
        &scores_path,
        r#"{ "file": "scores.json", "variants": { "v1": { "score": 7.5, "notes": "ok" } } }"#,
    )
    .expect("write scores");
    let import_out = run_llman(
        &[
            "x",
            "sdd-eval",
            "import-human",
            "--run",
            &manifest.run_id,
            "--file",
            scores_path.to_str().unwrap(),
        ],
        project_root,
        config_dir,
    );
    assert_success(&import_out);

    let report: Report =
        serde_json::from_str(&fs::read_to_string(run_dir.join("report.json")).unwrap())
            .expect("parse report.json");
    let v1 = report
        .variants
        .into_iter()
        .find(|v| v.name == "v1")
        .expect("variant v1");
    assert_eq!(v1.human_score.unwrap().score, 7.5);
}
