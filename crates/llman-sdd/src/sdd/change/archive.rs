use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::shared::tasks;
use anyhow::{Result, anyhow};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;
use time::OffsetDateTime;
use time::macros::format_description;

#[derive(Debug, Clone)]
pub(crate) struct ArchiveArgs {
    pub(crate) change: Option<String>,
    /// Accepted and ignored: archive never lands specs; flag exists so
    /// change subcommands share one flag matrix.
    #[allow(dead_code)]
    pub(crate) skip_specs: bool,
    pub(crate) dry_run: bool,
    pub(crate) force: bool,
    /// Merge target override (r113). Precedence: --into > binding
    /// base_branch > local default branch.
    pub(crate) into: Option<String>,
    /// Merge method override (r113): `squash` | `ff`. Precedence: --method >
    /// config `sdd.merge_method` > built-in default (squash).
    pub(crate) method: Option<String>,
    /// Accepted and ignored: archive has no interactive mode. Flag-matrix
    /// uniformity across change subcommands.
    #[allow(dead_code)]
    pub(crate) no_interactive: bool,
}

pub(crate) fn run(args: ArchiveArgs) -> Result<()> {
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
    let change_dir = crate::sdd::shared::discovery::resolve_change_dir(root, &change_name)?;

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
    let (feature_branch, base_branch) = if args.force {
        match crate::sdd::change::git_native::read_binding(root, &change_name) {
            Ok(Some(b)) => (Some(b.branch), b.base_branch),
            _ => (None, String::new()),
        }
    } else {
        let binding =
            crate::sdd::change::git_native::enforce_bdd_archive_gates(root, &change_name)?;
        (Some(binding.branch), binding.base_branch)
    };

    // r113 outcomes: docs archived + best-effort merge; rename is never rolled
    // back. Order is merge THEN rename: a dirty rename before merge is restored
    // from the feature tip (committed tree still has changes/<id>/).
    if let Some(ref branch) = feature_branch {
        let method = resolve_merge_method(
            config.sdd.as_ref().and_then(|s| s.merge_method.as_deref()),
            args.method.as_deref(),
        )?;
        let target = resolve_merge_target(root, &base_branch, args.into.as_deref())?;
        do_merge(root, branch, &target, method, &change_name);
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

/// Merge method for the close-out merge (r113).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MergeMethod {
    /// Default: stage the whole feature diff (`git merge --squash`) so the
    /// caller's single auto commit lands feature work + docs rename as ONE
    /// close-out commit on the target branch.
    Squash,
    /// Legacy: `git merge --ff-only` brings feature commits as-is; the caller
    /// then commits the docs rename once.
    Ff,
}

/// Resolve the merge method (r113): `--method` > config `sdd.merge_method` >
/// built-in default `squash`.
pub(crate) fn resolve_merge_method(
    config_method: Option<&str>,
    flag: Option<&str>,
) -> Result<MergeMethod> {
    let raw = flag
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or(config_method.map(str::trim).filter(|s| !s.is_empty()))
        .unwrap_or("squash");
    match raw {
        "squash" => Ok(MergeMethod::Squash),
        "ff" => Ok(MergeMethod::Ff),
        other => anyhow::bail!("invalid merge method `{other}` (expected `squash` or `ff`)"),
    }
}

/// Resolve the merge target (r113): `--into` > binding `base_branch` (when
/// non-empty AND the branch exists locally) > local default branch.
pub(crate) fn resolve_merge_target(
    root: &Path,
    binding_base_branch: &str,
    into: Option<&str>,
) -> Result<String> {
    if let Some(raw) = into {
        let trimmed = raw.trim();
        crate::env_safety::validate_user_git_ref(trimmed)
            .map_err(|e| anyhow!("invalid --into ref: {e}"))?;
        return Ok(trimmed.to_string());
    }
    if !binding_base_branch.trim().is_empty()
        && crate::git_utils::git_ref_exists(root, &format!("refs/heads/{binding_base_branch}"))
    {
        return Ok(binding_base_branch.to_string());
    }
    let default_ref = crate::git_utils::resolve_default_branch_ref(root)?;
    Ok(crate::sdd::change::git_native::local_branch_name(
        &default_ref,
    ))
}

/// User-executable manual command matching the intended merge, printed on any
/// degradation path so nothing fails silently (r113/r142).
fn manual_merge_command(method: MergeMethod, feature: &str, target: &str) -> String {
    match method {
        MergeMethod::Squash => format!(
            "git switch {target} && git merge --squash {feature} && git commit -m \"archive(sdd): <change-id>\""
        ),
        MergeMethod::Ff => format!("git switch {target} && git merge --ff-only {feature}"),
    }
}

/// r142 topology guard: is `branch` checked out in a worktree OTHER than
/// `root`? Returns that worktree's path. The current worktree is excluded so
/// running from the bound feature branch itself never trips the guard.
fn other_worktree_holding(root: &Path, branch: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let root_canon = fs::canonicalize(root).ok();
    let mut current_path: Option<String> = None;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(path.to_string());
        } else if let Some(refs) = line.strip_prefix("branch ") {
            let name = refs.strip_prefix("refs/heads/").unwrap_or(refs);
            if name == branch
                && let Some(path) = current_path.clone()
                && fs::canonicalize(&path).ok().as_deref() != root_canon.as_deref()
            {
                return Some(path);
            }
        }
    }
    None
}

/// Merge `<feature_branch>` into `<target>` using `<method>` (r113/r142).
///
/// Contract: best-effort with EXPLICIT degradation — any failure prints a
/// WARNING plus an executable manual command and returns without error, so
/// the caller still renames docs (never rolled back). Success leaves the repo
/// on `<target>`; the squash method leaves the feature diff staged for the
/// caller's single close-out commit.
pub(crate) fn do_merge(
    root: &Path,
    feature_branch: &str,
    target: &str,
    method: MergeMethod,
    change_name: &str,
) {
    if let Some(holder) = other_worktree_holding(root, target) {
        eprintln!(
            "merge skipped: target branch `{target}` is held by another worktree at `{holder}` \
             (r142 topology guard) — docs archive proceeds on the current branch only. \
             Finish manually in that worktree: {}",
            manual_merge_command(method, feature_branch, target)
        );
        return;
    }

    let original = match crate::git_utils::current_branch(root) {
        Ok(Some(b)) => b,
        Ok(None) => {
            eprintln!(
                "merge skipped: HEAD is detached (no current branch); run manually: {}",
                manual_merge_command(method, feature_branch, target)
            );
            return;
        }
        Err(e) => {
            eprintln!(
                "merge skipped: cannot detect current branch ({e}); run manually: {}",
                manual_merge_command(method, feature_branch, target)
            );
            return;
        }
    };

    let stashed = stash_if_dirty(root);

    let checkout_ok = Command::new("git")
        .args(["checkout", target])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !checkout_ok {
        eprintln!(
            "merge skipped: failed to checkout `{target}`; run manually: {}",
            manual_merge_command(method, feature_branch, target)
        );
        pop_stash_if(root, stashed);
        return;
    }

    let merge_args: &[&str] = match method {
        MergeMethod::Squash => &["merge", "--squash", feature_branch],
        MergeMethod::Ff => &["merge", "--ff-only", feature_branch],
    };
    let merge = Command::new("git")
        .args(merge_args)
        .current_dir(root)
        .output();
    match merge {
        Ok(o) if o.status.success() => match method {
            MergeMethod::Squash => {
                println!(
                    "squash-staged `{feature_branch}` onto `{target}` ({change_name}); \
                     finishing with the close-out commit"
                );
                pop_stash_if(root, stashed);
                // Stay on target — staged diff + docs rename land in the one
                // auto commit (r94).
            }
            MergeMethod::Ff => {
                println!("ff-merged `{feature_branch}` into `{target}` ({change_name})");
                pop_stash_if(root, stashed);
                // Stay on target — caller renames docs and commits once (r94).
            }
        },
        Ok(o) => {
            let reason = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let reason = if reason.is_empty() {
                format!("exit code {}", o.status.code().unwrap_or(-1))
            } else {
                reason
            };
            eprintln!(
                "merge failed: {reason}; run manually: {}",
                manual_merge_command(method, feature_branch, target)
            );
            // A failed squash can leave conflict entries in the index; all
            // valuable state is committed (feature branch + stash above), so a
            // hard reset of the index here is safe and unblocks the checkout.
            if method == MergeMethod::Squash {
                let _ = Command::new("git")
                    .args(["reset", "--hard", "HEAD"])
                    .current_dir(root)
                    .output();
            }
            let _ = Command::new("git")
                .args(["checkout", &original])
                .current_dir(root)
                .output();
            pop_stash_if(root, stashed);
        }
        Err(e) => {
            eprintln!(
                "merge failed: {e}; run manually: {}",
                manual_merge_command(method, feature_branch, target)
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
    OffsetDateTime::now_utc()
        .format(&format_description!("[year]-[month]-[day]"))
        .expect("valid date")
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
            into: None,
            method: None,
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
            into: None,
            method: None,
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
            into: None,
            method: None,
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
            base_branch: String::new(),
        };
        crate::sdd::change::git_native::write_binding(root, "test-change", &binding).unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "checkpoint binding"]);
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            into: None,
            method: None,
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
            into: None,
            method: None,
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
            into: None,
            method: None,
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
    fn archive_ff_method_fast_forwards_clean_target() {
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
            base_branch: String::new(),
        };
        crate::sdd::change::git_native::write_binding(root, "test-change", &binding).unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "checkpoint"]);

        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            into: None,
            method: Some("ff".to_string()),
            no_interactive: true,
        };
        assert!(run_with_root(root, args).is_ok());
        // Docs renamed to archive, active dir gone.
        assert!(!root.join("llmanspec/changes/test-change").exists());
        // ff-merge brought feature tip onto main; stay on default.
        let branch = crate::git_utils::current_branch(root).unwrap().unwrap();
        assert_eq!(branch, "main");
        assert!(
            root.join("new-file").exists(),
            "ff-merge must bring feature commits onto default"
        );
    }

    #[test]
    fn archive_squash_default_merges_diverged_target() {
        // r113 v2: the default squash method succeeds even when the target has
        // advanced — the exact scenario where --ff-only used to hard-fail.
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

        // Feature branch with a commit.
        git(root, &["checkout", "-b", "feat/y"]);
        write_file(&root.join("feat-file"), "feat");
        git(root, &["add", "feat-file"]);
        git(root, &["commit", "-m", "feat"]);

        // Advance main so histories diverge.
        git(root, &["checkout", "main"]);
        write_file(&root.join("main-file"), "main");
        git(root, &["add", "main-file"]);
        git(root, &["commit", "-m", "main-only"]);

        git(root, &["checkout", "feat/y"]);
        let binding = crate::sdd::change::git_native::ChangeGitBinding {
            branch: "feat/y".to_string(),
            base_sha: "abc123".to_string(),
            base_branch: String::new(),
        };
        crate::sdd::change::git_native::write_binding(root, "test-change", &binding).unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "checkpoint"]);

        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            into: None,
            method: None, // config unset → default squash
            no_interactive: true,
        };
        assert!(run_with_root(root, args).is_ok());
        assert!(!root.join("llmanspec/changes/test-change").exists());
        // Squash landed feature content on main (staged; archive leaves the
        // single close-out commit to the caller per r94).
        let branch = crate::git_utils::current_branch(root).unwrap().unwrap();
        assert_eq!(branch, "main");
        assert!(
            root.join("feat-file").exists(),
            "squash must bring feature content onto the target"
        );
    }

    #[test]
    fn archive_ff_method_degrades_explicitly_on_diverged_target() {
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
            base_branch: String::new(),
        };
        crate::sdd::change::git_native::write_binding(root, "test-change", &binding).unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "checkpoint"]);

        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            into: None,
            method: Some("ff".to_string()),
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
        let branch = crate::git_utils::current_branch(root).unwrap().unwrap();
        assert_eq!(branch, "feat/y");
    }

    #[test]
    fn archive_skips_merge_when_target_held_by_other_worktree() {
        // r142 topology guard: `main` checked out in a second worktree → the
        // auto merge is skipped with an explicit WARNING; docs rename proceeds.
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

        git(root, &["checkout", "-b", "feat/y"]);
        write_file(&root.join("feat-file"), "feat");
        git(root, &["add", "feat-file"]);
        git(root, &["commit", "-m", "feat"]);

        // A second worktree (outside the repo dir, inside its own TempDir so
        // parallel tests never collide) holds `main` — the merge target.
        let wt_tmp = tempdir().expect("worktree tmpdir");
        let wt = wt_tmp.path().join("holding-wt");
        assert!(git(
            root,
            &["worktree", "add", wt.to_str().unwrap(), "main"]
        ));

        let binding = crate::sdd::change::git_native::ChangeGitBinding {
            branch: "feat/y".to_string(),
            base_sha: "abc123".to_string(),
            base_branch: String::new(),
        };
        crate::sdd::change::git_native::write_binding(root, "test-change", &binding).unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "checkpoint"]);

        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            into: None,
            method: None,
            no_interactive: true,
        };
        assert!(run_with_root(root, args).is_ok());
        // Rename completed; the repo stays on the feature branch.
        assert!(!root.join("llmanspec/changes/test-change").exists());
        let branch = crate::git_utils::current_branch(root).unwrap().unwrap();
        assert_eq!(branch, "feat/y");
        // The merge was skipped: feature content never reached the target
        // branch in the holding worktree (root stays on feat/y where the file
        // legitimately exists).
        assert!(
            !wt_tmp.path().join("holding-wt/feat-file").exists(),
            "topology guard must skip the auto merge"
        );
    }
}
