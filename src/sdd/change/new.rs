//! Create a draft change directory with a minimal `proposal.md` skeleton.
//!
//! Does **not** create delta specs (`llman sdd change delta skeleton`) or other
//! planning artifacts — those are added by propose/authoring helpers.
//!
//! `--from <description>` (r99): derive a legal, meaningful change id from the
//! description instead of requiring `<CHANGE>`. Naming follows the repo's
//! `llmanspec/AGENTS.md` conventions when declared; otherwise the id is built
//! from the description's semantics (kebab-case, sanitized). Exactly one of
//! `<CHANGE>` or `--from` is required.

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use anyhow::{Result, anyhow, bail};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct NewArgs {
    /// Explicit change id. `None` when the caller passes `--from` instead.
    pub change: Option<String>,
    /// Free-form description to derive the change id from (r99 lightweight path).
    pub from: Option<String>,
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

/// Soft upper bound on derived change id length. This is a hygiene measure
/// (avoids unwieldy directory names), not a naming convention — the convention
/// is whatever the repo's `llmanspec/AGENTS.md` declares.
const DERIVED_ID_MAX_LEN: usize = 60;

pub fn run(root: &Path, args: NewArgs) -> Result<()> {
    match (args.change.as_deref(), args.from.as_deref()) {
        (Some(_), Some(_)) => {
            bail!("<CHANGE> and --from are mutually exclusive; pass one or the other");
        }
        (None, None) => {
            bail!("change id is required: pass <CHANGE> or --from <DESCRIPTION>");
        }
        (Some(id), None) => create_draft(root, id, false, args.force),
        (None, Some(desc)) => {
            let id = derive_change_id(desc)?;
            create_draft(root, &id, true, args.force)
        }
    }
}

/// Create the draft change directory + proposal skeleton. When `derived` is
/// true, stdout additionally announces the derived id so agents/users can see
/// what was generated.
fn create_draft(root: &Path, id: &str, derived: bool, force: bool) -> Result<()> {
    validate_sdd_id(id, "change")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let _config = load_required_config(&llmanspec_dir)?;

    let change_dir = change_dir(root, id);
    let proposal_path = proposal_path(root, id);

    if proposal_path.exists() && !force {
        return Err(anyhow!(
            "change proposal already exists: {} (pass --force to overwrite)",
            proposal_path.display()
        ));
    }

    fs::create_dir_all(&change_dir)?;
    atomic_write_with_mode(&proposal_path, PROPOSAL_SKELETON.as_bytes(), None)?;
    if derived {
        println!("derived change id: {id}");
    }
    println!("{}", proposal_path.display());
    Ok(())
}

/// Derive a legal, meaningful change id from a free-form description.
///
/// This is intentionally heuristic and conservative: it does **not** impose a
/// fixed naming convention (verb prefix, length cap as strict rule, etc.). The
/// only hard requirement is passing [`validate_sdd_id`]. Hygiene measures
/// (lowercase, collapse whitespace/punctuation to `-`, trim, cap length) keep
/// the id readable and filesystem-safe. The agent or user reading the repo's
/// `llmanspec/AGENTS.md` is the authority on project-specific naming style.
pub fn derive_change_id(desc: &str) -> Result<String> {
    let trimmed = desc.trim();
    if trimmed.is_empty() {
        bail!("--from <DESCRIPTION> must be non-empty");
    }
    let mut id = String::with_capacity(trimmed.len());
    let mut prev_dash = true; // suppress leading dashes
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() {
            id.extend(ch.to_lowercase());
            prev_dash = false;
        } else if ch.is_whitespace() || matches!(ch, '_' | '.' | '-' | '/' | '\\') {
            if !prev_dash {
                id.push('-');
                prev_dash = true;
            }
        } else {
            // Punctuation / CJK / etc.: drop (CJK is intentionally dropped to
            // keep ids ASCII-friendly; agents reading AGENTS.md can rename).
            // A dropped char at a word boundary still yields a `-` separator.
            if !prev_dash {
                id.push('-');
                prev_dash = true;
            }
        }
    }
    while id.ends_with('-') {
        id.pop();
    }
    if id.is_empty() {
        bail!(
            "--from <DESCRIPTION> yielded an empty id after sanitizing; \
             provide a description with at least one alphanumeric character"
        );
    }
    if id.len() > DERIVED_ID_MAX_LEN {
        // Truncate at a `-` boundary if possible to avoid splitting a token.
        let cutoff = id[..DERIVED_ID_MAX_LEN]
            .rfind('-')
            .unwrap_or(DERIVED_ID_MAX_LEN);
        id.truncate(cutoff);
        while id.ends_with('-') {
            id.pop();
        }
    }
    validate_sdd_id(&id, "change")?;
    Ok(id)
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
                change: Some("add-sample-change".into()),
                from: None,
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
            change: Some("add-sample-change".into()),
            from: None,
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
                change: Some(change.into()),
                from: None,
                force: false,
            },
        )
        .unwrap();

        let proposal = proposal_path(root, change);
        fs::write(&proposal, "## Why\nOld content\n").unwrap();

        run(
            root,
            NewArgs {
                change: Some(change.into()),
                from: None,
                force: true,
            },
        )
        .unwrap();

        let content = fs::read_to_string(&proposal).unwrap();
        assert!(content.contains("depends_on: []"));
        assert!(content.contains("TODO: Why is this change needed?"));
    }

    #[test]
    fn rejects_both_change_and_from() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);
        let err = run(
            root,
            NewArgs {
                change: Some("add-x".into()),
                from: Some("add x".into()),
                force: false,
            },
        )
        .unwrap_err();
        assert!(format!("{err}").contains("mutually exclusive"));
    }

    #[test]
    fn rejects_neither_change_nor_from() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);
        let err = run(
            root,
            NewArgs {
                change: None,
                from: None,
                force: false,
            },
        )
        .unwrap_err();
        assert!(format!("{err}").contains("change id is required"));
    }

    #[derive(Default)]
    struct DeriveCase {
        desc: &'static str,
        expect_contains: &'static str,
    }

    #[test]
    fn derive_change_id_kebabizes_and_sanitizes() {
        let cases = [
            DeriveCase {
                desc: "Add user login",
                expect_contains: "add-user-login",
            },
            DeriveCase {
                desc: "fix validate hint",
                expect_contains: "fix-validate-hint",
            },
            DeriveCase {
                desc: "Refactor  the   config  loader!!",
                expect_contains: "refactor-the-config-loader",
            },
        ];
        for c in cases {
            let id = derive_change_id(c.desc).expect(c.desc);
            assert_eq!(id, c.expect_contains, "desc={}", c.desc);
            assert!(validate_sdd_id(&id, "change").is_ok());
        }
    }

    #[test]
    fn derive_change_id_drops_cjk_and_keeps_ascii_words() {
        // CJK chars are dropped; ASCII words survive as kebab tokens.
        let id = derive_change_id("加一个 user login 功能").unwrap();
        assert_eq!(id, "user-login");
    }

    #[test]
    fn derive_change_id_rejects_empty_and_pure_punct() {
        assert!(derive_change_id("").is_err());
        assert!(derive_change_id("   ").is_err());
        assert!(derive_change_id("！！！").is_err());
    }

    #[test]
    fn derive_change_id_enforces_length_cap_at_boundary() {
        let long = "a".repeat(DERIVED_ID_MAX_LEN + 30);
        let id = derive_change_id(&long).unwrap();
        assert!(
            id.len() <= DERIVED_ID_MAX_LEN,
            "id len {} > cap {}",
            id.len(),
            DERIVED_ID_MAX_LEN
        );
        // Pure-alphanumeric input (no `-` boundary) truncates to the cap.
        assert_eq!(id.len(), DERIVED_ID_MAX_LEN);
    }

    #[test]
    fn from_creates_draft_and_announces_derived_id() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);

        run(
            root,
            NewArgs {
                change: None,
                from: Some("Add user login".into()),
                force: false,
            },
        )
        .unwrap();

        let proposal = root.join("llmanspec/changes/add-user-login/proposal.md");
        assert!(proposal.exists());
        assert!(
            proposal
                .to_string_lossy()
                .ends_with("add-user-login/proposal.md")
        );
    }
}
