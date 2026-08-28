//! Specs landing gate: live `llmanspec/specs/**` must land on the bound
//! feature branch (relative to `base_sha`) before a change is apply-ready.

use anyhow::Result;
use std::path::Path;

use crate::git_utils::{current_branch, is_default_branch, run_git};
use crate::sdd::change::git_native::{ChangeGitBinding, read_binding};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::spec::validation::{
    ChangeStage, ProposalFrontmatter, check_proposal_frontmatter, determine_stage,
};

/// Paths under the repo that count as live specs for landing detection.
pub const SPECS_PATHSPEC: &str = "llmanspec/specs";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecsLandingStatus {
    /// True when `base_sha...binding.branch` touches `llmanspec/specs/**`.
    pub specs_landed: bool,
    /// Frontmatter exemption: no live-contract edit expected.
    pub skip_specs_landing: bool,
    /// `stage == Full && (specs_landed || skip_specs_landing)`.
    pub ready_to_implement: bool,
    pub stage: ChangeStage,
    /// Short reason when not landed (token-friendly; may guide agents to skills).
    pub detail: Option<String>,
}

impl SpecsLandingStatus {
    /// Agent-oriented one-liner when apply should STOP.
    pub fn not_ready_message(&self, change_id: &str) -> String {
        if self.ready_to_implement {
            return String::new();
        }
        if self.stage != ChangeStage::Full {
            return format!(
                "change `{change_id}` stage={} (not full); run `change start` or `change attach` first. Skill: llman-sdd-propose.",
                self.stage.as_str()
            );
        }
        if let Some(detail) = &self.detail {
            return detail.clone();
        }
        format!(
            "specs not landed: change `{change_id}` is Full but has no llmanspec/specs/ diff on its bound branch. \
Edit live specs on the bound branch and commit (or set skip_specs_landing: true if no contract change). \
Skill: llman-sdd-propose — do NOT re-run change start if already attached. Apply when show --json readyToImplement=true (llman-sdd-apply)."
        )
    }
}

/// Evaluate specs-landing + apply-ready for a change directory.
pub fn evaluate_specs_landing(root: &Path, change_dir: &Path) -> SpecsLandingStatus {
    let stage = determine_stage(change_dir);
    let skip = read_skip_specs_landing(change_dir);
    let binding = read_binding_for_change(root, change_dir);

    if stage != ChangeStage::Full {
        return SpecsLandingStatus {
            specs_landed: false,
            skip_specs_landing: skip,
            ready_to_implement: false,
            stage,
            detail: None,
        };
    }

    let Some(binding) = binding else {
        let msg = "change is Full but Git binding unreadable; re-run `llman sdd change attach` on the feature branch. Skill: llman-sdd-propose.".to_string();
        return SpecsLandingStatus {
            specs_landed: false,
            skip_specs_landing: skip,
            ready_to_implement: skip,
            stage,
            detail: Some(msg),
        };
    };

    let (landed, detail) = match specs_diff_nonempty(root, &binding) {
        Ok(true) => (true, None),
        Ok(false) => (
            false,
            Some(format!(
                "specs not landed: change bound to `{}` but `{}...{}` has no changes under `{SPECS_PATHSPEC}/`. \
Edit live specs on that branch and commit. Skill: llman-sdd-propose (land specs) — do NOT re-run change start if already attached. \
Apply only when `llman sdd show <id> --json` has readyToImplement=true (llman-sdd-apply). \
Or set `skip_specs_landing: true` in proposal frontmatter if this change has no live contract edits.",
                binding.branch, binding.base_sha, binding.branch
            )),
        ),
        Err(err) => (
            false,
            Some(format!(
                "specs landing check failed for branch `{}` (base {}): {err}. \
Ensure the bound branch exists locally; recover by checkout/recreate then `change attach --force` if needed. Skill: llman-sdd-propose.",
                binding.branch, binding.base_sha
            )),
        ),
    };

    SpecsLandingStatus {
        specs_landed: landed,
        skip_specs_landing: skip,
        ready_to_implement: landed || skip,
        stage,
        detail: if landed || skip { None } else { detail },
    }
}

fn read_skip_specs_landing(change_dir: &Path) -> bool {
    let (issues, fm) = check_proposal_frontmatter(change_dir, &[], &[], false);
    let _ = issues;
    fm.skip_specs_landing
}

fn read_binding_for_change(root: &Path, change_dir: &Path) -> Option<ChangeGitBinding> {
    let name = change_dir.file_name()?.to_str()?;
    read_binding(root, name).ok().flatten()
}

/// True when three-dot diff `base_sha...branch` lists any path under live specs.
pub fn specs_diff_nonempty(root: &Path, binding: &ChangeGitBinding) -> Result<bool> {
    let range = format!("{}...{}", binding.base_sha, binding.branch);
    let out = run_git(
        root,
        &[
            "diff",
            "--name-only",
            "--find-renames",
            &range,
            "--",
            SPECS_PATHSPEC,
        ],
    )?;
    // r130: only `.feature` files count as contract landing.
    Ok(out
        .lines()
        .any(|l| l.trim().ends_with(".feature") && !l.trim().is_empty()))
}

/// WARNING when the default branch has uncommitted edits under live specs.
pub fn warn_dirty_specs_on_default_branch(root: &Path) -> Option<String> {
    let branch = current_branch(root).ok()?;
    if !is_default_branch(root, &branch).ok()? {
        return None;
    }
    let status = run_git(root, &["status", "--porcelain", "--", SPECS_PATHSPEC]).ok()?;
    if status.lines().all(|l| l.trim().is_empty()) {
        return None;
    }
    Some(format!(
        "live specs dirty on default branch `{branch}`: do not commit unimplemented contracts to the default branch. \
Switch to the change's bound `sdd/<id>` branch (or `llman sdd change start <id>`) before editing `{SPECS_PATHSPEC}/`. \
See AGENTS.md Specs landing. Skill: llman-sdd-propose."
    ))
}

/// Repo root from `…/llmanspec/changes/<id>`.
pub fn repo_root_from_change_dir(change_dir: &Path) -> Option<&Path> {
    let changes = change_dir.parent()?;
    let llmanspec = changes.parent()?;
    if llmanspec.file_name()?.to_str()? != LLMANSPEC_DIR_NAME {
        return None;
    }
    llmanspec.parent()
}

/// Re-export parsing helper used by validate after frontmatter gains the field.
pub fn skip_from_frontmatter(fm: &ProposalFrontmatter) -> bool {
    fm.skip_specs_landing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_utils::run_git;
    use crate::sdd::change::git_native::{ChangeGitBinding, write_binding};
    use std::fs;
    use tempfile::TempDir;

    fn init_repo() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        run_git(root, &["init"]).unwrap();
        run_git(root, &["config", "user.email", "t@t"]).unwrap();
        run_git(root, &["config", "user.name", "t"]).unwrap();
        run_git(root, &["checkout", "-b", "main"]).unwrap();
        fs::create_dir_all(root.join("llmanspec/specs/sample")).unwrap();
        fs::write(
            root.join("llmanspec/specs/sample/spec.toon"),
            "kind: llman.sdd.spec\nname: sample\npurpose: p\nvalid_scope[1]: x\nrequirements[0]{req_id,title,statement}:\nscenarios[0]{req_id,id,given,when,then}:\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/changes/c1/proposal.md"),
            "---\ndepends_on: []\n---\n\n## Why\nw\n\n## What Changes\n- x\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/design.md"), "# d\n").unwrap();
        fs::write(root.join("llmanspec/changes/c1/tasks.md"), "- [ ] t\n").unwrap();
        run_git(root, &["add", "."]).unwrap();
        run_git(root, &["commit", "-m", "init"]).unwrap();
        tmp
    }

    #[test]
    fn landed_when_specs_commit_on_bound_branch() {
        let tmp = init_repo();
        let root = tmp.path();
        let base = run_git(root, &["rev-parse", "HEAD"]).unwrap();
        let base = base.trim();
        run_git(root, &["checkout", "-b", "sdd/c1"]).unwrap();
        write_binding(
            root,
            "c1",
            &ChangeGitBinding {
                branch: "sdd/c1".into(),
                base_sha: base.to_string(),
                checkpointed: false,
                checkpoint_sha: None,
            },
        )
        .unwrap();
        fs::write(
            root.join("llmanspec/specs/sample/sample.feature"),
            "# capability: sample\n# purpose: updated\n# scope: x\n\nFeature: sample\n",
        )
        .unwrap();
        run_git(root, &["add", "llmanspec/specs"]).unwrap();
        run_git(root, &["commit", "-m", "specs"]).unwrap();

        let st = evaluate_specs_landing(root, &root.join("llmanspec/changes/c1"));
        assert!(st.specs_landed, "{st:?}");
        assert!(st.ready_to_implement);
        assert!(!st.skip_specs_landing);
    }

    #[test]
    fn not_ready_when_full_without_specs_diff() {
        let tmp = init_repo();
        let root = tmp.path();
        let base = run_git(root, &["rev-parse", "HEAD"]).unwrap();
        let base = base.trim().to_string();
        run_git(root, &["checkout", "-b", "sdd/c1"]).unwrap();
        write_binding(
            root,
            "c1",
            &ChangeGitBinding {
                branch: "sdd/c1".into(),
                base_sha: base,
                checkpointed: false,
                checkpoint_sha: None,
            },
        )
        .unwrap();
        // Binding write dirties proposal — commit so only binding changed, no specs.
        run_git(root, &["add", "llmanspec/changes"]).unwrap();
        run_git(root, &["commit", "-m", "bind"]).unwrap();

        let st = evaluate_specs_landing(root, &root.join("llmanspec/changes/c1"));
        assert!(!st.specs_landed, "{st:?}");
        assert!(!st.ready_to_implement);
        assert!(st.detail.as_ref().unwrap().contains("llman-sdd-propose"));
    }

    #[test]
    fn skip_flag_makes_ready_without_specs_diff() {
        let tmp = init_repo();
        let root = tmp.path();
        let base = run_git(root, &["rev-parse", "HEAD"]).unwrap();
        let base = base.trim().to_string();
        run_git(root, &["checkout", "-b", "sdd/c1"]).unwrap();
        fs::write(
            root.join("llmanspec/changes/c1/proposal.md"),
            "---\ndepends_on: []\nbranch: sdd/c1\nbase_sha: {base}\nskip_specs_landing: true\n---\n\n## Why\nw\n\n## What Changes\n- x\n"
                .replace("{base}", &base),
        )
        .unwrap();
        run_git(root, &["add", "."]).unwrap();
        run_git(root, &["commit", "-m", "skip"]).unwrap();

        let st = evaluate_specs_landing(root, &root.join("llmanspec/changes/c1"));
        assert!(st.skip_specs_landing);
        assert!(st.ready_to_implement);
        assert!(!st.specs_landed);
    }
}
