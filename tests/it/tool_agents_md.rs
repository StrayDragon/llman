//! Integration tests for `llman tool agents-md` (scan / clean / revert).

use llman::tool::agents_md;
use llman::tool::command::{
    AgentsMdCleanArgs, AgentsMdCommands, AgentsMdRevertArgs, AgentsMdScanArgs,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Initialize a fresh git repo on `main` with an initial commit.
fn git_repo() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path();
    git(root, &["init", "-b", "main"]);
    git(root, &["config", "user.name", "t"]);
    git(root, &["config", "user.email", "t@x"]);
    fs::write(root.join("README"), "hi").expect("write README");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "init"]);
    dir
}

fn git(root: &std::path::Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write_manifest(root: &std::path::Path, files: &[&str]) -> PathBuf {
    let dir = root.join(".llman");
    fs::create_dir_all(&dir).expect("mkdir .llman");
    let list: Vec<String> = files.iter().map(|f| format!("      - {f}")).collect();
    let yaml = format!(
        "version: \"0.1\"\ntools:\n  agents-md:\n    files:\n{}\n",
        list.join("\n")
    );
    let path = dir.join("config.yaml");
    fs::write(&path, yaml).expect("write manifest");
    path
}

// ---------------------------------------------------------------------------
// scan (r121)
// ---------------------------------------------------------------------------

#[test]
fn scan_lists_discovered_agent_init_files() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "x").expect("write");
    fs::write(root.join("CLAUDE.md"), "x").expect("write");

    // capture stdout by running in subprocess is overkill; scan returns Ok and
    // writes paths. We assert success + that files still exist (scan is read-only).
    let args = AgentsMdScanArgs {
        upsert_project_configs: false,
        config: None,
        verbose: false,
    };
    // scan relies on cwd git root; run from root.
    let _guard = cwd_guard(root);
    let result = agents_md::run_scan(&args);
    assert!(result.is_ok(), "scan failed: {:?}", result.err());
    // read-only: files untouched
    assert!(root.join("AGENTS.md").exists());
    assert!(root.join("CLAUDE.md").exists());
}

#[test]
fn scan_upsert_writes_project_manifest() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "x").expect("write");
    assert!(!root.join(".llman/config.yaml").exists());

    let args = AgentsMdScanArgs {
        upsert_project_configs: true,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_scan(&args).expect("scan --upsert");

    let config = fs::read_to_string(root.join(".llman/config.yaml")).expect("read config");
    assert!(config.contains("agents-md"), "config: {config}");
    assert!(config.contains("AGENTS.md"), "config: {config}");
}

#[test]
fn scan_upsert_creates_config_when_absent() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "x").expect("write");

    let args = AgentsMdScanArgs {
        upsert_project_configs: true,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_scan(&args).expect("scan --upsert creates config");

    assert!(root.join(".llman/config.yaml").exists(), "config created");
}

// ---------------------------------------------------------------------------
// clean (r122)
// ---------------------------------------------------------------------------

#[test]
fn clean_dry_run_does_not_delete() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "x").expect("write");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "add agents"]);
    write_manifest(root, &["AGENTS.md"]);

    let args = AgentsMdCleanArgs {
        yes: false,
        commit: false,
        force: false,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_clean(&args).expect("clean dry-run");
    assert!(root.join("AGENTS.md").exists(), "dry-run must not delete");
}

#[test]
fn clean_yes_deletes_tracked_files() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "x").expect("write");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "add agents"]);
    write_manifest(root, &["AGENTS.md"]);

    let args = AgentsMdCleanArgs {
        yes: true,
        commit: false,
        force: false,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_clean(&args).expect("clean --yes");
    assert!(!root.join("AGENTS.md").exists(), "AGENTS.md deleted");
}

#[test]
fn clean_expands_directory_manifest_to_tracked_files() {
    let dir = git_repo();
    let root = dir.path();
    fs::create_dir_all(root.join(".cursor/rules")).expect("mkdir");
    fs::write(root.join(".cursor/rules/a.mdc"), "rule").expect("write");
    fs::write(root.join(".cursor/rules/b.mdc"), "rule").expect("write");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "add cursor rules"]);
    write_manifest(root, &[".cursor/"]);

    let args = AgentsMdCleanArgs {
        yes: true,
        commit: false,
        force: false,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_clean(&args).expect("clean dir");
    assert!(!root.join(".cursor/rules/a.mdc").exists(), "a.mdc deleted");
    assert!(!root.join(".cursor/rules/b.mdc").exists(), "b.mdc deleted");
}

#[test]
fn clean_commit_on_default_branch_is_rejected() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "x").expect("write");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "add agents"]);
    write_manifest(root, &["AGENTS.md"]);

    let args = AgentsMdCleanArgs {
        yes: false,
        commit: true,
        force: false,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    let err = agents_md::run_clean(&args).unwrap_err().to_string();
    assert!(err.contains("default branch"), "got: {err}");
}

#[test]
fn clean_commit_with_force_executes_on_default_branch() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "x").expect("write");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "add agents"]);
    write_manifest(root, &["AGENTS.md"]);

    let args = AgentsMdCleanArgs {
        yes: false,
        commit: true,
        force: true,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_clean(&args).expect("clean --commit --force");
    // committed: the file removal lands in HEAD
    let log = String::from_utf8_lossy(
        &Command::new("git")
            .args(["log", "--oneline", "-1"])
            .current_dir(root)
            .output()
            .expect("git log")
            .stdout,
    )
    .to_string();
    assert!(log.contains("chore(agents-md)"), "log: {log}");
}

#[test]
fn clean_commit_on_feature_branch_executes() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "x").expect("write");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "add agents"]);
    write_manifest(root, &["AGENTS.md"]);
    git(root, &["checkout", "-b", "feature/dev"]);

    let args = AgentsMdCleanArgs {
        yes: false,
        commit: true,
        force: false,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_clean(&args).expect("clean --commit on feature branch");
    let log = String::from_utf8_lossy(
        &Command::new("git")
            .args(["log", "--oneline", "-1"])
            .current_dir(root)
            .output()
            .expect("git log")
            .stdout,
    )
    .to_string();
    assert!(log.contains("chore(agents-md)"), "log: {log}");
}

// ---------------------------------------------------------------------------
// revert (r123)
// ---------------------------------------------------------------------------

#[test]
fn revert_restores_file_from_default_branch() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "main content").expect("write");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "add agents"]);
    write_manifest(root, &["AGENTS.md"]);

    // feature branch: clean via commit, then revert.
    git(root, &["checkout", "-b", "feature/dev"]);
    let clean_args = AgentsMdCleanArgs {
        yes: false,
        commit: true,
        force: false,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_clean(&clean_args).expect("clean");
    assert!(!root.join("AGENTS.md").exists());

    let revert_args = AgentsMdRevertArgs {
        yes: true,
        commit: false,
        config: None,
        verbose: false,
    };
    agents_md::run_revert(&revert_args).expect("revert");
    let content = fs::read_to_string(root.join("AGENTS.md")).expect("restored");
    assert_eq!(content, "main content");
}

#[test]
fn revert_commit_on_default_branch_creates_recovery_branch() {
    let dir = git_repo();
    let root = dir.path();
    fs::write(root.join("AGENTS.md"), "main content").expect("write");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "add agents"]);
    write_manifest(root, &["AGENTS.md"]);

    // Corrupt the file on the default branch (simulating a bad merge/edit) so
    // revert has a real change to restore.
    fs::write(root.join("AGENTS.md"), "corrupted").expect("write corrupt");

    let revert_args = AgentsMdRevertArgs {
        yes: false,
        commit: true,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_revert(&revert_args).expect("revert --commit on main");

    let branch = String::from_utf8_lossy(
        &Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(root)
            .output()
            .expect("git branch")
            .stdout,
    )
    .trim()
    .to_string();
    assert!(branch.starts_with("agents-md/revert-"), "branch: {branch}");
    let content = fs::read_to_string(root.join("AGENTS.md")).expect("restored");
    assert_eq!(content, "main content");
}

#[test]
fn revert_expands_directory_manifest() {
    let dir = git_repo();
    let root = dir.path();
    fs::create_dir_all(root.join(".cursor/rules")).expect("mkdir");
    fs::write(root.join(".cursor/rules/a.mdc"), "rule-main").expect("write");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "add cursor"]);
    write_manifest(root, &[".cursor/"]);

    git(root, &["checkout", "-b", "feature/dev"]);
    let clean_args = AgentsMdCleanArgs {
        yes: false,
        commit: true,
        force: false,
        config: None,
        verbose: false,
    };
    let _guard = cwd_guard(root);
    agents_md::run_clean(&clean_args).expect("clean");
    assert!(!root.join(".cursor/rules/a.mdc").exists());

    let revert_args = AgentsMdRevertArgs {
        yes: true,
        commit: false,
        config: None,
        verbose: false,
    };
    agents_md::run_revert(&revert_args).expect("revert");
    let content = fs::read_to_string(root.join(".cursor/rules/a.mdc")).expect("restored");
    assert_eq!(content, "rule-main");
}

// ---------------------------------------------------------------------------
// dispatch wiring
// ---------------------------------------------------------------------------

#[test]
fn agents_md_subcommand_dispatches() {
    // Ensure the enum variants exist and dispatch compiles.
    let _ = AgentsMdCommands::Scan(AgentsMdScanArgs {
        upsert_project_configs: false,
        config: None,
        verbose: false,
    });
}

/// Serialize tests that mutate the process-wide cwd.
///
/// `std::env::set_current_dir` is process-global, not thread-local. When
/// nextest runs multiple tests from this binary in one process, concurrent
/// chdir calls race: one test finishes and its `TempDir` is dropped (deleted)
/// while another test's `current_dir()` still points at it, producing
/// `cwd: Os { code: 2, NotFound }` (CI failure, run 30551746073). Holding this
/// mutex for the whole test forces cwd-touching tests to run one at a time.
/// See AGENTS.md "Avoid parallel test collisions".
static CWD_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Helper to temporarily change cwd into `root` and restore on drop, while
/// holding `CWD_MUTEX` so parallel tests can't race on the process cwd.
struct CwdGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prev: PathBuf,
}
impl Drop for CwdGuard {
    fn drop(&mut self) {
        // Restore cwd first, then release the mutex (field order: prev, then _lock).
        let _ = std::env::set_current_dir(&self.prev);
    }
}
fn cwd_guard(root: &std::path::Path) -> CwdGuard {
    let lock = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(root).expect("chdir root");
    CwdGuard { _lock: lock, prev }
}
