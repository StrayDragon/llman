//! Worktree support for `change start --worktree` (r116).
//!
//! Creates a linked worktree at `<repo>/.git/sdd/worktrees/<dir>/` so
//! multiple changes can be worked on in parallel without switching branches.

use crate::sdd::project::config::SddConfig;
use anyhow::{Result, anyhow, bail};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Create a linked worktree for a change and return its absolute path.
pub fn run_start_worktree(
    root: &Path,
    change_id: &str,
    branch: &str,
    base_sha: &str,
    config: &SddConfig,
) -> Result<PathBuf> {
    // Resolve worktree root: config override or default `<repo>/.git/sdd/worktrees/`.
    let git_dir = git_dir_absolute(root)?;
    let wt_root = worktree_root(&git_dir, config);

    // Compute directory name.
    let wt_name = worktree_dir_name(change_id, config);
    let wt_path = wt_root.join(&wt_name);

    // Reuse-if-checked-out.
    if wt_path.exists() {
        if git_worktree_exists(root, &wt_path)? {
            println!(
                "worktree `{}` already checked out; reusing {}",
                wt_name,
                wt_path.display()
            );
            return Ok(wt_path);
        }
        bail!(
            "worktree path `{}` exists but is not a git worktree; remove it or choose a different change id",
            wt_path.display()
        );
    }

    // depends_on guard: block if any dependency is not in Full stage.
    check_depends_on_guard(root, change_id)?;

    fs::create_dir_all(&wt_root)?;

    // git worktree add <path> <base_sha> (creates and checks out the branch)
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            wt_path.to_str().unwrap(),
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

    println!("created worktree `{}` → {}", wt_name, wt_path.display());
    Ok(wt_path)
}

/// Inline SHA-256 → base32 (Crockford, lowercase) → first 8 chars — deterministic, no extra dep.
fn worktree_dir_name(change_id: &str, config: &SddConfig) -> String {
    let naming = config
        .sdd
        .as_ref()
        .and_then(|f| f.worktree_naming.as_deref())
        .unwrap_or("id");

    match naming {
        "hash" => {
            let mut hasher = Sha256::new();
            hasher.update(change_id.as_bytes());
            let digest = hasher.finalize();
            // Crockford base32, lowercase, first 8.
            let (encoded, _) = base32_encode(&digest);
            encoded[..8.min(encoded.len())].to_lowercase()
        }
        _ => change_id.to_string(),
    }
}

/// Encode bytes as Crockford base32 (lowercase).
fn base32_encode(bytes: &[u8]) -> (String, usize) {
    const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
    let mut output = String::new();
    let mut bits = 0u16;
    let mut bit_count = 0u8;

    for &byte in bytes {
        bits = (bits << 8) | byte as u16;
        bit_count += 8;

        while bit_count >= 5 {
            bit_count -= 5;
            let idx = ((bits >> bit_count) & 0x1f) as usize;
            output.push(ALPHABET[idx] as char);
        }
    }

    if bit_count > 0 {
        let idx = ((bits << (5 - bit_count)) & 0x1f) as usize;
        output.push(ALPHABET[idx] as char);
    }

    (output, bit_count as usize)
}

/// Resolve the worktree root directory.
fn worktree_root(git_dir: &Path, config: &SddConfig) -> PathBuf {
    if let Some(root) = config.sdd.as_ref().and_then(|f| f.worktree_root.as_deref()) {
        return root.to_path_buf();
    }
    git_dir.join("sdd").join("worktrees")
}

/// Resolve the absolute `.git` directory path.
fn git_dir_absolute(root: &Path) -> Result<PathBuf> {
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
fn git_worktree_exists(root: &Path, path: &Path) -> Result<bool> {
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

/// Check depends_on guard: any dependency not in archive or Full stage → bail.
fn check_depends_on_guard(root: &Path, change_id: &str) -> Result<()> {
    let proposal_path = root
        .join("llmanspec")
        .join("changes")
        .join(change_id)
        .join("proposal.md");

    let content = match fs::read_to_string(&proposal_path) {
        Ok(c) => c,
        Err(_) => return Ok(()), // no proposal → nothing to check
    };

    let (yaml_str, _) = crate::sdd::spec::frontmatter::split_frontmatter(&content);
    let Some(yaml_str) = yaml_str else {
        return Ok(());
    };

    let parsed: serde_yaml::Value = match serde_yaml::from_str(&yaml_str) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    let deps: Vec<String> = parsed
        .get("depends_on")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    for dep_id in &deps {
        let dep_dir = root.join("llmanspec").join("changes").join(dep_id);
        let archive_dir = root.join("llmanspec").join("changes").join("archive");

        // Check if archived.
        let archived = archive_dir.exists()
            && fs::read_dir(&archive_dir)
                .ok()
                .map(|entries| {
                    entries.filter_map(|e| e.ok()).any(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .contains(&format!("-{dep_id}"))
                    })
                })
                .unwrap_or(false);

        if archived {
            continue;
        }

        if !dep_dir.exists() {
            bail!(
                "depends_on `{dep_id}` not found; complete or archive it before starting `{change_id}`"
            );
        }

        let stage = crate::sdd::spec::validation::determine_stage(&dep_dir);
        if !matches!(stage, crate::sdd::spec::validation::ChangeStage::Full) {
            bail!(
                "depends_on `{dep_id}` is not in 'full' stage (current: {}); complete it before starting `{change_id}` in a worktree",
                stage.as_str()
            );
        }
    }

    Ok(())
}

/// Prune stale worktrees: remove directories under `<repo>/.git/sdd/worktrees/`
/// whose change has been archived or whose proposal no longer exists.
pub fn run_worktree_prune(root: &Path, config: &SddConfig) -> Result<()> {
    let git_dir = git_dir_absolute(root)?;
    let wt_root = worktree_root(&git_dir, config);
    if !wt_root.exists() {
        println!("no worktrees to prune (directory does not exist)");
        return Ok(());
    }

    let changes_dir = root.join("llmanspec").join("changes");
    let archive_dir = changes_dir.join("archive");

    for entry in fs::read_dir(&wt_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip non-change-id directories (unlikely but safe).
        if name == "." || name == ".." {
            continue;
        }

        let change_dir = changes_dir.join(&name);
        let proposal_exists = change_dir.join("proposal.md").exists();

        // Check if archived.
        let archived = archive_dir.exists()
            && fs::read_dir(&archive_dir)
                .ok()
                .map(|entries| {
                    entries.filter_map(|e| e.ok()).any(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .contains(&format!("-{name}"))
                    })
                })
                .unwrap_or(false);

        if archived || !proposal_exists {
            // Prune: remove git worktree metadata + directory.
            println!("pruning worktree `{name}` ({})", path.display());
            let _ = Command::new("git")
                .args(["worktree", "remove", "--force", path.to_str().unwrap()])
                .current_dir(root)
                .output();
            // Also nuke the directory if git left it behind.
            let _ = fs::remove_dir_all(&path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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

    fn init_repo(root: &Path) {
        git(root, &["init", "-b", "main"]);
        git(root, &["config", "user.name", "t"]);
        git(root, &["config", "user.email", "t@x"]);
        std::fs::write(root.join("README"), "repo").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "init"]);
    }

    fn setup_sdd_project(root: &Path, change_id: &str) {
        let lm = root.join("llmanspec");
        fs::create_dir_all(lm.join("specs")).unwrap();
        fs::create_dir_all(lm.join("changes/archive")).unwrap();
        fs::write(lm.join("config.yaml"), "schema: spec-driven\nlocale: en\n").unwrap();
        let change_dir = lm.join("changes").join(change_id);
        fs::create_dir_all(&change_dir).unwrap();
        fs::write(
            change_dir.join("proposal.md"),
            format!("---\nid: {change_id}\n---\n## Why\nTest\n\n## What Changes\nTest\n"),
        )
        .unwrap();
        fs::write(change_dir.join("design.md"), "# Design\n").unwrap();
        fs::write(change_dir.join("tasks.md"), "- [x] t1\n").unwrap();
    }

    fn default_config() -> SddConfig {
        SddConfig::default()
    }

    #[test]
    fn test_worktree_create_and_reuse() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        init_repo(root);
        setup_sdd_project(root, "wt-test");
        let base = "HEAD";

        let config = default_config();
        let path = run_start_worktree(root, "wt-test", "sdd/wt-test", base, &config).unwrap();
        assert!(path.exists());
        assert!(path.join(".git").exists()); // linked worktree git metadata

        // Reuse: calling again should return the same path without error.
        let path2 = run_start_worktree(root, "wt-test", "sdd/wt-test", base, &config).unwrap();
        assert_eq!(path, path2);
    }

    #[test]
    fn test_worktree_dir_name_id_default() {
        let config = default_config();
        let name = worktree_dir_name("my-change", &config);
        assert_eq!(name, "my-change");
    }

    #[test]
    fn test_worktree_dir_name_hash_deterministic() {
        let mut config = default_config();
        config.sdd = Some(crate::sdd::project::config::FlowConfig {
            worktree_naming: Some("hash".to_string()),
            ..Default::default()
        });
        let a = worktree_dir_name("my-change", &config);
        let b = worktree_dir_name("my-change", &config);
        assert_eq!(a, b);
        assert_eq!(a.len(), 8);
        // Must be lowercase letters/digits only.
        assert!(
            a.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
    }

    #[test]
    fn test_worktree_dir_name_hash_different_ids() {
        let mut config = default_config();
        config.sdd = Some(crate::sdd::project::config::FlowConfig {
            worktree_naming: Some("hash".to_string()),
            ..Default::default()
        });
        let a = worktree_dir_name("change-a", &config);
        let b = worktree_dir_name("change-b", &config);
        assert_ne!(a, b);
    }

    #[test]
    fn test_depends_on_guard_blocks_when_dep_not_full() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        init_repo(root);
        setup_sdd_project(root, "parent");
        // dep: proposal only, no design/tasks → draft (not full).
        let dep_dir = root.join("llmanspec/changes/dep-test");
        fs::create_dir_all(&dep_dir).unwrap();
        fs::write(
            dep_dir.join("proposal.md"),
            "## Why\nx\n\n## What Changes\nx\n",
        )
        .unwrap();

        // parent depends on dep-test.
        let parent_dir = root.join("llmanspec/changes/parent");
        fs::write(
            parent_dir.join("proposal.md"),
            "---\ndepends_on:\n  - dep-test\n---\n## Why\nx\n\n## What Changes\nx\n",
        )
        .unwrap();

        let config = default_config();
        let err = run_start_worktree(root, "parent", "sdd/parent", "HEAD", &config).unwrap_err();
        assert!(err.to_string().contains("depends_on"), "got: {err}");
        assert!(err.to_string().contains("dep-test"), "got: {err}");
    }

    #[test]
    fn test_worktree_prune_removes_archived_change() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        init_repo(root);
        setup_sdd_project(root, "prune-me");
        let config = default_config();
        let path = run_start_worktree(root, "prune-me", "sdd/prune-me", "HEAD", &config).unwrap();
        assert!(path.exists());

        // "Archive" the change by moving its docs to archive/.
        let changes_dir = root.join("llmanspec/changes");
        let archive_dir = changes_dir.join("archive");
        fs::create_dir_all(&archive_dir).unwrap();
        fs::rename(
            changes_dir.join("prune-me"),
            archive_dir.join("2026-01-01-prune-me"),
        )
        .unwrap();

        run_worktree_prune(root, &config).unwrap();
        // worktree directory should be gone after prune.
        assert!(!path.exists());
    }
}
