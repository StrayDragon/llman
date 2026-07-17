//! Create a draft change directory with a minimal `proposal.md` skeleton.
//!
//! Does **not** create delta specs (`llman sdd change delta skeleton`) or other
//! planning artifacts — those are added by propose/authoring helpers.

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct NewArgs {
    pub change: String,
    pub force: bool,
}

const PROPOSAL_SKELETON: &str = "\
---
depends_on: []
---

## Why

TODO: Why is this change needed?

## What Changes

TODO: Bullet list of what changes.
";

pub fn run(root: &Path, args: NewArgs) -> Result<()> {
    validate_sdd_id(&args.change, "change")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let _config = load_required_config(&llmanspec_dir)?;

    let change_dir = change_dir(root, &args.change);
    let proposal_path = proposal_path(root, &args.change);

    if proposal_path.exists() && !args.force {
        return Err(anyhow!(
            "change proposal already exists: {} (pass --force to overwrite)",
            proposal_path.display()
        ));
    }

    fs::create_dir_all(&change_dir)?;
    atomic_write_with_mode(&proposal_path, PROPOSAL_SKELETON.as_bytes(), None)?;
    println!("{}", proposal_path.display());
    Ok(())
}

fn change_dir(root: &Path, change_id: &str) -> PathBuf {
    root.join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join(change_id)
}

fn proposal_path(root: &Path, change_id: &str) -> PathBuf {
    change_dir(root, change_id).join("proposal.md")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdd::project::init;
    use tempfile::TempDir;

    fn init_project(root: &Path) {
        init::run(root, None, false).expect("sdd init");
    }

    #[test]
    fn creates_proposal_skeleton() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);

        run(
            root,
            NewArgs {
                change: "add-sample-change".into(),
                force: false,
            },
        )
        .unwrap();

        let proposal = root.join("llmanspec/changes/add-sample-change/proposal.md");
        assert!(proposal.exists());
        let content = fs::read_to_string(&proposal).unwrap();
        assert!(content.contains("depends_on: []"));
        assert!(content.contains("## Why"));
        assert!(content.contains("## What Changes"));
        assert!(
            !root
                .join("llmanspec/changes/add-sample-change/specs")
                .exists()
        );
    }

    #[test]
    fn rejects_existing_without_force() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);

        let args = NewArgs {
            change: "add-sample-change".into(),
            force: false,
        };
        run(root, args.clone()).unwrap();
        assert!(run(root, args).is_err());
    }

    #[test]
    fn force_overwrites_proposal() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);

        let change = "add-sample-change";
        run(
            root,
            NewArgs {
                change: change.into(),
                force: false,
            },
        )
        .unwrap();

        let proposal = proposal_path(root, change);
        fs::write(&proposal, "## Why\nOld content\n").unwrap();

        run(
            root,
            NewArgs {
                change: change.into(),
                force: true,
            },
        )
        .unwrap();

        let content = fs::read_to_string(&proposal).unwrap();
        assert!(content.contains("depends_on: []"));
        assert!(content.contains("TODO: Why is this change needed?"));
    }
}
