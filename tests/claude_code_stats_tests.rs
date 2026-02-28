use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn llman_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_llman"))
}

fn run_llman(args: &[&str], work_dir: &Path, config_dir: &Path) -> Output {
    Command::new(llman_bin())
        .args(["--config-dir", config_dir.to_str().expect("config dir")])
        .args(args)
        .current_dir(work_dir)
        .output()
        .expect("run llman")
}

fn write_sessions_index(project_dir: &Path, entries_json: &str) {
    let index = format!(
        r#"{{
  "version": 1,
  "entries": {entries_json},
  "originalPath": "/ignored"
}}"#
    );
    fs::write(project_dir.join("sessions-index.json"), index).expect("write sessions-index");
}

#[test]
fn claude_code_stats_summary_json_includes_sidechain_by_default() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir).expect("mkdir work");
    let work_dir = fs::canonicalize(&work_dir).expect("canonicalize work");
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).expect("mkdir config");

    let projects_dir = temp.path().join("projects");
    let project = projects_dir.join("p1");
    fs::create_dir_all(&project).expect("mkdir project");

    let s1 = project.join("s1.jsonl");
    let s2 = project.join("s2.jsonl");
    let s3 = project.join("s3.jsonl");

    fs::write(
        &s1,
        r#"{"timestamp":"2026-02-01T00:00:00Z","message":{"usage":{"input_tokens":10,"output_tokens":5}}}"#,
    )
    .expect("write s1");
    fs::write(
        &s2,
        r#"{"timestamp":"2026-02-03T00:00:00Z","message":{"usage":{"input_tokens":1,"output_tokens":1,"cache_read_input_tokens":8}}}"#,
    )
    .expect("write s2");
    fs::write(&s3, r#"{"timestamp":"2026-02-04T00:00:00Z","message":{}}"#).expect("write s3");

    write_sessions_index(
        &project,
        &format!(
            r#"
[
  {{
    "sessionId": "s1",
    "fullPath": "{}",
    "projectPath": "{}",
    "created": "2026-02-01T00:00:00Z",
    "modified": "2026-02-01T00:00:00Z",
    "isSidechain": false
  }},
  {{
    "sessionId": "s2",
    "fullPath": "{}",
    "projectPath": "{}",
    "created": "2026-02-03T00:00:00Z",
    "modified": "2026-02-03T00:00:00Z",
    "isSidechain": true
  }},
  {{
    "sessionId": "s3",
    "fullPath": "{}",
    "projectPath": "{}",
    "created": "2026-02-04T00:00:00Z",
    "modified": "2026-02-04T00:00:00Z",
    "isSidechain": false
  }}
]
"#,
            s1.to_string_lossy(),
            work_dir.to_string_lossy(),
            s2.to_string_lossy(),
            work_dir.to_string_lossy(),
            s3.to_string_lossy(),
            work_dir.to_string_lossy(),
        ),
    );

    let output = run_llman(
        &[
            "x",
            "claude-code",
            "stats",
            "--projects-dir",
            projects_dir.to_str().unwrap(),
            "--format",
            "json",
        ],
        &work_dir,
        &config_dir,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let v: Value = serde_json::from_slice(&output.stdout).expect("parse json");
    assert_eq!(v["tool"], "claude-code");
    assert_eq!(v["view"], "summary");
    assert_eq!(v["result"]["coverage"]["total_sessions"], 3);
    assert_eq!(v["result"]["coverage"]["known_token_sessions"], 2);
    assert_eq!(v["result"]["totals"]["tokens_total_known"], 25);
    assert_eq!(
        v["result"]["sidechain_totals"]["primary"]["tokens_total_known"],
        15
    );
    assert_eq!(
        v["result"]["sidechain_totals"]["sidechain"]["tokens_total_known"],
        10
    );
}

#[test]
fn claude_code_stats_no_sidechain_excludes_sidechain_sessions() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir).expect("mkdir work");
    let work_dir = fs::canonicalize(&work_dir).expect("canonicalize work");
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).expect("mkdir config");

    let projects_dir = temp.path().join("projects");
    let project = projects_dir.join("p1");
    fs::create_dir_all(&project).expect("mkdir project");

    let s1 = project.join("s1.jsonl");
    let s2 = project.join("s2.jsonl");
    fs::write(
        &s1,
        r#"{"timestamp":"2026-02-01T00:00:00Z","message":{"usage":{"input_tokens":10,"output_tokens":5}}}"#,
    )
    .expect("write s1");
    fs::write(
        &s2,
        r#"{"timestamp":"2026-02-03T00:00:00Z","message":{"usage":{"input_tokens":1,"output_tokens":1}}}"#,
    )
    .expect("write s2");

    write_sessions_index(
        &project,
        &format!(
            r#"
[
  {{
    "sessionId": "s1",
    "fullPath": "{}",
    "projectPath": "{}",
    "created": "2026-02-01T00:00:00Z",
    "modified": "2026-02-01T00:00:00Z",
    "isSidechain": false
  }},
  {{
    "sessionId": "s2",
    "fullPath": "{}",
    "projectPath": "{}",
    "created": "2026-02-03T00:00:00Z",
    "modified": "2026-02-03T00:00:00Z",
    "isSidechain": true
  }}
]
"#,
            s1.to_string_lossy(),
            work_dir.to_string_lossy(),
            s2.to_string_lossy(),
            work_dir.to_string_lossy(),
        ),
    );

    let output = run_llman(
        &[
            "x",
            "cc",
            "stats",
            "--projects-dir",
            projects_dir.to_str().unwrap(),
            "--no-sidechain",
            "--format",
            "json",
        ],
        &work_dir,
        &config_dir,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let v: Value = serde_json::from_slice(&output.stdout).expect("parse json");
    assert_eq!(v["result"]["coverage"]["total_sessions"], 1);
    assert_eq!(v["result"]["totals"]["tokens_total_known"], 15);
}

#[test]
fn claude_code_stats_since_filters_by_end_ts() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir).expect("mkdir work");
    let work_dir = fs::canonicalize(&work_dir).expect("canonicalize work");
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).expect("mkdir config");

    let projects_dir = temp.path().join("projects");
    let project = projects_dir.join("p1");
    fs::create_dir_all(&project).expect("mkdir project");

    let old = project.join("old.jsonl");
    let new = project.join("new.jsonl");
    fs::write(
        &old,
        r#"{"timestamp":"2026-02-01T00:00:00Z","message":{"usage":{"input_tokens":1,"output_tokens":1}}}"#,
    )
    .expect("write old");
    fs::write(
        &new,
        r#"{"timestamp":"2026-02-10T00:00:00Z","message":{"usage":{"input_tokens":2,"output_tokens":2}}}"#,
    )
    .expect("write new");

    write_sessions_index(
        &project,
        &format!(
            r#"
[
  {{
    "sessionId": "old",
    "fullPath": "{}",
    "projectPath": "{}",
    "created": "2026-02-01T00:00:00Z",
    "modified": "2026-02-01T00:00:00Z",
    "isSidechain": false
  }},
  {{
    "sessionId": "new",
    "fullPath": "{}",
    "projectPath": "{}",
    "created": "2026-02-10T00:00:00Z",
    "modified": "2026-02-10T00:00:00Z",
    "isSidechain": false
  }}
]
"#,
            old.to_string_lossy(),
            work_dir.to_string_lossy(),
            new.to_string_lossy(),
            work_dir.to_string_lossy(),
        ),
    );

    let output = run_llman(
        &[
            "x",
            "claude-code",
            "stats",
            "--projects-dir",
            projects_dir.to_str().unwrap(),
            "--since",
            "2026-02-05T00:00:00Z",
            "--format",
            "json",
        ],
        &work_dir,
        &config_dir,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let v: Value = serde_json::from_slice(&output.stdout).expect("parse json");
    assert_eq!(v["result"]["coverage"]["total_sessions"], 1);
    assert_eq!(v["result"]["totals"]["tokens_total_known"], 4);
}

