#![cfg(unix)]

use expectrl::{Eof, Session, WaitStatus};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn llman_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_llman"))
}

fn run_llman(args: &[&str], work_dir: &Path, config_dir: &Path) -> Output {
    Command::new(llman_bin())
        .env("LLMAN_ARENA_TEST_FAKE_RUNNER", "1")
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

#[derive(Debug, Deserialize)]
struct ApplyRecord {
    match_id: String,
    ok: bool,
}

#[derive(Debug, Deserialize)]
struct VerificationRecord {
    match_id: String,
    command: String,
    status: String,
}

fn write_fixture_config(root: &Path, config_dir: &Path, seed: u64, rounds: u32) -> PathBuf {
    let repo_template = root.join("repo_template");
    fs::create_dir_all(&repo_template).expect("create repo_template");

    // Prompts (system) are loaded from <config>/prompt/codex/<name>.md
    let prompt_dir = config_dir.join("prompt").join("codex");
    fs::create_dir_all(&prompt_dir).expect("create prompt dir");
    fs::write(prompt_dir.join("ok.md"), "ARENA_FAKE_DIFF_OK\n").expect("write ok prompt");
    fs::write(prompt_dir.join("fail.md"), "ARENA_FAKE_DIFF_FAIL\n").expect("write fail prompt");

    let contest_dir = config_dir.join("arena").join("contests");
    fs::create_dir_all(&contest_dir).expect("create contest dir");
    fs::write(
        contest_dir.join("c1.toml"),
        format!(
            r#"version = 1
name = "c1"
app = "codex"
models = ["m1"]
temperature = 0.0
max_output_tokens = 200
timeout_secs = 5
retries = 0
verify = ["bash -lc \"test -f arena_fake.txt\""]

[[prompts]]
id = "p_ok"
prompt_name = "ok"

[[prompts]]
id = "p_fail"
prompt_name = "fail"
"#
        ),
    )
    .expect("write contest");

    let dataset_dir = config_dir.join("arena").join("datasets");
    fs::create_dir_all(&dataset_dir).expect("create dataset dir");
    fs::write(
        dataset_dir.join("d1.yaml"),
        format!(
            r#"version: 1
name: d1
repo_template_path: {}

tasks:
  - id: t_repo_1
    type: repo
    prompt: |
      Produce a unified diff that creates `arena_fake.txt` in the repo root.
      Output ONLY the diff (no commentary).
"#,
            repo_template.display()
        ),
    )
    .expect("write dataset");

    let rounds_s = rounds.to_string();
    let seed_s = seed.to_string();
    let output = run_llman(
        &[
            "x",
            "arena",
            "gen",
            "--contest",
            "c1",
            "--dataset",
            "d1",
            "--rounds",
            rounds_s.as_str(),
            "--seed",
            seed_s.as_str(),
        ],
        root,
        config_dir,
    );
    assert_success(&output);

    config_dir
        .join("arena")
        .join("runs")
        .join(format!("run_{seed}"))
}

#[test]
fn arena_gen_creates_run_artifacts_and_repo_eval_records() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");

    let run_dir = write_fixture_config(root, &config_dir, 123, 1);

    assert!(run_dir.exists(), "expected {}", run_dir.display());
    assert!(
        run_dir.join("matches.jsonl").exists(),
        "expected matches.jsonl"
    );
    assert!(
        run_dir.join("generations.jsonl").exists(),
        "expected generations.jsonl"
    );
    assert!(
        run_dir.join("applies.jsonl").exists(),
        "expected applies.jsonl"
    );
    assert!(
        run_dir.join("verifications.jsonl").exists(),
        "expected verifications.jsonl"
    );

    // Ensure repo template is never modified in place.
    assert!(
        !root.join("repo_template/arena_fake.txt").exists(),
        "repo template was modified in place"
    );

    let applies = fs::read_to_string(run_dir.join("applies.jsonl")).expect("read applies");
    let apply_records = applies
        .lines()
        .map(|l| serde_json::from_str::<ApplyRecord>(l).expect("parse apply"))
        .collect::<Vec<_>>();
    assert_eq!(apply_records.len(), 2);
    assert!(apply_records.iter().all(|r| r.match_id == "000001"));
    let oks = apply_records.iter().map(|r| r.ok).collect::<Vec<_>>();
    assert!(oks.contains(&true), "expected at least one ok apply");
    assert!(oks.contains(&false), "expected at least one failed apply");

    let verifs = fs::read_to_string(run_dir.join("verifications.jsonl")).expect("read verifs");
    let verif_records = verifs
        .lines()
        .map(|l| serde_json::from_str::<VerificationRecord>(l).expect("parse verif"))
        .collect::<Vec<_>>();
    assert_eq!(verif_records.len(), 2);
    assert!(verif_records.iter().all(|r| r.match_id == "000001"));
    let statuses = verif_records
        .iter()
        .map(|r| r.status.as_str())
        .collect::<Vec<_>>();
    assert!(statuses.contains(&"ok"), "expected ok verification");
    assert!(
        statuses.contains(&"skipped"),
        "expected skipped verification"
    );
    assert!(verif_records.iter().any(|r| r.command == "<skipped>"));
    assert!(
        verif_records
            .iter()
            .any(|r| r.command.contains("test -f arena_fake.txt"))
    );
}

#[test]
fn arena_report_errors_when_no_votes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");

    let run_dir = write_fixture_config(root, &config_dir, 456, 1);
    assert!(run_dir.exists());

    let output = run_llman(
        &["x", "arena", "report", "--run", "run_456"],
        root,
        &config_dir,
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("At least one vote is required"),
        "unexpected stderr:\n{stderr}"
    );
}

#[test]
fn arena_vote_resumes_after_preexisting_vote() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");

    let run_dir = write_fixture_config(root, &config_dir, 789, 2);
    let votes_path = run_dir.join("votes.jsonl");

    // Pre-seed a vote so `vote` should skip match 000001 and continue with 000002.
    fs::write(
        &votes_path,
        "{\"match_id\":\"000001\",\"winner\":\"a\",\"ts_ms\":0}\n",
    )
    .expect("write votes.jsonl");

    let mut cmd = Command::new(llman_bin());
    cmd.env("LLMAN_ARENA_TEST_FAKE_RUNNER", "1")
        .args([
            "--config-dir",
            config_dir.to_str().expect("config dir"),
            "x",
            "arena",
            "vote",
            "--run",
            "run_789",
        ])
        .current_dir(root);

    let mut session = Session::spawn(cmd).expect("spawn llman in pty");
    session
        .expect("=== match 000002 ===")
        .expect("see match 000002");

    // Let inquire render its prompt before sending keys.
    thread::sleep(Duration::from_millis(200));
    session.expect("Pick winner").expect("winner prompt");

    // Select "Quit" (4x Down, then Enter).
    session
        .send("\u{1b}[B\u{1b}[B\u{1b}[B\u{1b}[B\r")
        .expect("send quit");

    session.expect(Eof).expect("eof");
    assert_eq!(
        session.wait().expect("wait"),
        WaitStatus::Exited(session.pid(), 0)
    );

    let votes_after = fs::read_to_string(&votes_path).expect("read votes.jsonl");
    assert_eq!(votes_after.lines().count(), 1);
}
