//! `llman sdd change finalize` — BDD-on single-commit closure.
//!
//! Combines checkpoint (relaxed gates) + docs-only archive in one process,
//! leaving a single dirty tree for one `git commit`. Differs from the
//! `checkpoint` + `archive` pair in two ways (see [`run_finalize`] and the
//! `Finalize` variant in `src/sdd/command.rs`):
//!
//! 1. Does NOT require a clean working tree — the implementation diff stays
//!    dirty so it can be committed together with the finalize metadata.
//! 2. Writes `checkpoint_sha = base_sha` (attach-time merge-base), NOT the
//!    HEAD commit carrying the implementation. For the strict sha semantics,
//!    use `change checkpoint` then `change archive`.

use crate::sdd::change::archive::{archive_name_for, do_archive_rename};
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use anyhow::{Result, bail};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FinalizeArgs {
    pub change: String,
    pub no_check: bool,
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
pub fn run_finalize(root: &Path, args: FinalizeArgs) -> Result<()> {
    validate_sdd_id(&args.change, "change")?;
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec)?;
    if config.bdd.is_none() {
        bail!("`sdd change finalize` requires BDD-on (`bdd:` in config.yaml)");
    }

    // Relaxed gates enforce attach/branch/default/feature_delta but skip
    // clean-tree and `checkpointed` (finalize itself writes the latter).
    let mut binding =
        crate::sdd::change::git_native::enforce_bdd_archive_gates_relaxed(root, &args.change)?;

    let already_checkpointed = binding.checkpointed && binding.checkpoint_sha.is_some();
    if already_checkpointed {
        eprintln!(
            "change `{}` already checkpointed (checkpoint_sha={}); proceeding to archive rename",
            args.change,
            binding.checkpoint_sha.as_deref().unwrap_or(""),
        );
    } else {
        // Fast + optional full validation of the live branch tree.
        crate::sdd::shared::validate::run(crate::sdd::shared::validate::ValidateArgs {
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
        })?;

        // Also validate the change documentation itself (proposal/tasks stage).
        crate::sdd::shared::validate::run(crate::sdd::shared::validate::ValidateArgs {
            item: Some(args.change.clone()),
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
        })?;

        // Write frontmatter. checkpoint_sha = base_sha (single-commit semantics;
        // the implementation commit has not happened yet so HEAD would be stale).
        binding.checkpointed = true;
        binding.checkpoint_sha = Some(binding.base_sha.clone());
        crate::sdd::change::git_native::write_binding(root, &args.change, &binding)?;
    }

    // Docs-only archive rename. Same naming as `archive` so the on-disk layout
    // is indistinguishable regardless of which path produced it.
    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");
    let change_dir = changes_dir.join(&args.change);
    let archive_dir = changes_dir.join("archive");
    let archive_name = archive_name_for(&args.change);
    do_archive_rename(&change_dir, &archive_dir, &archive_name)?;

    println!(
        "finalized change `{}` → archive `{archive_name}` on branch `{}` (checkpoint_sha=base_sha=`{}`)",
        args.change, binding.branch, binding.base_sha,
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
            format!(
                "---\nid: {change_id}\nstage: full\n---\n\n# Proposal\n\n## Why\n\nx\n\n## What Changes\n\nx\n"
            ),
        )
        .unwrap();
        // tasks.md all-checked so archive tasks-gate does not interfere.
        fs::write(changes.join("tasks.md"), "# Tasks\n\n- [x] done\n").unwrap();

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
    fn finalize_rejects_when_not_attached() {
        // Build repo, then wipe the binding to simulate unattached.
        let (tmp, id, _base) = setup_repo_with_attached_change("finalize-noattach");
        let root = tmp.path();

        // Strip binding fields from proposal.md frontmatter.
        let proposal_path = root.join("llmanspec/changes").join(&id).join("proposal.md");
        let stripped = format!(
            "---\nid: {id}\nstage: full\n---\n\n# Proposal\n\n## Why\n\nx\n\n## What Changes\n\nx\n"
        );
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
    fn finalize_rejects_bdd_off() {
        // Same as not-attached setup, but flip config to BDD-off.
        let (tmp, id, _base) = setup_repo_with_attached_change("finalize-bddoff");
        let root = tmp.path();

        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        )
        .unwrap();

        let err = run_finalize(
            root,
            FinalizeArgs {
                change: id,
                no_check: true,
            },
        )
        .unwrap_err();
        assert!(format!("{err}").contains("BDD-on"), "expected BDD-on error");
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
