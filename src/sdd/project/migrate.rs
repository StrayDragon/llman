//! One-shot, idempotent conversion: legacy `spec.md` (YAML frontmatter + fenced
//! ```` ```toon ```` block) → canonical `spec.toon`.
//!
//! The runtime's own decode/encode is used so the migrated output is guaranteed to
//! round-trip. Re-running on an already-current tree is a no-op.
//!
//! Per spec directory it handles three states:
//! - `spec.md` (legacy: YAML frontmatter + a fenced ```toon block) → fold
//!   `valid_scope`, write `spec.toon`, delete `spec.md`.
//! - `spec.toon` that strict-parses → already current, skip.
//! - `spec.toon` carrying dropped fields (`valid_commands`/`evidence`) → strip them
//!   and re-encode.

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::spec::backend::{BACKEND, SpecBackend};
use crate::sdd::spec::frontmatter::split_frontmatter;
use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
use anyhow::{Context, Result, anyhow};
use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};

const LEGACY_SPEC_FILE: &str = "spec.md";
/// Top-level TOON keys that were dropped from the spec format. Stale `.toon` files
/// carrying them are normalized by removing these (single-line inline-array) lines.
const DROPPED_KEYS: &[&str] = &["valid_commands", "evidence"];

#[derive(Debug, Clone)]
pub struct MigrateArgs {
    pub dry_run: bool,
    pub force: bool,
    /// Skip the confirmation prompt and apply (for agents/scripts).
    pub yes: bool,
    /// Treat the terminal as non-interactive even when stdin is a TTY.
    pub no_interactive: bool,
}

pub fn run(args: MigrateArgs) -> Result<()> {
    let root = Path::new(".");
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);

    // Every spec directory under specs/ and changes/**/specs/.
    let mut dirs = collect_spec_dirs(&llmanspec.join("specs"))?;
    let changes_dir = llmanspec.join("changes");
    if changes_dir.exists() {
        for entry in walk_dirs(&changes_dir)? {
            if entry
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "specs")
                .unwrap_or(false)
            {
                dirs.extend(collect_spec_dirs(&entry)?);
            }
        }
    }

    if dirs.is_empty() {
        println!("No spec directories found; nothing to migrate.");
        return Ok(());
    }

    // Phase 1: scan — build a plan per dir WITHOUT writing anything.
    let mut plans: Vec<(PathBuf, Plan)> = Vec::new();
    let mut errors = Vec::new();
    for dir in &dirs {
        match plan_dir(dir, args.force) {
            Ok(plan) => plans.push((dir.clone(), plan)),
            Err(e) => errors.push(format!("{}: {e}", dir.display())),
        }
    }

    print_scan_report(&plans, &errors, args.dry_run);
    if !errors.is_empty() {
        eprintln!("Errors ({}):", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
        return Err(anyhow!("scan completed with {} error(s)", errors.len()));
    }

    let to_apply: Vec<&(PathBuf, Plan)> = plans
        .iter()
        .filter(|(_, p)| matches!(p, Plan::Migrate { .. }))
        .collect();
    if to_apply.is_empty() {
        println!("Nothing to migrate; all specs are current.");
        return Ok(());
    }

    // Phase 2: confirm, then apply.
    if args.dry_run {
        println!("\n(dry-run: no files written)");
        return Ok(());
    }
    if !args.yes {
        let interactive = crate::sdd::shared::interactive::is_interactive(args.no_interactive);
        if !interactive {
            return Err(anyhow!(
                "non-interactive terminal: re-run with --yes to apply, or --dry-run to preview"
            ));
        }
        let confirmed = inquire::Confirm::new(&format!(
            "Migrate {} spec file(s) as shown above?",
            to_apply.len()
        ))
        .with_default(false)
        .prompt()
        .map_err(|e| anyhow!("confirmation prompt failed: {e}"))?;
        if !confirmed {
            println!("Aborted; no files changed.");
            return Ok(());
        }
    }

    let mut migrated = 0usize;
    let mut apply_errors = Vec::new();
    for (dir, plan) in &plans {
        if let Plan::Migrate { action, .. } = plan {
            match apply_plan(dir, action) {
                Ok(()) => {
                    migrated += 1;
                    println!("{}", action.summary_line(dir));
                }
                Err(e) => apply_errors.push(format!("{}: {e}", dir.display())),
            }
        }
    }

    println!("Migrated {migrated} file(s).");
    if !apply_errors.is_empty() {
        eprintln!("Errors ({}):", apply_errors.len());
        for e in &apply_errors {
            eprintln!("  - {e}");
        }
        return Err(anyhow!(
            "migration completed with {} error(s)",
            apply_errors.len()
        ));
    }
    Ok(())
}

/// What `migrate` plans to do with one spec directory.
enum Plan {
    /// Needs migration; `action` carries the computed output + side effects.
    Migrate { action: Action },
    /// Already current (or un-diagnosable) — left untouched.
    Skip { _reason: String },
}

/// A concrete migration to apply: the serialized target content and whether the
/// legacy `spec.md` must be removed.
struct Action {
    serialized: String,
    remove_legacy: bool,
}

impl Action {
    fn summary_line(&self, dir: &Path) -> String {
        let current = dir.join(SPEC_FILE);
        if self.remove_legacy {
            format!(
                "  {} -> {}",
                display_rel(&dir.join(LEGACY_SPEC_FILE)),
                display_rel(&current)
            )
        } else {
            format!("  normalized: {}", display_rel(&current))
        }
    }
}

/// Print a clear scan report: per-dir intentions + totals.
fn print_scan_report(plans: &[(PathBuf, Plan)], errors: &[String], dry_run: bool) {
    let heading = if dry_run { "DRY RUN scan" } else { "Scan" };
    println!(
        "{heading}: {} spec director(ies)",
        plans.len() + errors.len()
    );

    let mut legacy = 0usize;
    let mut normalized = 0usize;
    let mut skipped = 0usize;
    for (dir, plan) in plans {
        match plan {
            Plan::Migrate { action } => {
                if action.remove_legacy {
                    legacy += 1;
                    println!(
                        "  [migrate] {} -> {}",
                        display_rel(&dir.join(LEGACY_SPEC_FILE)),
                        display_rel(&dir.join(SPEC_FILE))
                    );
                } else {
                    normalized += 1;
                    println!("  [normalize] {}", display_rel(&dir.join(SPEC_FILE)));
                }
            }
            Plan::Skip { .. } => {
                skipped += 1;
            }
        }
    }
    println!(
        "Summary: {legacy} legacy .md to migrate, {normalized} stale .toon to normalize, {skipped} already-current, {} error(s).",
        errors.len()
    );
}

/// Collect immediate child directories of `specs_root` (one per capability/change).
fn collect_spec_dirs(specs_root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !specs_root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(specs_root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            out.push(entry.path());
        }
    }
    Ok(out)
}

/// Recursively yield all subdirectories under `root`.
fn walk_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let path = entry.path();
            out.push(path.clone());
            out.extend(walk_dirs(&path)?);
        }
    }
    Ok(out)
}

/// Scan one spec directory and decide what to do, WITHOUT writing. The same
/// single pipeline computes the canonical output regardless of source: extract the
/// TOON payload (fence-aware), strip dropped keys, fold `valid_scope` from any
/// legacy frontmatter, re-encode. One run lands at the latest format; re-running
/// is a no-op (current files plan as Skip).
fn plan_dir(dir: &Path, force: bool) -> Result<Plan> {
    let legacy = dir.join(LEGACY_SPEC_FILE);
    let current = dir.join(SPEC_FILE);
    let both = legacy.exists() && current.exists();

    let (source, is_legacy) = if current.exists() {
        (current.clone(), false)
    } else if legacy.exists() {
        (legacy.clone(), true)
    } else {
        // No spec file at all — nothing to do.
        return Ok(Plan::Skip {
            _reason: "no spec file".into(),
        });
    };

    if both && !force {
        return Ok(Plan::Skip {
            _reason: format!("both {SPEC_FILE} and {LEGACY_SPEC_FILE} exist (pass --force)"),
        });
    }

    let content =
        fs::read_to_string(&source).with_context(|| format!("read {}", source.display()))?;

    // Only plan work when there is something to migrate: a legacy .md, or a .toon
    // still carrying dropped fields. A current .toon (or one we cannot diagnose) is
    // left untouched — `validate` reports any real error.
    if !is_legacy && !has_dropped_keys(&content) {
        return Ok(Plan::Skip {
            _reason: "already current".into(),
        });
    }

    let serialized = reencode(&content, is_legacy)?;
    Ok(Plan::Migrate {
        action: Action {
            serialized,
            remove_legacy: is_legacy || (both && force),
        },
    })
}

/// Compute the canonical serialized output for a spec's content. Strips dropped
/// keys, folds `valid_scope` from any legacy frontmatter, and re-encodes.
fn reencode(content: &str, is_legacy: bool) -> Result<String> {
    let (frontmatter_yaml, body) = split_frontmatter(content);
    let payload = extract_fenced_toon(&body).or_else(|_| {
        // Already a raw TOON document (no fence) — use it as-is.
        if body.trim_start().starts_with("kind:") {
            Ok(body.trim().to_string())
        } else {
            Err(anyhow!(
                "no ```toon fenced block and not a raw TOON document"
            ))
        }
    })?;
    // Always strip dropped keys: no-op for clean docs, fixes stale or hybrid inputs.
    let payload = strip_dropped_keys(&payload);

    if is_delta_kind(&payload) {
        let mut doc: DeltaSpecDoc = BACKEND
            .parse_delta_spec(&payload, "migrate")
            .context("parse delta")?;
        doc.kind = "llman.sdd.delta".to_string();
        BACKEND.dump_delta_spec(&doc).context("serialize delta")
    } else {
        let mut doc: MainSpecDoc = BACKEND
            .parse_main_spec(&payload, "migrate")
            .context("parse main spec")?;
        // Only valid_scope is carried over from the legacy frontmatter.
        if is_legacy && let Some(yaml) = frontmatter_yaml.as_deref() {
            merge_frontmatter_into_doc(&mut doc, yaml);
        }
        doc.kind = "llman.sdd.spec".to_string();
        BACKEND.dump_main_spec(&doc).context("serialize main spec")
    }
}

/// Apply a planned migration: write `spec.toon` and remove a superseded `spec.md`.
fn apply_plan(dir: &Path, action: &Action) -> Result<()> {
    let current = dir.join(SPEC_FILE);
    atomic_write_with_mode(&current, action.serialized.as_bytes(), None)?;
    if action.remove_legacy {
        fs::remove_file(dir.join(LEGACY_SPEC_FILE))?;
    }
    Ok(())
}

/// Whether the content carries any dropped top-level keys. Checked directly (not
/// via strip-and-compare) so trailing-newline differences don't cause false
/// positives that would break idempotency.
fn has_dropped_keys(content: &str) -> bool {
    content.lines().any(is_dropped_key_line)
}

fn is_dropped_key_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    DROPPED_KEYS
        .iter()
        .any(|k| trimmed.starts_with(k) && trimmed[k.len()..].trim_start().starts_with('['))
}

/// Remove top-level `valid_commands[...]` / `evidence[...]` lines (standalone inline
/// single-column arrays). These keys are never nested under indented tabular rows,
/// so a leading-anchored match is safe.
fn strip_dropped_keys(content: &str) -> String {
    content
        .lines()
        .filter(|line| !is_dropped_key_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Pull the ```` ```toon ... ``` ```` fenced block's payload out of a markdown body.
///
/// Legacy spec files contain exactly one fenced block at the top level; any other
/// ```` ``` ```` sequences in the body live *inside* quoted TOON string values.
/// We treat the **opening** ```` ```toon ```` line and the **final** ```` ``` ````
/// line as the block boundaries.
fn extract_fenced_toon(body: &str) -> Result<String> {
    let lines: Vec<&str> = body.lines().collect();
    let open = lines.iter().position(|line| {
        let t = line.trim();
        t.starts_with("```")
            && t.trim_start_matches('`')
                .trim()
                .eq_ignore_ascii_case("toon")
    });
    let open = open.ok_or_else(|| anyhow!("no ```toon fenced block"))?;

    let close = lines
        .iter()
        .rposition(|line| line.trim().starts_with("```") && line.trim() != "```toon");
    let close = close.ok_or_else(|| anyhow!("unterminated ```toon fenced block"))?;

    if close <= open {
        return Err(anyhow!("malformed ```toon fenced block"));
    }
    Ok(lines[open + 1..close].join("\n"))
}

/// Carry `valid_scope` over from the legacy frontmatter (other keys are ignored).
fn merge_frontmatter_into_doc(doc: &mut MainSpecDoc, yaml: &str) {
    let parsed: Value = match serde_yaml::from_str(yaml) {
        Ok(v) => v,
        Err(_) => return,
    };
    if doc.valid_scope.is_empty() {
        doc.valid_scope = frontmatter_list(&parsed, "llman_spec_valid_scope");
    }
}

fn frontmatter_list(doc: &Value, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(value) = doc.get(key) {
        match value {
            Value::String(s) => out.push(s.trim().to_string()),
            Value::Sequence(items) => {
                for item in items {
                    if let Value::String(s) = item {
                        let t = s.trim();
                        if !t.is_empty() {
                            out.push(t.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn display_rel(path: &Path) -> String {
    let s = path.display().to_string();
    if let Some(idx) = s.find("llmanspec") {
        s[idx..].to_string()
    } else {
        s
    }
}

/// Detect a delta spec from its `kind:` declaration (the first key) — spec text
/// values may legitimately mention `llman.sdd.delta`.
fn is_delta_kind(payload: &str) -> bool {
    for line in payload.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(value) = t.strip_prefix("kind:") {
            return value.trim() == "llman.sdd.delta";
        }
        return false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn migrates_main_spec_folding_frontmatter() {
        let dir = tempdir().unwrap();
        let specs = dir.path().join("llmanspec/specs/foo");
        let legacy = specs.join(LEGACY_SPEC_FILE);
        write(
            &legacy,
            "---\nllman_spec_valid_scope:\n  - src/\nllman_spec_valid_commands:\n  - cargo test\nllman_spec_evidence:\n  - \"CI #1\"\n---\n\n```toon\nkind: llman.sdd.spec\nname: foo\npurpose: \"x\"\nrequirements[1]{req_id,title,statement}:\n  r1,A,System MUST do a.\nscenarios[1]{req_id,id,given,when,then}:\n  r1,b,\"\",t happens,it works\n```\n",
        );

        let dirs = collect_spec_dirs(&dir.path().join("llmanspec/specs")).unwrap();
        assert_eq!(dirs.len(), 1);
        let Plan::Migrate { action } = plan_dir(&dirs[0], false).unwrap() else {
            panic!("expected Plan::Migrate");
        };
        assert!(action.remove_legacy, "legacy .md should be removed");
        apply_plan(&dirs[0], &action).unwrap();

        let out = dirs[0].join(SPEC_FILE);
        assert!(out.exists(), "spec.toon should be written");
        assert!(!legacy.exists(), "spec.md should be removed");
        let content = fs::read_to_string(&out).unwrap();
        assert!(content.contains("valid_scope"));
        assert!(!content.contains("valid_commands"));
        assert!(!content.contains("evidence"));
        assert!(!content.contains("```toon"));
        assert!(content.contains("System MUST do a."));
    }

    #[test]
    fn migrates_delta_spec_dropping_fence() {
        let dir = tempdir().unwrap();
        let specs = dir.path().join("llmanspec/changes/add/specs/foo");
        let legacy = specs.join(LEGACY_SPEC_FILE);
        write(
            &legacy,
            "```toon\nkind: llman.sdd.delta\nops[1]{op,req_id,title,statement,from,to,name}:\n  add_requirement,r1,A,System MUST do a.,null,null,null\nop_scenarios[1]{req_id,id,given,when,then}:\n  r1,b,\"\",t happens,it works\n```\n",
        );

        let dirs =
            collect_spec_dirs(dir.path().join("llmanspec/changes/add/specs").as_path()).unwrap();
        assert_eq!(dirs.len(), 1);
        let Plan::Migrate { action } = plan_dir(&dirs[0], false).unwrap() else {
            panic!("expected Plan::Migrate");
        };
        apply_plan(&dirs[0], &action).unwrap();

        let out = dirs[0].join(SPEC_FILE);
        let content = fs::read_to_string(&out).unwrap();
        assert!(content.contains("llman.sdd.delta"));
        assert!(!content.contains("```toon"));
    }

    #[test]
    fn normalizes_stale_toon_with_dropped_fields() {
        let dir = tempdir().unwrap();
        let specs = dir.path().join("llmanspec/specs/foo");
        write(
            &specs.join(SPEC_FILE),
            "kind: llman.sdd.spec\nname: foo\npurpose: \"x\"\nvalid_scope[1]: src\nvalid_commands[1]: \"cargo test\"\nevidence[1]: ci\nrequirements[1]{req_id,title,statement}:\n  r1,A,System MUST do a.\nscenarios[1]{req_id,id,given,when,then}:\n  r1,b,\"\",t happens,it works\n",
        );

        let dirs = collect_spec_dirs(&dir.path().join("llmanspec/specs")).unwrap();
        let Plan::Migrate { action } = plan_dir(&dirs[0], false).unwrap() else {
            panic!("expected Plan::Migrate (normalize)");
        };
        assert!(!action.remove_legacy, "no legacy .md to remove");
        apply_plan(&dirs[0], &action).unwrap();
        let content = fs::read_to_string(dirs[0].join(SPEC_FILE)).unwrap();
        assert!(content.contains("valid_scope"));
        assert!(!content.contains("valid_commands"));
        assert!(!content.contains("evidence"));
    }

    #[test]
    fn skips_already_current_toon() {
        let dir = tempdir().unwrap();
        let specs = dir.path().join("llmanspec/specs/foo");
        write(
            &specs.join(SPEC_FILE),
            "kind: llman.sdd.spec\nname: foo\npurpose: \"x\"\nvalid_scope[1]: src\nrequirements[1]{req_id,title,statement}:\n  r1,A,System MUST do a.\nscenarios[1]{req_id,id,given,when,then}:\n  r1,b,\"\",t happens,it works\n",
        );
        let dirs = collect_spec_dirs(&dir.path().join("llmanspec/specs")).unwrap();
        let plan = plan_dir(&dirs[0], false).unwrap();
        assert!(matches!(plan, Plan::Skip { .. }));
    }

    #[test]
    fn strip_only_drops_targeted_top_level_keys() {
        let content = "kind: llman.sdd.spec\nname: foo\nvalid_scope[1]: src\nvalid_commands[1]: cargo test\nevidence[1]: ci\nrequirements[1]{req_id,title,statement}:\n  r1,A,System MUST reference evidence[0] in prose.\n";
        let stripped = strip_dropped_keys(content);
        assert!(!stripped.contains("valid_commands"));
        assert!(stripped.contains("evidence[0] in prose"));
    }
}
