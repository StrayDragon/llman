//! Create a draft change directory with a minimal `proposal.md` skeleton.
//!
//! Does **not** create delta specs (`llman sdd change delta skeleton`) or other
//! planning artifacts — those are added by propose/authoring helpers.
//!
//! `--from <description>` (r99): derive a legal, meaningful change id from the
//! description instead of requiring `<CHANGE>`. When the project configures
//! `change_id.template` (r29), the id is rendered from that template with
//! preset vars — output then follows the project's declared convention.
//! Otherwise the id is built from the description's semantics (kebab-case,
//! sanitized). Exactly one of `<CHANGE>` or `--from` is required.

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::{self, SddConfig};
use crate::sdd::shared::change_id::harvest_unique_numbers;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::{change_dir, proposal_path};
use crate::sdd::shared::ids::validate_sdd_id;
use anyhow::{Context, Result, anyhow, bail};
use minijinja::{Environment, UndefinedBehavior};
use std::fs;
use std::path::Path;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub(crate) struct NewArgs {
    /// Explicit change id. `None` when the caller passes `--from` instead.
    pub(crate) change: Option<String>,
    /// Free-form description to derive the change id from (r99 lightweight path).
    pub(crate) from: Option<String>,
    pub(crate) force: bool,
    /// Print the id that would be produced without creating anything (r29).
    pub(crate) dry_run: bool,
    /// Explicit verb for `change_id.template` rendering (r29); overrides the
    /// auto-detected verb prefix.
    pub(crate) verb: Option<String>,
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

pub(crate) fn run(root: &Path, args: NewArgs) -> Result<()> {
    match (args.change.as_deref(), args.from.as_deref()) {
        (Some(_), Some(_)) => {
            bail!("<CHANGE> and --from are mutually exclusive; pass one or the other");
        }
        (None, None) => {
            bail!("change id is required: pass <CHANGE> or --from <DESCRIPTION>");
        }
        (Some(id), None) => {
            if args.dry_run {
                println!("{id}");
                return Ok(());
            }
            create_draft(root, id, false, args.force)
        }
        (None, Some(desc)) => {
            let id = derive_id_for_run(root, desc, args.verb.as_deref())?;
            if args.dry_run {
                // r29: preview only — no directory, no file, no force check.
                println!("{id}");
                return Ok(());
            }
            create_draft(root, &id, true, args.force)
        }
    }
}

/// Resolve the id for a `--from` run: template rendering when the project
/// configures `change_id.template`, heuristic sanitization otherwise.
fn derive_id_for_run(root: &Path, desc: &str, verb_override: Option<&str>) -> Result<String> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = config::load_required_config(&llmanspec_dir)?;
    if config
        .change_id
        .as_ref()
        .and_then(|change_id| change_id.template.as_deref())
        .is_some()
    {
        render_template_id(&llmanspec_dir, &config, desc, verb_override)
    } else {
        // Legacy heuristic path (r99 pre-r29 behavior, unchanged).
        derive_change_id(desc)
    }
}

/// Render the change id from the configured `change_id.template` (r29).
fn render_template_id(
    llmanspec_dir: &Path,
    config: &SddConfig,
    desc: &str,
    verb_override: Option<&str>,
) -> Result<String> {
    let change_id = config
        .change_id
        .as_ref()
        .expect("caller checked change_id.template");
    let template = change_id
        .template
        .as_deref()
        .expect("caller checked template");
    let pattern_re =
        crate::sdd::shared::change_id::unique_group_regex(change_id.pattern.as_deref())?;
    let harvest = harvest_unique_numbers(llmanspec_dir, pattern_re.as_ref())?;
    for warning in &harvest.warnings {
        eprintln!("warning: {warning}");
    }

    let derived = derive_change_id(desc)?;
    let (verb, subject) = split_verb(&derived, verb_override);

    let date = OffsetDateTime::now_utc().date().to_string();
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.add_template("change_id", template)
        .context("invalid change_id.template")?;
    let template_handle = env.get_template("change_id")?;

    // Strict rendering names no variable, so pre-check undeclared refs and
    // report exactly which unprovided vars the template uses (r29 acceptance:
    // clear error for illegal template vars).
    let mut provided: std::collections::BTreeSet<&str> = ["llman_sdd_unique_id", "subject", "date"]
        .into_iter()
        .collect();
    if verb.is_some() {
        provided.insert("verb");
    }
    let missing: Vec<String> = template_handle
        .undeclared_variables(false)
        .into_iter()
        .filter(|var| !provided.contains(var.as_str()))
        .map(|var| var.to_string())
        .collect();
    if !missing.is_empty() {
        bail!(
            "change_id.template references unprovided variable(s): {} — preset vars are \
             llman_sdd_unique_id, verb, subject, date (pass --verb when it uses {{{{ verb }}}})",
            missing.join(", ")
        );
    }

    env.add_global("llman_sdd_unique_id", harvest.next_number.to_string());
    if let Some(verb) = verb {
        env.add_global("verb", verb);
    }
    env.add_global("subject", subject);
    env.add_global("date", date);

    let rendered = env
        .get_template("change_id")
        .context("invalid change_id.template")?
        .render(())
        .map_err(|err| anyhow!("failed to render change_id.template: {err}"))?;
    let id = rendered.trim().to_string();
    if id.is_empty() {
        bail!("change_id.template rendered an empty id");
    }
    validate_sdd_id(&id, "change")?;
    Ok(id)
}

/// Split a sanitized id into (verb, subject). The subject is the derived id
/// minus a detected table-verb prefix (falls back to the full derived id when
/// no verb prefix is present). `--verb` overrides the verb only — it never
/// changes the subject.
fn split_verb(derived: &str, verb_override: Option<&str>) -> (Option<String>, String) {
    const VERB_TABLE: [&str; 5] = ["add", "update", "remove", "refactor", "fix"];
    let mut subject = derived;
    let mut detected: Option<String> = None;
    for verb in VERB_TABLE {
        if let Some(rest) = derived.strip_prefix(&format!("{verb}-")) {
            detected = Some(verb.to_string());
            subject = rest;
            break;
        }
    }
    let verb = verb_override.map(str::to_string).or(detected);
    (verb, subject.to_string())
}

/// Create the draft change directory + proposal skeleton. When `derived` is
/// true, stdout additionally announces the derived id so agents/users can see
/// what was generated.
fn create_draft(root: &Path, id: &str, derived: bool, force: bool) -> Result<()> {
    validate_sdd_id(id, "change")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let _config = config::load_required_config(&llmanspec_dir)?;

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
/// the id readable and filesystem-safe. Projects that want a convention
/// machine-enforced configure `change_id.pattern` / `change_id.template`
/// (sdd-workflow r29); this function then supplies the `subject` fragment.
pub(crate) fn derive_change_id(desc: &str) -> Result<String> {
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
                dry_run: false,
                verb: None,
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
            dry_run: false,
            verb: None,
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
                dry_run: false,
                verb: None,
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
                dry_run: false,
                verb: None,
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
                dry_run: false,
                verb: None,
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
                dry_run: false,
                verb: None,
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
                dry_run: false,
                verb: None,
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

    fn write_change_id_config(root: &Path, pattern: &str, template: &str) {
        let path = root.join("llmanspec/config.yaml");
        let content = fs::read_to_string(&path).unwrap();
        let addition = format!("change_id:\n  pattern: '{pattern}'\n  template: '{template}'\n");
        fs::write(&path, content + &addition).unwrap();
    }

    #[test]
    fn template_renders_unique_id_verb_subject() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);
        write_change_id_config(
            root,
            r"^c[0-9]+-(add|update|remove|refactor|fix)-[a-z0-9-]+$",
            "c{{ llman_sdd_unique_id }}-{{ verb }}-{{ subject }}",
        );
        // Issue #20 shape: a deep dir with a higher number must be picked up.
        fs::create_dir_all(root.join("llmanspec/delayed-changes/tools/c2620-tool-x")).unwrap();

        run(
            root,
            NewArgs {
                change: None,
                from: Some("add user login".into()),
                force: false,
                dry_run: false,
                verb: None,
            },
        )
        .unwrap();
        let created: Vec<String> = fs::read_dir(root.join("llmanspec/changes"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            root.join("llmanspec/changes/c2621-add-user-login/proposal.md")
                .exists(),
            "unique id must scan the whole tree (next = 2621); verb must not duplicate; created={created:?}"
        );
    }

    #[test]
    fn dry_run_prints_id_and_creates_nothing() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);
        write_change_id_config(
            root,
            r"^c[0-9]+-.*$",
            "c{{ llman_sdd_unique_id }}-{{ verb }}-{{ subject }}",
        );
        run(
            root,
            NewArgs {
                change: None,
                from: Some("Add user login".into()),
                force: false,
                dry_run: true,
                verb: None,
            },
        )
        .unwrap();
        assert!(
            !root
                .join("llmanspec/changes")
                .join("c1-add-user-login")
                .exists(),
            "dry-run must not create the change dir"
        );
    }

    #[test]
    fn explicit_verb_overrides_detection_and_dedupes_subject() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);
        write_change_id_config(
            root,
            r"^c[0-9]+-.*$",
            "c{{ llman_sdd_unique_id }}-{{ verb }}-{{ subject }}",
        );
        run(
            root,
            NewArgs {
                change: None,
                from: Some("add user login".into()),
                force: false,
                dry_run: false,
                verb: Some("update".into()),
            },
        )
        .unwrap();
        assert!(root.join("llmanspec/changes/c1-update-user-login").exists());
    }

    #[test]
    fn missing_verb_yields_actionable_error() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);
        write_change_id_config(
            root,
            r"^c[0-9]+-.*$",
            "c{{ llman_sdd_unique_id }}-{{ verb }}-{{ subject }}",
        );
        let err = run(
            root,
            NewArgs {
                change: None,
                from: Some("login flow".into()), // no table-verb prefix → no verb
                force: false,
                dry_run: true,
                verb: None,
            },
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("--verb"), "got: {msg}");
    }

    #[test]
    fn unknown_template_var_yields_clear_error() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init_project(root);
        write_change_id_config(root, r"^c[0-9]+-.*$", "c{{ nope_var }}");
        let err = run(
            root,
            NewArgs {
                change: None,
                from: Some("add x".into()),
                force: false,
                dry_run: true,
                verb: None,
            },
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("nope_var") && msg.contains("preset vars"),
            "got: {msg}"
        );
    }
}
