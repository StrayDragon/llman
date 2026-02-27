#![cfg(unix)]

use expectrl::{ControlCode, Eof, Session, WaitStatus};
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

#[test]
fn agents_new_creates_skill_and_manifest() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let skills_root = root.join("skills");

    let output = run_llman(
        &[
            "agents",
            "new",
            "foo-agent",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert_success(&output);

    let skill_file = skills_root.join("foo-agent").join("SKILL.md");
    let manifest_file = config_dir
        .join("agents")
        .join("foo-agent")
        .join("agent.toml");

    assert!(skill_file.exists(), "expected {}", skill_file.display());
    assert!(
        manifest_file.exists(),
        "expected {}",
        manifest_file.display()
    );

    let skill_md = fs::read_to_string(&skill_file).expect("read SKILL.md");
    assert!(skill_md.contains("name: foo-agent"));
    assert!(skill_md.contains("## Requirements"));

    let manifest = fs::read_to_string(&manifest_file).expect("read agent.toml");
    assert!(manifest.contains("version = 1"));
    assert!(manifest.contains("id = \"foo-agent\""));
    assert!(manifest.contains("includes = []"));
}

#[test]
fn agents_new_rejects_path_traversal_id() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let skills_root = root.join("skills");

    let output = run_llman(
        &[
            "agents",
            "new",
            "../evil",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert!(
        !output.status.success(),
        "expected failure, got success:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid agent id"),
        "expected error message to mention invalid agent id:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        !root.join("evil").exists(),
        "unexpected write outside skills root"
    );
    assert!(
        !config_dir.join("agents").join("evil").exists(),
        "unexpected write outside config dir"
    );
}

#[test]
fn agents_new_fails_when_already_exists_without_force() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let skills_root = root.join("skills");

    let output = run_llman(
        &[
            "agents",
            "new",
            "foo-agent",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert_success(&output);

    let original_skill_md =
        fs::read_to_string(skills_root.join("foo-agent/SKILL.md")).expect("read original SKILL.md");
    let original_manifest = fs::read_to_string(config_dir.join("agents/foo-agent/agent.toml"))
        .expect("read original manifest");

    let output = run_llman(
        &[
            "agents",
            "new",
            "foo-agent",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert!(
        !output.status.success(),
        "expected failure, got success:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let after_skill_md =
        fs::read_to_string(skills_root.join("foo-agent/SKILL.md")).expect("read SKILL.md");
    let after_manifest =
        fs::read_to_string(config_dir.join("agents/foo-agent/agent.toml")).expect("read manifest");
    assert_eq!(after_skill_md, original_skill_md);
    assert_eq!(after_manifest, original_manifest);
}

#[test]
fn agents_new_force_overwrites_existing_outputs() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let skills_root = root.join("skills");

    let output = run_llman(
        &[
            "agents",
            "new",
            "foo-agent",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert_success(&output);

    fs::write(skills_root.join("foo-agent/SKILL.md"), "changed").expect("mutate SKILL.md");
    fs::write(
        config_dir.join("agents/foo-agent/agent.toml"),
        "version = 1\nid = \"foo-agent\"\nincludes = [\"x\"]\n",
    )
    .expect("mutate manifest");

    let output = run_llman(
        &[
            "agents",
            "new",
            "foo-agent",
            "--force",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert_success(&output);

    let skill_md = fs::read_to_string(skills_root.join("foo-agent/SKILL.md")).expect("read");
    assert!(skill_md.contains("name: foo-agent"));
    assert!(!skill_md.contains("changed"));

    let manifest =
        fs::read_to_string(config_dir.join("agents/foo-agent/agent.toml")).expect("read");
    assert!(manifest.contains("includes = []"));
}

#[test]
fn agents_new_interactive_cancel_writes_nothing() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let skills_root = root.join("skills");

    // Ensure at least one selectable skill exists so the TUI runs.
    fs::create_dir_all(skills_root.join("dummy")).expect("create dummy skill dir");
    fs::write(
        skills_root.join("dummy/SKILL.md"),
        "---\nname: dummy\n---\n",
    )
    .expect("write SKILL");

    let mut cmd = Command::new(llman_bin());
    cmd.args([
        "--config-dir",
        config_dir.to_str().unwrap(),
        "agents",
        "new",
        "foo-agent",
        "--skills-dir",
        skills_root.to_str().unwrap(),
    ])
    .current_dir(root);

    let mut session = Session::spawn(cmd).expect("spawn llman in pty");
    thread::sleep(Duration::from_millis(300));
    session
        .send_control(ControlCode::Escape)
        .expect("send escape");
    session.expect(Eof).expect("eof");
    assert_eq!(
        session.wait().expect("wait"),
        WaitStatus::Exited(session.pid(), 0)
    );

    assert!(
        !skills_root.join("foo-agent").exists(),
        "unexpected agent-skill dir created"
    );
    assert!(
        !config_dir.join("agents/foo-agent").exists(),
        "unexpected agent manifest dir created"
    );
}

#[test]
fn agents_new_ai_requires_feature_gate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let skills_root = root.join("skills");

    let output = run_llman(
        &[
            "agents",
            "new",
            "foo-agent",
            "--ai",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("agents-ai"));
}
