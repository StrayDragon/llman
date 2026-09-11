//! Pure git plumbing shared across feature modules (sdd change binding,
//! tool agents-md, skills config discovery, prompts paths).
//!
//! Function bodies are moved verbatim from their original homes
//! (`sdd::change::git_native`, `skills::shared::git`) so error messages stay
//! byte-identical. Sdd-specific binding semantics (ChangeGitBinding,
//! read/write_binding, start/attach/checkpoint flows) remain in
//! `sdd::change::git_native`.
//!
//! This module is a member of the top-level utility layer (future
//! `llman-core`); it MUST NOT import feature modules (sdd/skills/tool/x).

use anyhow::{Result, anyhow, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|err| anyhow!("git {:?} failed to spawn: {err}", args))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            bail!("git {:?} failed", args);
        }
        bail!("{stderr}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn current_branch(root: &Path) -> Result<Option<String>> {
    let branch = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch.is_empty() || branch == "HEAD" {
        // Detached HEAD: `--abbrev-ref HEAD` prints `HEAD`. Callers decide
        // whether that is acceptable and own the user-facing message.
        return Ok(None);
    }
    Ok(Some(branch))
}

pub fn current_head_sha(root: &Path) -> Result<String> {
    run_git(root, &["rev-parse", "HEAD"])
}

/// Resolve the default branch ref, **local-first** (git-native-v2 D1): local
/// `main` → local `master` → `origin/HEAD` target → `origin/main` →
/// `origin/master`. The local ref is the anchor for all range semantics so
/// long-lived local work without push never drifts the base (the previous
/// origin-first order left every change's base at the last push position).
pub fn resolve_default_branch_ref(root: &Path) -> Result<String> {
    for candidate in ["main", "master"] {
        if git_ref_exists(root, candidate) {
            return Ok(candidate.to_string());
        }
    }
    if let Ok(sym) = run_git(root, &["symbolic-ref", "refs/remotes/origin/HEAD"])
        && let Some(name) = sym.strip_prefix("refs/remotes/origin/")
    {
        let remote = format!("origin/{name}");
        if git_ref_exists(root, &remote) {
            return Ok(remote);
        }
        if git_ref_exists(root, name) {
            return Ok(name.to_string());
        }
    }
    for candidate in ["origin/main", "origin/master"] {
        if git_ref_exists(root, candidate) {
            return Ok(candidate.to_string());
        }
    }
    bail!("unable to resolve default branch (tried main, master, origin/main, origin/master)");
}

/// Local/remote divergence INFO hint (deduplicated per process): printed once
/// when the resolved local default ref leads its `origin/*` counterpart, so
/// users know the effective range base is ahead of the remote. `pub(crate)`
/// only in core — callers outside the crate use [`effective_range_base`].
fn hint_local_ahead_of_remote(root: &Path, default_ref: &str) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static HINT_SHOWN: AtomicBool = AtomicBool::new(false);
    if HINT_SHOWN.swap(true, Ordering::Relaxed) {
        return;
    }
    let Some(remote) = default_ref
        .strip_prefix("main")
        .map(|_| "origin/main")
        .or_else(|| default_ref.strip_prefix("master").map(|_| "origin/master"))
    else {
        return;
    };
    if !git_ref_exists(root, remote) {
        return;
    }
    let local_ahead = run_git(
        root,
        &["rev-list", "--count", &format!("{remote}..{default_ref}")],
    )
    .map(|s| s.trim().parse::<i64>().unwrap_or(0))
    .unwrap_or(0);
    if local_ahead <= 0 {
        return;
    }
    let remote_ahead = run_git(
        root,
        &["rev-list", "--count", &format!("{default_ref}..{remote}")],
    )
    .map(|s| s.trim().parse::<i64>().unwrap_or(0))
    .unwrap_or(0);
    if remote_ahead > 0 {
        eprintln!(
            "INFO: local `{default_ref}` diverged from `{remote}` (local +{local_ahead}/remote +{remote_ahead}); range anchors use the local ref"
        );
    } else {
        eprintln!(
            "INFO: local `{default_ref}` is ahead of `{remote}` (+{local_ahead}); range anchors use the local ref"
        );
    }
}

/// Effective diff-range base (git-native-v2 D1): the **live** merge-base of
/// the local default branch with HEAD, the single entry point for all range
/// semantics (locked-rule gate, specs landing, change diff/commit count,
/// staleness). Computing it on demand means the range always covers exactly
/// the branch's own work and shrinks automatically after merge/rebase — no
/// stored state, immune to unpushed accumulation. Fails when git is
/// unavailable or no default ref exists; callers fall back to the stored
/// base_sha (fail-open, same as the pre-v2 behavior).
pub fn effective_range_base(root: &Path) -> Result<String> {
    let default_ref = resolve_default_branch_ref(root)?;
    if default_ref.starts_with("main") || default_ref.starts_with("master") {
        hint_local_ahead_of_remote(root, &default_ref);
    }
    merge_base_sha(root, &default_ref)
}

// NOTE: do NOT insert `--` before `reference` here. `rev-parse --verify`
// treats `--` as an end-of-options separator, which makes git interpret the
// following argument as a PATH rather than a ref — so `-- origin/main` would
// always fail. All callers pass validated refs (hardcoded literals or values
// sanitized by `validate_user_git_ref`), so option injection is not a concern.
pub fn git_ref_exists(root: &Path, reference: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn is_default_branch(root: &Path, branch: &str) -> Result<bool> {
    let default_ref = resolve_default_branch_ref(root)?;
    let default_name = default_ref
        .strip_prefix("origin/")
        .unwrap_or(default_ref.as_str());
    Ok(branch == default_name || branch == default_ref)
}

pub fn working_tree_clean(root: &Path) -> Result<bool> {
    let status = run_git(root, &["status", "--porcelain"])?;
    Ok(status.trim().is_empty())
}

pub fn merge_base_sha(root: &Path, base_ref: &str) -> Result<String> {
    run_git(root, &["merge-base", base_ref, "HEAD"])
}

pub fn branch_diff(root: &Path, base_sha: &str) -> Result<String> {
    run_git(
        root,
        &["diff", "--find-renames", &format!("{base_sha}...HEAD")],
    )
}

pub fn branch_has_upstream(root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(root)
        .output()
        .map_err(|err| anyhow!("git upstream check failed: {err}"))?;
    Ok(output.status.success())
}

/// Resolve the absolute `.git` directory path.
pub fn git_common_dir(root: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(root)
        .output()
        .map_err(|e| anyhow!("git rev-parse --git-common-dir failed to spawn: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("not a git repository: {stderr}");
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = Path::new(&raw);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(root.join(path))
    }
}

/// Check if a directory is already a git worktree.
pub fn worktree_exists(root: &Path, path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(root)
        .output()
        .map_err(|e| anyhow!("git worktree list failed to spawn: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let target = path.display().to_string();
    Ok(stdout
        .lines()
        .any(|line| line.starts_with("worktree ") && line.contains(&target)))
}

/// `git worktree add <path> -b <branch> <base_sha>` (creates and checks out).
pub fn worktree_add(root: &Path, path: &Path, branch: &str, base_sha: &str) -> Result<()> {
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            path.to_str().unwrap(),
            "-b",
            branch,
            base_sha,
        ])
        .current_dir(root)
        .output()
        .map_err(|e| anyhow!("git worktree add failed to spawn: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("worktree create failed: {stderr}");
    }
    Ok(())
}

/// Walk up from `start` looking for a `.git` entry (dir or worktree file).
pub fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if is_git_root(&current) {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn is_git_root(path: &Path) -> bool {
    let git = path.join(".git");
    if let Ok(metadata) = fs::symlink_metadata(&git) {
        return metadata.is_dir() || metadata.is_file();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_git_root() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).expect("create nested dirs");
        fs::create_dir_all(root.join(".git")).expect("create git dir");

        let found = find_git_root(&nested).expect("git root");
        assert_eq!(found, root);
    }

    #[test]
    fn test_find_git_root_none() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        fs::create_dir_all(&root).expect("create dir");
        let found = find_git_root(&root);
        assert!(found.is_none());
    }

    fn git(root: &Path, args: &[&str]) {
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

    fn init_repo_with_commit(root: &Path) {
        fs::create_dir_all(root).expect("create repo dir");
        git(root, &["init", "-q", "-b", "main"]);
        git(
            root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ],
        );
    }

    #[test]
    fn test_current_branch_some_on_branch() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        init_repo_with_commit(&root);
        let branch = current_branch(&root)
            .expect("current_branch")
            .expect("branch");
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_current_branch_none_when_detached() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        init_repo_with_commit(&root);
        git(&root, &["checkout", "-q", "--detach"]);
        let branch = current_branch(&root).expect("current_branch");
        assert!(
            branch.is_none(),
            "detached HEAD must map to None, got {branch:?}"
        );
    }

    /// git-native-v2 D1: the default ref resolves LOCAL-first, and
    /// `effective_range_base` = merge-base(local default, HEAD) — unpushed
    /// local accumulation must NOT drift the range anchor to origin.
    #[test]
    fn effective_range_base_is_local_first_and_live() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        init_repo_with_commit(&root);
        // origin/main exists at the same commit; local main then moves ahead
        // (long-lived local work, never pushed).
        git(&root, &["remote", "add", "origin", root.to_str().unwrap()]);
        git(&root, &["fetch", "-q", "origin"]);
        git(
            &root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "--allow-empty",
                "-qm",
                "local-only",
            ],
        );
        let local_main = run_git(&root, &["rev-parse", "main"]).unwrap();
        assert_eq!(
            resolve_default_branch_ref(&root).unwrap(),
            "main",
            "local main must win over origin/main"
        );
        assert_eq!(
            effective_range_base(&root).unwrap(),
            local_main,
            "effective base must be the LOCAL main merge-base (== HEAD here)"
        );
        // On a feature branch fork point, the base is the live fork point.
        git(&root, &["checkout", "-q", "-b", "sdd/c1"]);
        git(
            &root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "--allow-empty",
                "-qm",
                "branch work",
            ],
        );
        assert_eq!(
            effective_range_base(&root).unwrap(),
            local_main,
            "fork point stays the live merge-base after branch commits"
        );
        // Merge main in: the base advances to the new main tip (range shrinks).
        git(&root, &["checkout", "-q", "main"]);
        git(
            &root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "--allow-empty",
                "-qm",
                "main moves again",
            ],
        );
        let new_main = run_git(&root, &["rev-parse", "main"]).unwrap();
        git(&root, &["checkout", "-q", "sdd/c1"]);
        git(
            &root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "merge",
                "-q",
                "--no-edit",
                "main",
            ],
        );
        assert_eq!(
            effective_range_base(&root).unwrap(),
            new_main,
            "after merge, base must shrink to the new main tip (merge-base)"
        );
    }
}
