//! `llman sdd change finalize` — unified single-commit close-out (r25).
//!
//! Combines relaxed gates + auto merge (target: --into > base_branch >
//! default; method: --method > config, squash by default) + docs-only
//! archive rename in one process, then **auto-commits** the whole thing as
//! `archive(sdd): <change-id>` (one commit bundling the squash-staged feature
//! diff, frontmatter and rename). `--no-commit` skips the auto commit for
//! CI / pre-commit-hook scenarios.

use crate::sdd::change::archive::{
    archive_name_for, do_archive_rename, do_merge, resolve_merge_method, resolve_merge_target,
};
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct FinalizeArgs {
    pub(crate) change: String,
    pub(crate) no_check: bool,
    pub(crate) no_commit: bool,
    /// Merge target override (r113): --into > binding base_branch > default.
    pub(crate) into: Option<String>,
    /// Merge method override (r113): --method > config `sdd.merge_method` >
    /// built-in default (squash).
    pub(crate) method: Option<String>,
    /// Accepted and ignored since the r135 report-only rework (the interactive
    /// confirmation path is gone); kept so skills can pass it unconditionally.
    #[allow(dead_code)]
    pub(crate) no_interactive: bool,
}

/// Auto-commit the finalized tree as `archive(sdd): <change-id>`. Returns the
/// commit sha when a commit was created, `None` when there was nothing staged.
/// Failure (e.g. a pre-commit hook rejection) is surfaced as an error with
/// recovery guidance — the merge/rename are NOT rolled back.
fn auto_commit(root: &Path, change_id: &str) -> Result<Option<String>> {
    crate::git_utils::run_git(root, &["add", "-A"])?;
    let staged = crate::git_utils::run_git(root, &["diff", "--cached", "--name-only"])?;
    if staged.trim().is_empty() {
        return Ok(None);
    }
    let msg = format!("archive(sdd): {change_id}");
    crate::git_utils::run_git(root, &["commit", "-m", &msg]).map_err(|err| {
        anyhow::anyhow!(
            "auto-commit failed (pre-commit hook or identity?): {err}. \
The archive rename is already done and NOT rolled back. \
Finish manually: `git add -A && git commit -m \"{msg}\"`, \
or re-run `llman sdd change finalize {change_id} --no-commit`."
        )
    })?;
    // Commit succeeded: HEAD is authoritative (no output parsing).
    let sha = crate::git_utils::run_git(root, &["rev-parse", "--short", "HEAD"])?;
    Ok(Some(sha.trim().to_string()))
}

/// Run `finalize` against a repo rooted at `root`.
///
/// Order (design §5):
/// 1. Idempotency: change already renamed into `changes/archive/` → finish a
///    possibly-failed auto commit (unless `--no-commit`) and stop.
/// 2. Relaxed gates (attach/branch/default/feature_delta). No clean-tree check.
/// 3. Locked-rule confirmation (r135; report-only).
/// 4. Validate (live strict + change stage; `--no-check` skips the BDD runner).
/// 5. r137 commit count.
/// 6. Auto merge (r113 target/method resolution; r142 topology guard) +
///    docs-only archive rename.
/// 7. Auto `git commit -m "archive(sdd): <change-id>"` (skip with `--no-commit`).
pub(crate) fn run_finalize(root: &Path, args: FinalizeArgs) -> Result<()> {
    // Idempotency probes must survive an already-archived change: resolution
    // against the active tree fails after the rename, so fall back to an exact
    // match under `changes/archive/` before giving up.
    let change_name =
        match crate::sdd::shared::discovery::resolve_change_id_human(root, &args.change) {
            Ok(name) => name,
            Err(err) => {
                let archived = root
                    .join(LLMANSPEC_DIR_NAME)
                    .join("changes")
                    .join("archive")
                    .join(archive_name_for(&args.change));
                if archived.exists() {
                    args.change.clone()
                } else {
                    return Err(err);
                }
            }
        };
    validate_sdd_id(&change_name, "change")?;
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec)?;

    // 1. Idempotency: rename already done (previous run failed at the commit).
    let changes_dir = llmanspec.join("changes");
    let archive_dir = changes_dir.join("archive");
    let archive_name = archive_name_for(&change_name);
    // Idempotent probe BEFORE resolution: after the rename the active dir is
    // gone and `resolve_change_dir` would error.
    let change_dir = match crate::sdd::shared::discovery::resolve_change_dir(root, &change_name) {
        Ok(dir) => dir,
        Err(_) => changes_dir.join(&change_name),
    };
    if !change_dir.exists() && archive_dir.join(&archive_name).exists() {
        eprintln!(
            "change `{change_name}` was already finalized (archive `{archive_name}`); finishing the auto commit if needed"
        );
        if !args.no_commit {
            auto_commit(root, &change_name)?;
        }
        return Ok(());
    }

    // 2. Relaxed gates.
    let binding =
        crate::sdd::change::git_native::enforce_bdd_archive_gates_relaxed(root, &change_name)?;

    // 3. Locked-rule confirmation / acknowledgement (r135).

    // 4. Validate (unless --no-check): live specs strict + change docs.
    if !args.no_check {
        crate::sdd::commands::validate::run(
            root,
            crate::sdd::commands::validate::ValidateArgs {
                item: None,
                all: false,
                changes: false,
                specs: true,
                item_type: None,
                strict: true,
                json: false,
                compact_json: false,
                stage: None,
                no_interactive: true,
                check: true,
                no_check: false,
            },
        )?;
        crate::sdd::commands::validate::run(
            root,
            crate::sdd::commands::validate::ValidateArgs {
                item: Some(change_name.clone()),
                all: false,
                changes: false,
                specs: false,
                item_type: Some("change".into()),
                strict: true,
                json: false,
                compact_json: false,
                stage: None,
                no_interactive: true,
                check: false,
                no_check: true,
            },
        )?;
    }

    // 5. r137: show commits since the effective base; non-blocking hint.
    crate::sdd::change::git_native::print_commit_count(
        root,
        &crate::sdd::change::lock_gate::effective_range_base(root, Some(&binding.base_sha))
            .unwrap_or_else(|_| binding.base_sha.clone()),
    )?;

    // 6. merge THEN rename (merge first so the dirty impl/frontmatter carry
    //    across; rename lands on the target branch). Merge failure still
    //    renames (no rollback); `do_merge` degrades explicitly (r113) with
    //    the r142 worktree topology guard.
    let feature_branch = binding.branch.clone();
    let method = resolve_merge_method(
        config.sdd.as_ref().and_then(|s| s.merge_method.as_deref()),
        args.method.as_deref(),
    )?;
    let target = resolve_merge_target(root, &binding.base_branch, args.into.as_deref())?;
    do_merge(root, &feature_branch, &target, method, &change_name);
    do_archive_rename(&change_dir, &archive_dir, &archive_name)?;

    // 7. Auto commit (unless --no-commit).
    if args.no_commit {
        println!(
            "finalized change `{change_name}` → archive `{archive_name}` on branch `{target}`"
        );
        eprintln!(
            "auto-commit skipped (--no-commit): the tree is dirty on `{target}`. \
Run `git add -A && git commit -m \"archive(sdd): {change_name}\"` manually."
        );
        return Ok(());
    }
    let sha = auto_commit(root, &change_name)?;
    match sha {
        Some(sha) => {
            println!("finalized change `{change_name}` → archive `{archive_name}` (commit `{sha}`)")
        }
        None => println!(
            "finalized change `{change_name}` → archive `{archive_name}` (nothing to commit)"
        ),
    }
    // r98: a next-step hint after the archive line — confirm the close-out
    // commit on the merge target; push / hosting PR stays opt-in.
    println!(
        "next: confirm the close-out commit on `{target}`; \
         cleanup with `git branch -D {feature_branch}` (squash leaves it unmerged); \
         push / hosting PR only if explicitly requested"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdd::change::git_native::ChangeGitBinding;
    use std::fs;
    use tempfile::TempDir;

    /// Build a minimal BDD-on repo shell in a TempDir: llmanspec/config.yaml,
    /// a git repo on a non-default branch, and a change dir with proposal.md.
    /// Returns (tmp, change_id, base_sha).
    fn setup_repo_with_attached_change(change_id: &str) -> (TempDir, String, String) {
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path();
        let llmanspec = root.join("llmanspec");
        let changes = llmanspec.join("changes").join(change_id);
        let specs = llmanspec.join("specs");
        fs::create_dir_all(&changes).unwrap();
        fs::create_dir_all(&specs).unwrap();

        fs::write(
            llmanspec.join("config.yaml"),
            "schema: spec-driven\nlocale: en\nbdd:\n  run_command: \"cargo test --features bdd\"\n",
        )
        .unwrap();
        fs::write(
            changes.join("proposal.md"),
            "---\ndepends_on: []\n---\n\n# Proposal\n\n## Why\n\nx\n\n## What Changes\n\nx\n",
        )
        .unwrap();
        fs::write(changes.join("tasks.md"), "# Tasks\n\n- [x] done\n").unwrap();
        fs::write(
            changes.join("design.md"),
            "# Design\n\nTest fixture design.\n",
        )
        .unwrap();

        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("git");
            if !out.status.success() {
                panic!(
                    "git {:?} failed: {}",
                    args,
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            out
        };
        git(&["init", "--initial-branch=main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        git(&["add", "."]);
        git(&["commit", "-m", "init"]);
        let base_out = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .expect("git rev-parse");
        let base_sha = String::from_utf8(base_out.stdout)
            .unwrap()
            .trim()
            .to_string();
        git(&["checkout", "-b", "feat/x"]);

        let binding = ChangeGitBinding {
            branch: "feat/x".to_string(),
            base_sha: base_sha.clone(),
            base_branch: String::new(),
        };
        crate::sdd::change::git_native::write_binding(root, change_id, &binding).unwrap();

        (tmp, change_id.to_string(), base_sha)
    }

    /// Seed a minimal locked (@human) sample spec committed to the bound branch.
    fn seed_sample_spec(root: &std::path::Path) {
        let sample_dir = root.join("llmanspec/specs/sample");
        fs::create_dir_all(&sample_dir).unwrap();
        fs::write(
            sample_dir.join("sample.feature"),
            "# capability: sample\n\
             # purpose: sample for finalize tests\n\
             # scope: llmanspec/specs/sample\n\n\
             Feature: sample\n\n\
             \x20 @req:r1 @human\n\
             \x20 Scenario: R1\n\
             \x20   System MUST do X.\n",
        )
        .unwrap();
        std::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(root)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "add sample spec"])
            .current_dir(root)
            .output()
            .unwrap();
    }

    fn last_commit_subject(root: &std::path::Path) -> String {
        let out = std::process::Command::new("git")
            .args(["log", "-1", "--format=%s"])
            .current_dir(root)
            .output()
            .expect("git log");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn finalize_auto_commits_archive_message() {
        let _env_lock = crate::test_utils::lock_env();
        // Safety: env mutation only during tests, never in shipped binaries.
        unsafe { std::env::remove_var("LLMANSPEC_BASE_REF") };

        let (tmp, id, _base_sha) = setup_repo_with_attached_change("finalize-happy");
        let root = tmp.path();
        seed_sample_spec(root);

        // Dirty implementation diff must ride along into the auto commit.
        fs::write(
            root.join("llmanspec/specs/sample/impl.txt"),
            "dirty implementation",
        )
        .unwrap();

        run_finalize(
            root,
            FinalizeArgs {
                change: id.clone(),
                no_check: true,
                no_commit: false,
                into: None,
                method: None,
                no_interactive: true,
            },
        )
        .expect("finalize succeeds");

        // Active change dir gone; archive entry exists.
        assert!(
            !root.join("llmanspec/changes").join(&id).exists(),
            "active change dir should be gone"
        );
        let entries: Vec<_> = std::fs::read_dir(root.join("llmanspec/changes/archive"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        let archived_name = entries
            .iter()
            .find(|n| n.ends_with(&format!("-{id}")))
            .cloned()
            .unwrap_or_else(|| panic!("archive entry not found: {entries:?}"));

        // r25: one auto commit with the fixed subject; tree clean afterwards.
        assert_eq!(last_commit_subject(root), format!("archive(sdd): {id}"));
        let dirty = crate::git_utils::run_git(root, &["status", "--porcelain"]).unwrap();
        assert!(
            dirty.trim().is_empty(),
            "tree must be clean after auto commit"
        );
        // The dirty implementation ride-along is inside the archive commit.
        assert!(
            root.join("llmanspec/changes/archive")
                .join(&archived_name)
                .exists()
        );

        // r94: auto ff-merge leaves us on the default branch.
        let branch = crate::git_utils::current_branch(root).unwrap().unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn finalize_no_commit_leaves_dirty_tree() {
        let _env_lock = crate::test_utils::lock_env();
        // Safety: env mutation only during tests, never in shipped binaries.
        unsafe { std::env::remove_var("LLMANSPEC_BASE_REF") };

        let (tmp, id, _base_sha) = setup_repo_with_attached_change("finalize-nocommit");
        let root = tmp.path();
        seed_sample_spec(root);

        run_finalize(
            root,
            FinalizeArgs {
                change: id.clone(),
                no_check: true,
                no_commit: true,
                into: None,
                method: None,
                no_interactive: true,
            },
        )
        .expect("finalize --no-commit succeeds");

        // Rename happened but no commit. Under the default squash method the
        // feature work stays STAGED (never committed), so HEAD remains at the
        // fixture's base commit and the tree stays dirty for manual close-out.
        assert_eq!(
            last_commit_subject(root),
            "init",
            "no auto commit must be created"
        );
        let dirty = crate::git_utils::run_git(root, &["status", "--porcelain"]).unwrap();
        assert!(
            !dirty.trim().is_empty(),
            "tree must stay dirty after --no-commit finalize"
        );
    }

    #[test]
    fn finalize_squash_single_commit_into_recorded_base_branch() {
        let _env_lock = crate::test_utils::lock_env();
        // Safety: env mutation only during tests, never in shipped binaries.
        unsafe { std::env::remove_var("LLMANSPEC_BASE_REF") };

        let (tmp, id, _base_sha) = setup_repo_with_attached_change("finalize-squash");
        let root = tmp.path();
        seed_sample_spec(root);

        // Record a stacked fork source: base_branch = a non-default branch.
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .unwrap()
        };
        git(&["branch", "stack-base"]);
        let before = {
            let out = std::process::Command::new("git")
                .args(["rev-parse", "stack-base"])
                .current_dir(root)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        let binding = ChangeGitBinding {
            branch: "feat/x".to_string(),
            base_sha: _base_sha.clone(),
            base_branch: "stack-base".to_string(),
        };
        crate::sdd::change::git_native::write_binding(root, &id, &binding).unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-m", "record stacked binding"]);

        run_finalize(
            root,
            FinalizeArgs {
                change: id.clone(),
                no_check: true,
                no_commit: false,
                into: None,
                method: None,
                no_interactive: true,
            },
        )
        .expect("finalize succeeds");

        // r113 v2: the merge target is the recorded base_branch (not the
        // default branch), and the close-out is ONE squash commit there.
        let out = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(root)
            .output()
            .unwrap();
        let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert_eq!(
            branch, "stack-base",
            "must land on the recorded base_branch"
        );
        assert_eq!(last_commit_subject(root), format!("archive(sdd): {id}"));
        let count = crate::git_utils::run_git(
            root,
            &["rev-list", "--count", &format!("{before}..stack-base")],
        )
        .unwrap();
        assert_eq!(
            count.trim(),
            "1",
            "squash close-out must be a single commit"
        );
    }

    #[test]
    fn finalize_into_flag_overrides_target() {
        let _env_lock = crate::test_utils::lock_env();
        unsafe { std::env::remove_var("LLMANSPEC_BASE_REF") };

        let (tmp, id, _base_sha) = setup_repo_with_attached_change("finalize-into");
        let root = tmp.path();
        seed_sample_spec(root);

        run_finalize(
            root,
            FinalizeArgs {
                change: id.clone(),
                no_check: true,
                no_commit: false,
                into: Some("main".to_string()),
                method: None,
                no_interactive: true,
            },
        )
        .expect("finalize --into succeeds");

        let branch = crate::git_utils::current_branch(root).unwrap().unwrap();
        assert_eq!(branch, "main", "--into must override the merge target");
        assert_eq!(last_commit_subject(root), format!("archive(sdd): {id}"));
    }

    #[test]
    fn finalize_rejects_when_not_attached() {
        let (tmp, id, _base) = setup_repo_with_attached_change("finalize-noattach");
        let root = tmp.path();

        let proposal_path = root.join("llmanspec/changes").join(&id).join("proposal.md");
        let stripped =
            "---\ndepends_on: []\n---\n\n# Proposal\n\n## Why\n\nx\n\n## What Changes\n\nx\n";
        fs::write(&proposal_path, stripped).unwrap();
        std::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(root)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "strip"])
            .current_dir(root)
            .output()
            .unwrap();

        let err = run_finalize(
            root,
            FinalizeArgs {
                change: id,
                no_check: true,
                no_commit: false,
                into: None,
                method: None,
                no_interactive: true,
            },
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("attach") || msg.contains("binding"),
            "expected attach/binding error, got: {msg}"
        );
    }

    #[test]
    fn finalize_idempotent_after_partial_failure() {
        // Simulate "rename done, auto-commit failed": pre-create the archive
        // entry, wipe the active dir, then finalize again → finishes the commit.
        let (tmp, id, _base_sha) = setup_repo_with_attached_change("finalize-idem");
        let root = tmp.path();
        let changes_dir = root.join("llmanspec/changes");
        let archived = changes_dir.join("archive").join(archive_name_for(&id));
        fs::create_dir_all(&archived).unwrap();
        fs::write(archived.join("proposal.md"), "archived").unwrap();
        fs::remove_dir_all(changes_dir.join(&id)).unwrap();

        run_finalize(
            root,
            FinalizeArgs {
                change: id.clone(),
                no_check: false, // skipped: idempotent path returns early
                no_commit: false,
                into: None,
                method: None,
                no_interactive: true,
            },
        )
        .expect("finalize succeeds (idempotent)");

        assert_eq!(last_commit_subject(root), format!("archive(sdd): {id}"));
    }

    #[test]
    fn finalize_works_unified_regardless_of_bdd_config() {
        let _env_lock = crate::test_utils::lock_env();
        // Safety: env mutation only during tests, never in shipped binaries.
        unsafe { std::env::remove_var("LLMANSPEC_BASE_REF") };

        let (tmp, id, _base) = setup_repo_with_attached_change("finalize-unified");
        let root = tmp.path();
        seed_sample_spec(root);

        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        )
        .unwrap();

        run_finalize(
            root,
            FinalizeArgs {
                change: id.clone(),
                no_check: true,
                no_commit: false,
                into: None,
                method: None,
                no_interactive: true,
            },
        )
        .expect("unified finalize should succeed without bdd: block");

        assert!(!root.join("llmanspec/changes").join(&id).exists());
    }

    // Compile-time anchor for the binding struct shape (r25: no checkpoint fields).
    #[test]
    fn _binding_shape_anchor() {
        let _ = ChangeGitBinding {
            branch: String::new(),
            base_sha: String::new(),
            base_branch: String::new(),
        };
    }
}
