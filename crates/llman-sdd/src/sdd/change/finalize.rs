//! `llman sdd change finalize` — unified single-commit close-out.
//!
//! Combines checkpoint (relaxed gates) + docs-only archive + ff-merge in one
//! process, leaving a single dirty tree for one `git commit`. Differs from the
//! `checkpoint` + `archive` pair in two ways (see [`run_finalize`] and the
//! `Finalize` variant in `src/sdd/command.rs`):
//!
//! 1. Does NOT require a clean working tree — the implementation diff stays
//!    dirty so it can be committed together with the finalize metadata.
//! 2. Writes `checkpoint_sha = base_sha` (attach-time merge-base), NOT the
//!    HEAD commit carrying the implementation. For the strict sha semantics,
//!    use `change checkpoint` then `change archive`.

use crate::sdd::change::archive::{archive_name_for, do_archive_rename, do_ff_merge};
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use anyhow::Result;
use std::path::Path;

#[cfg(test)]
use std::process::Command;

#[derive(Debug, Clone)]
pub(crate) struct FinalizeArgs {
    pub(crate) change: String,
    pub(crate) no_check: bool,
}

/// Run `finalize` against a repo rooted at `root`.
///
/// Order (see proposal §3 failure semantics):
/// 1. Read binding; reject if not attached.
/// 2. Relaxed gates (branch match, non-default, no legacy feature_delta).
///    **No clean-tree check, no checkpointed check** — finalize owns those.
/// 3. Idempotent check: if `checkpointed && checkpoint_sha.is_some()`, skip
///    validate + write_binding and go straight to archive rename.
/// 4. Otherwise: run validate (live strict + change stage; `--no-check` skips
///    the BDD runner), then write `checkpointed=true` + `checkpoint_sha=base_sha`.
/// 5. Docs-only archive rename.
pub(crate) fn run_finalize(root: &Path, args: FinalizeArgs) -> Result<()> {
    let change_name = crate::sdd::shared::discovery::resolve_change_id_human(root, &args.change)?;
    validate_sdd_id(&change_name, "change")?;
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);
    let _config = load_required_config(&llmanspec)?;

    // Relaxed gates enforce attach/branch/default/feature_delta but skip
    // clean-tree and `checkpointed` (finalize itself writes the latter).
    let mut binding =
        crate::sdd::change::git_native::enforce_bdd_archive_gates_relaxed(root, &change_name)?;

    // Locked-rule integrity (spec-format r135): @human scenarios under
    // llmanspec/specs/** must be untouched vs base_sha unless acked.
    let acked = crate::sdd::change::lock_gate::rules_edit_acked_for(root, &change_name);
    let lock_issues = crate::sdd::change::lock_gate::check(root, &binding.base_sha, acked);
    for issue in &lock_issues {
        match issue.level {
            crate::sdd::spec::validation::ValidationLevel::Error => {
                eprintln!("{}", issue.message);
                anyhow::bail!("locked-rule gate failed");
            }
            _ => eprintln!("{}", issue.message),
        }
    }

    let already_checkpointed = binding.checkpointed && binding.checkpoint_sha.is_some();
    if already_checkpointed {
        eprintln!(
            "change `{}` already checkpointed (checkpoint_sha={}); proceeding to archive rename",
            change_name,
            binding.checkpoint_sha.as_deref().unwrap_or(""),
        );
    } else {
        // Fast + optional full validation of the live branch tree.
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
                check: !args.no_check,
                no_check: args.no_check,
            },
        )?;

        // Also validate the change documentation itself (proposal/tasks stage).
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

        // Write frontmatter. checkpoint_sha = base_sha (single-commit semantics;
        // the implementation commit has not happened yet so HEAD would be stale).
        binding.checkpointed = true;
        binding.checkpoint_sha = Some(binding.base_sha.clone());
        crate::sdd::change::git_native::write_binding(root, &change_name, &binding)?;
    }

    // Docs-only archive rename + auto ff-merge (r94 / r113).
    //
    // Order is ff-merge THEN rename: merging after a dirty rename restores
    // `changes/<id>/` from the feature tip. Merge first (dirty frontmatter /
    // impl carry across), then rename on the default branch so one follow-up
    // commit lands the archive move. On merge failure, still rename (no
    // rollback) — `do_ff_merge` restores the feature branch best-effort.
    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");
    let change_dir = crate::sdd::shared::discovery::resolve_change_dir(root, &change_name)?;
    let archive_dir = changes_dir.join("archive");
    let archive_name = archive_name_for(&change_name);
    let feature_branch = binding.branch.clone();

    do_ff_merge(root, &feature_branch, &change_name);
    do_archive_rename(&change_dir, &archive_dir, &archive_name)?;

    println!(
        "finalized change `{}` → archive `{archive_name}` on branch `{}` (checkpoint_sha=base_sha=`{}`)",
        change_name, feature_branch, binding.base_sha,
    );

    let default_branch = crate::git_utils::resolve_default_branch_ref(root)
        .map(|r| r.strip_prefix("origin/").unwrap_or(r.as_str()).to_string())
        .unwrap_or_else(|_| "<default>".to_string());
    println!(
        "{}",
        t!("sdd.archive.finalize_next_step", default = default_branch)
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
        // r124: proposal.md frontmatter must not carry lifecycle fields (id,
        // stage) — stage is inferred from on-disk artifacts. Mirror the format
        // produced by `change new` (depends_on only) so the schema guard passes.
        fs::write(
            changes.join("proposal.md"),
            "---\ndepends_on: []\n---\n\n# Proposal\n\n## Why\n\nx\n\n## What Changes\n\nx\n",
        )
        .unwrap();
        // tasks.md all-checked so archive tasks-gate does not interfere.
        fs::write(changes.join("tasks.md"), "# Tasks\n\n- [x] done\n").unwrap();
        // validate requires design.md when tasks.md is present.
        fs::write(
            changes.join("design.md"),
            "# Design\n\nTest fixture design.\n",
        )
        .unwrap();

        // git init, default branch rename, commit, branch off.
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
        // Set default branch name explicitly so is_default_branch sees a stable
        // value on hosts that default to something other than main/master.
        git(&["init", "--initial-branch=main"]);
        // Bypass any commit identity requirement in CI sandboxes.
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        git(&["add", "."]);
        git(&["commit", "-m", "init"]);
        // Record base_sha on main HEAD, then switch to a feature branch.
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

        // Write attach binding manually (mirrors run_attach output) so we don't
        // need a network/merge-base available; base_sha points at main HEAD.
        let binding = ChangeGitBinding {
            branch: "feat/x".to_string(),
            base_sha: base_sha.clone(),
            checkpointed: false,
            checkpoint_sha: None,
        };
        crate::sdd::change::git_native::write_binding(root, change_id, &binding).unwrap();

        (tmp, change_id.to_string(), base_sha)
    }

    #[test]
    fn finalize_writes_checkpointed_and_base_sha_then_archives() {
        // Full happy path: dirty tree → finalize → archive rename, with the
        // internal validate::run exercised against the TempDir root (no chdir).
        // This is the coverage gap flagged in the parent change's verify report
        // (W1); it became possible once validate::run accepted a root parameter.

        // validate's staleness check reads the process-wide LLMANSPEC_BASE_REF
        // env. Another unit test (staleness::invalid_llmanspec_base_ref...)
        // temporarily sets it under ENV_MUTEX; hold the same lock for the whole
        // test so this test's validate can't observe the leaked value. Under
        // `cargo test` (threaded, the CI path) this race otherwise fails ~1/3.
        let _env_lock = crate::test_utils::lock_env();
        // Safety: env mutation only during tests, never in shipped binaries.
        unsafe { std::env::remove_var("LLMANSPEC_BASE_REF") };

        let (tmp, id, base_sha) = setup_repo_with_attached_change("finalize-happy");
        let root = tmp.path();

        // Seed a minimal single-track spec so `validate --specs` has something
        // to pass on. r1 as a @human rule; no runner is invoked (--no-check).
        let sample_dir = root.join("llmanspec/specs/sample");
        fs::create_dir_all(&sample_dir).unwrap();
        fs::write(
            sample_dir.join("sample.feature"),
            "# capability: sample\n\
             # purpose: sample for finalize happy-path test\n\
             # scope: llmanspec/specs/sample\n\n\
             Feature: sample\n\n\
             \x20 @req:r1 @human\n\
             \x20 Scenario: R1\n\
             \x20   System MUST do X.\n",
        )
        .unwrap();
        // Commit the spec so the tree isn't carrying untracked files that would
        // trip staleness warnings (warnings, not errors — but keep it clean).
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

        // Make the tree dirty (simulating uncommitted implementation) to prove
        // finalize does not require a clean tree.
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
            },
        )
        .expect("finalize succeeds");

        // Active change dir is gone; archive entry exists.
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

        // Archived proposal.md carries the finalize semantics: checkpointed=true
        // and checkpoint_sha == base_sha (Route C).
        let proposal = fs::read_to_string(
            root.join("llmanspec/changes/archive")
                .join(&archived_name)
                .join("proposal.md"),
        )
        .unwrap();
        assert!(proposal.contains("checkpointed: true"));
        assert!(
            proposal.contains(&format!("checkpoint_sha: {base_sha}")),
            "expected checkpoint_sha == base_sha in:\n{proposal}"
        );

        // r94: auto ff-merge leaves us on the default branch.
        let branch = crate::git_utils::current_branch(root).unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn finalize_rejects_when_not_attached() {
        // Build repo, then wipe the binding to simulate unattached.
        let (tmp, id, _base) = setup_repo_with_attached_change("finalize-noattach");
        let root = tmp.path();

        // Strip binding fields from proposal.md frontmatter. Keep the r124-legal
        // shape (no id/stage lifecycle fields) so the schema guard stays happy.
        let proposal_path = root.join("llmanspec/changes").join(&id).join("proposal.md");
        let stripped =
            "---\ndepends_on: []\n---\n\n# Proposal\n\n## Why\n\nx\n\n## What Changes\n\nx\n";
        fs::write(&proposal_path, stripped).unwrap();

        // Commit so the tree is clean-ish (doesn't matter; finalize doesn't check).
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
        // Simulate the "binding already written, archive rename pending" state
        // by pre-writing checkpointed=true + checkpoint_sha, then calling finalize.
        let (tmp, id, base_sha) = setup_repo_with_attached_change("finalize-idem");
        let root = tmp.path();

        let binding = ChangeGitBinding {
            branch: "feat/x".to_string(),
            base_sha: base_sha.clone(),
            checkpointed: true,
            checkpoint_sha: Some(base_sha.clone()),
        };
        crate::sdd::change::git_native::write_binding(root, &id, &binding).unwrap();

        run_finalize(
            root,
            FinalizeArgs {
                change: id.clone(),
                no_check: false, // should be ignored because already checkpointed
            },
        )
        .expect("finalize succeeds (idempotent)");

        // active change gone
        assert!(!root.join("llmanspec/changes").join(&id).exists());
    }

    #[test]
    fn finalize_works_unified_regardless_of_bdd_config() {
        // Unified flow: finalize works with or without bdd: block (r94).

        // Same LLMANSPEC_BASE_REF env-race guard as the happy-path test above:
        // hold ENV_MUTEX so validate's staleness check can't read a value leaked
        // by a concurrent staleness unit test under `cargo test` (CI path).
        let _env_lock = crate::test_utils::lock_env();
        // Safety: env mutation only during tests, never in shipped binaries.
        unsafe { std::env::remove_var("LLMANSPEC_BASE_REF") };

        let (tmp, id, _base) = setup_repo_with_attached_change("finalize-unified");
        let root = tmp.path();

        // Seed a minimal spec so validate --specs passes.
        let sample_dir = root.join("llmanspec/specs/sample");
        fs::create_dir_all(&sample_dir).unwrap();
        fs::write(
            sample_dir.join("sample.feature"),
            "# capability: sample\n# purpose: sample\n# scope: llmanspec/specs/sample\n\nFeature: sample\n\n  @req:r1 @human\n  Scenario: R1\n    System MUST do X.\n",
        ).unwrap();
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add sample spec"])
            .current_dir(root)
            .output()
            .unwrap();

        // Flip config to no bdd block (unified — finalize still works).
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
            },
        )
        .expect("unified finalize should succeed without bdd: block");

        // Active change dir is gone; archive entry exists.
        assert!(!root.join("llmanspec/changes").join(&id).exists());
    }

    // Keep this as a compile-time anchor for the helper struct shape so future
    // renames in git_native.rs surface here rather than silently drift.
    #[test]
    fn _binding_shape_anchor() {
        let _ = ChangeGitBinding {
            branch: String::new(),
            base_sha: String::new(),
            checkpointed: false,
            checkpoint_sha: None,
        };
    }
}
