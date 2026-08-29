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

pub fn resolve_default_branch_ref(root: &Path) -> Result<String> {
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
    for candidate in ["origin/main", "origin/master", "main", "master"] {
        if git_ref_exists(root, candidate) {
            return Ok(candidate.to_string());
        }
    }
    bail!("unable to resolve default branch (tried origin/main, origin/master, main, master)");
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
}
