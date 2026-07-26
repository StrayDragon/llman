use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::shared::tasks;
use anyhow::{Result, anyhow};
use chrono::Utc;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ArchiveArgs {
    pub change: Option<String>,
    pub skip_specs: bool,
    pub dry_run: bool,
    pub force: bool,
    pub no_interactive: bool,
}

pub fn run(args: ArchiveArgs) -> Result<()> {
    run_with_root(Path::new("."), args)
}

fn run_with_root(root: &Path, args: ArchiveArgs) -> Result<()> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    let archive_config = config.archive_config();

    let raw_name = args
        .change
        .as_ref()
        .ok_or_else(|| anyhow!(t!("sdd.archive.change_required")))?;
    let change_name = crate::sdd::shared::discovery::resolve_change_id_human(root, raw_name)?;
    validate_sdd_id(&change_name, "change")?;
    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");
    let change_dir = changes_dir.join(&change_name);

    if !change_dir.exists() {
        return Err(anyhow!(t!(
            "sdd.archive.change_not_found",
            id = change_name
        )));
    }

    if !args.force {
        let tasks_path = change_dir.join("tasks.md");
        if let Some(report) = tasks::parse_tasks_file(&tasks_path)? {
            if report.pending > 0 {
                eprintln!(
                    "{}",
                    t!("sdd.archive.task_gate_blocked", pending = report.pending)
                );
                for item in &report.items {
                    if matches!(item.status, tasks::TaskStatus::Pending) {
                        eprintln!("{}", t!("sdd.archive.task_gate_item", task = item.text));
                    }
                }
                eprintln!("{}", t!("sdd.archive.task_gate_options"));
                return Err(anyhow!("archive blocked by unchecked tasks"));
            }

            if let Some(min_ratio) = archive_config.min_completion_ratio() {
                let actual = report.completion_ratio();
                if actual < min_ratio {
                    let ratio_pct = (actual * 100.0) as u32;
                    let min_pct = (min_ratio * 100.0) as u32;
                    return Err(anyhow!(
                        "{}",
                        t!(
                            "sdd.archive.task_completion_low",
                            ratio = ratio_pct,
                            min = min_pct
                        )
                    ));
                }
            }
        }
    }

    let archive_dir = changes_dir.join("archive");
    let archive_name = archive_name_for(&change_name);
    let archive_path = archive_dir.join(&archive_name);

    if args.dry_run {
        print_archive_move(&change_dir, &archive_path);
        return Ok(());
    }

    // Capture feature branch / gates before any mutation.
    // Strict gates (attach / branch / clean / checkpointed) unless `--force`.
    let feature_branch = if args.force {
        match crate::sdd::change::git_native::read_binding(root, &change_name) {
            Ok(Some(b)) => Some(b.branch),
            _ => None,
        }
    } else {
        Some(crate::sdd::change::git_native::enforce_bdd_archive_gates(root, &change_name)?.branch)
    };

    // r113 outcomes: docs archived + best-effort ff-merge; rename is never rolled
    // back. Order is ff-merge THEN rename: a dirty rename before merge is restored
    // from the feature tip (committed tree still has changes/<id>/).
    if let Some(ref branch) = feature_branch {
        do_ff_merge(root, branch, &change_name);
    }

    do_archive_rename(&change_dir, &archive_dir, &archive_name)?;

    println!(
        "{}",
        t!(
            "sdd.archive.archived",
            change = change_name,
            archive = archive_name
        )
    );

    Ok(())
}

/// Try `git merge --ff-only <feature>` into the default branch.
///
/// On success: stay on the default branch so the caller can rename docs and
/// commit once (r94/r113). On failure: print a token-friendly hint and
/// best-effort restore the original branch (rename still proceeds afterward).
///
/// When the working tree is dirty (finalize's intentional single-commit path),
/// local changes are stashed across checkout/merge and popped afterward so
/// they land on the default branch.
pub(crate) fn do_ff_merge(root: &Path, feature_branch: &str, change_name: &str) {
    let default_ref = match crate::sdd::change::git_native::resolve_default_branch_ref(root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "ff-merge: cannot resolve default branch ({e}); run manually: git switch <default> && git merge --ff-only {feature_branch}"
            );
            return;
        }
    };
    let default_name = default_ref
        .strip_prefix("origin/")
        .unwrap_or(default_ref.as_str())
        .to_string();

    let original = match crate::sdd::change::git_native::current_branch(root) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "ff-merge: cannot detect current branch ({e}); run manually: git switch {default_name} && git merge --ff-only {feature_branch}"
            );
            return;
        }
    };

    let stashed = stash_if_dirty(root);

    // Switch to default branch.
    let checkout_ok = Command::new("git")
        .args(["checkout", &default_name])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !checkout_ok {
        eprintln!(
            "ff-merge: failed to checkout `{default_name}`; run manually: git switch {default_name} && git merge --ff-only {feature_branch}"
        );
        pop_stash_if(root, stashed);
        return;
    }

    // Attempt ff-only merge.
    let merge = Command::new("git")
        .args(["merge", "--ff-only", feature_branch])
        .current_dir(root)
        .output();
    match merge {
        Ok(o) if o.status.success() => {
            println!("ff-merged `{feature_branch}` into `{default_name}` ({change_name})");
            pop_stash_if(root, stashed);
            // Stay on default — caller renames docs and commits once (r94).
        }
        Ok(o) => {
            let reason = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let reason = if reason.is_empty() {
                format!("exit code {}", o.status.code().unwrap_or(-1))
            } else {
                reason
            };
            eprintln!(
                "ff-merge failed: {reason}; run manually: git switch {default_name} && git merge --ff-only {feature_branch}"
            );
            let _ = Command::new("git")
                .args(["checkout", &original])
                .current_dir(root)
                .output();
            pop_stash_if(root, stashed);
        }
        Err(e) => {
            eprintln!(
                "ff-merge failed: {e}; run manually: git switch {default_name} && git merge --ff-only {feature_branch}"
            );
            let _ = Command::new("git")
                .args(["checkout", &original])
                .current_dir(root)
                .output();
            pop_stash_if(root, stashed);
        }
    }
}

fn stash_if_dirty(root: &Path) -> bool {
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);
    if !dirty {
        return false;
    }
    Command::new("git")
        .args([
            "stash",
            "push",
            "--include-untracked",
            "-m",
            "llman-sdd-ff-merge",
        ])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn pop_stash_if(root: &Path, stashed: bool) {
    if !stashed {
        return;
    }
    let ok = Command::new("git")
        .args(["stash", "pop"])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("ff-merge: stash pop failed; your changes are in `git stash` — resolve manually");
    }
}

/// Perform the final rename of `change_dir` into `archive_dir/<archive_name>`,
/// creating `archive_dir` if needed. Shared by `archive` and `finalize`.
///
/// Errors with the localized "archive_exists" message when the target already
/// exists (matches prior behavior).
pub(crate) fn do_archive_rename(
    change_dir: &Path,
    archive_dir: &Path,
    archive_name: &str,
) -> Result<()> {
    let archive_path = archive_dir.join(archive_name);
    fs::create_dir_all(archive_dir)?;
    match fs::rename(change_dir, &archive_path) {
        Ok(()) => Ok(()),
        Err(e)
            if e.kind() == ErrorKind::AlreadyExists
                || e.kind() == ErrorKind::DirectoryNotEmpty
                || archive_path.exists() =>
        {
            Err(anyhow!(t!(
                "sdd.archive.archive_exists",
                name = archive_name
            )))
        }
        Err(e) => Err(e.into()),
    }
}

/// Compute the archive directory name for a change: `YYYY-MM-DD-<change_id>`.
/// Shared by `archive` and `finalize` so both produce identical naming.
pub(crate) fn archive_name_for(change_name: &str) -> String {
    format!("{}-{}", archive_date(), change_name)
}

fn print_archive_move(from: &Path, to: &Path) {
    println!(
        "{}",
        t!(
            "sdd.archive.dry_run_move",
            from = display_llmanspec_path(from),
            to = display_llmanspec_path(to)
        )
    );
}

fn display_llmanspec_path(path: &Path) -> String {
    let display = path.display().to_string();
    if let Some(idx) = display.find(LLMANSPEC_DIR_NAME) {
        return display[idx..].to_string();
    }
    display
}

fn archive_date() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create dir");
        }
        fs::write(path, content).expect("write file");
    }

    fn git(root: &Path, args: &[&str]) -> bool {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn init_repo(root: &Path) {
        git(root, &["init", "-b", "main"]);
        git(root, &["config", "user.name", "t"]);
        git(root, &["config", "user.email", "t@x"]);
        // Commit so default branch exists.
        write_file(&root.join("README"), "repo");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "init"]);
    }

    #[test]
    fn rejects_path_traversal_change_id() {
        let dir = tempdir().expect("tempdir");
        let args = ArchiveArgs {
            change: Some("../oops".to_string()),
            skip_specs: true,
            dry_run: true,
            force: false,
            no_interactive: false,
        };
        let result = run_with_root(dir.path(), args);
        assert!(result.is_err());
    }

    #[test]
    fn archive_blocked_by_pending_tasks() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let config_path = root.join("llmanspec/config.yaml");
        write_file(&config_path, "schema: spec-driven\nlocale: en\n");
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(
            &change_dir.join("proposal.md"),
            "## Why\nTest change for archive gate",
        );
        write_file(
            &change_dir.join("tasks.md"),
            "- [x] Done task\n- [ ] Pending task\n",
        );
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unchecked tasks"));
    }

    #[test]
    fn archive_allowed_when_force_with_pending() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let config_path = root.join("llmanspec/config.yaml");
        write_file(&config_path, "schema: spec-driven\nlocale: en\n");
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(
            &change_dir.join("proposal.md"),
            "## Why\nTest change for archive gate",
        );
        write_file(&change_dir.join("tasks.md"), "- [x] Done\n- [ ] Pending\n");
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: true,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_ok());
    }

    #[test]
    fn archive_passes_with_all_completed() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        init_repo(root);
        write_file(
            &root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        );
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(&change_dir.join("proposal.md"), "## Why\nAll done");
        write_file(&change_dir.join("tasks.md"), "- [x] Done1\n- [x] Done2\n");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "seed change"]);
        git(root, &["checkout", "-b", "feat/x"]);
        let binding = crate::sdd::change::git_native::ChangeGitBinding {
            branch: "feat/x".to_string(),
            base_sha: "abc".to_string(),
            checkpointed: true,
            checkpoint_sha: Some("abc".into()),
        };
        crate::sdd::change::git_native::write_binding(root, "test-change", &binding).unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "checkpoint binding"]);
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_ok());
    }

    #[test]
    fn archive_blocked_by_cancelled_now_pending() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let config_path = root.join("llmanspec/config.yaml");
        write_file(&config_path, "schema: spec-driven\nlocale: en\n");
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(&change_dir.join("proposal.md"), "## Why\nTest");
        write_file(
            &change_dir.join("tasks.md"),
            "- [x] Done\n- [ ] Not needed (cancelled — done)\n",
        );
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unchecked"));
    }

    #[test]
    fn archive_blocked_by_completion_ratio() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let config_path = root.join("llmanspec/config.yaml");
        write_file(
            &config_path,
            "schema: spec-driven\nlocale: en\narchive:\n  min_completion_ratio: 0.8\n",
        );
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(&change_dir.join("proposal.md"), "## Why\nTest");
        write_file(
            &change_dir.join("tasks.md"),
            "- [x] Done\n- [ ] Not needed (cancelled — x)\n- [ ] Also cancelled (cancelled — y)\n",
        );
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unchecked")
                || msg.contains("completion ratio")
                || msg.contains("below minimum"),
            "got: {msg}"
        );
    }

    #[test]
    fn archive_ff_merge_success() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        init_repo(root);
        write_file(
            &root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\nbdd:\n  run_command: \"echo ok\"\n",
        );
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(
            &change_dir.join("proposal.md"),
            "---\nbranch: feat/x\nbase_sha: abc123\n---\n## Why\nTest",
        );
        write_file(&change_dir.join("tasks.md"), "- [x] done\n");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "seed change"]);

        // Create feature branch and commit on it.
        git(root, &["checkout", "-b", "feat/x"]);
        write_file(&root.join("new-file"), "content");
        git(root, &["add", "new-file"]);
        git(root, &["commit", "-m", "feat commit"]);

        // Write attach binding and commit so clean-tree gate passes.
        let binding = crate::sdd::change::git_native::ChangeGitBinding {
            branch: "feat/x".to_string(),
            base_sha: "abc123".to_string(),
            checkpointed: true,
            checkpoint_sha: Some("abc123".into()),
        };
        crate::sdd::change::git_native::write_binding(root, "test-change", &binding).unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "checkpoint"]);

        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        assert!(run_with_root(root, args).is_ok());
        // Docs renamed to archive, active dir gone.
        assert!(!root.join("llmanspec/changes/test-change").exists());
        // ff-merge brought feature tip onto main; stay on default.
        let branch = crate::sdd::change::git_native::current_branch(root).unwrap();
        assert_eq!(branch, "main");
        assert!(
            root.join("new-file").exists(),
            "ff-merge must bring feature commits onto default"
        );
    }

    #[test]
    fn archive_ff_merge_non_ff_downgrades_gracefully() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        init_repo(root);
        write_file(
            &root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        );
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(
            &change_dir.join("proposal.md"),
            "---\nbranch: feat/y\nbase_sha: abc123\n---\n## Why\nTest",
        );
        write_file(&change_dir.join("tasks.md"), "- [x] done\n");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "seed change"]);

        // Create feature branch and commit diverging history.
        git(root, &["checkout", "-b", "feat/y"]);
        write_file(&root.join("feat-file"), "feat");
        git(root, &["add", "feat-file"]);
        git(root, &["commit", "-m", "feat"]);

        // Switch back and advance main (diverging).
        git(root, &["checkout", "main"]);
        write_file(&root.join("main-file"), "main");
        git(root, &["add", "main-file"]);
        git(root, &["commit", "-m", "main-only"]);

        // Switch to feat/y for archive (simulating checkpoint flow).
        git(root, &["checkout", "feat/y"]);

        let binding = crate::sdd::change::git_native::ChangeGitBinding {
            branch: "feat/y".to_string(),
            base_sha: "abc123".to_string(),
            checkpointed: true,
            checkpoint_sha: Some("abc123".into()),
        };
        crate::sdd::change::git_native::write_binding(root, "test-change", &binding).unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "checkpoint"]);

        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        // Archive succeeds (rename happened) even though ff-merge fails.
        assert!(run_with_root(root, args).is_ok());
        assert!(!root.join("llmanspec/changes/test-change").exists());
        // Active change dir is gone; archive entry exists.
        let mut found = false;
        for entry in fs::read_dir(root.join("llmanspec/changes/archive")).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            if name.contains("test-change") {
                found = true;
            }
        }
        assert!(found, "archive entry not found");
        // On ff-merge failure, restore to the feature branch (best-effort).
        let branch = crate::sdd::change::git_native::current_branch(root).unwrap();
        assert_eq!(branch, "feat/y");
    }
}
