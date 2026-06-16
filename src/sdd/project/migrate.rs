//! One-shot, idempotent migration that brings an `llmanspec/` tree to the current
//! canonical spec format: standalone `.toon` files with `valid_scope` in-document.
//!
//! The runtime's own decode/encode is used so the migrated output is guaranteed to
//! round-trip (`dump_*` already re-decodes strictly). Re-running `migrate` on an
//! already-current tree is a no-op.
//!
//! Per spec directory it handles three states:
//! - `spec.md` (legacy: YAML frontmatter + a fenced ```` ```toon ```` block) -> fold
//!   `valid_scope`, write `spec.toon`, delete `spec.md`.
//! - `spec.toon` that strict-parses -> already current, skip.
//! - `spec.toon` carrying dropped fields (`valid_commands`/`evidence`) -> strip them
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

    let mode = if args.dry_run { "DRY RUN" } else { "migrating" };
    println!("{mode}: {} spec director(ies)", dirs.len());

    let mut migrated = 0usize;
    let mut already_current = 0usize;
    let mut errors = Vec::new();
    for dir in &dirs {
        match migrate_dir(dir, args.dry_run, args.force) {
            Ok(MigrateOutcome::Migrated) => migrated += 1,
            Ok(MigrateOutcome::AlreadyCurrent) => already_current += 1,
            Err(e) => errors.push(format!("{}: {e}", dir.display())),
        }
    }

    println!(
        "Migrated {migrated}, already-current {already_current}, errors {}.",
        errors.len()
    );
    if !errors.is_empty() {
        eprintln!("Errors ({}):", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
        return Err(anyhow!(
            "migration completed with {} error(s)",
            errors.len()
        ));
    }
    Ok(())
}

enum MigrateOutcome {
    Migrated,
    AlreadyCurrent,
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

fn migrate_dir(dir: &Path, dry_run: bool, force: bool) -> Result<MigrateOutcome> {
    let legacy = dir.join(LEGACY_SPEC_FILE);
    let current = dir.join(SPEC_FILE);

    if legacy.exists() {
        return migrate_legacy(&legacy, &current, dry_run, force);
    }
    if current.exists() {
        return normalize_toon(&current, dry_run);
    }
    // No spec file at all — nothing to do.
    Ok(MigrateOutcome::AlreadyCurrent)
}

/// `.md` + fence -> standalone `.toon`.
fn migrate_legacy(
    legacy: &Path,
    current: &Path,
    dry_run: bool,
    force: bool,
) -> Result<MigrateOutcome> {
    if current.exists() && !force {
        println!(
            "  skip ({} exists, use --force): {}",
            SPEC_FILE,
            display_rel(current)
        );
        return Ok(MigrateOutcome::AlreadyCurrent);
    }

    let content =
        fs::read_to_string(legacy).with_context(|| format!("read {}", legacy.display()))?;

    let (frontmatter_yaml, body) = split_frontmatter(&content);

    let payload = extract_fenced_toon(&body)
        .or_else(|_| {
            // Already a raw TOON document (no fence) — allow it.
            if body.trim_start().starts_with("kind:") {
                Ok(body.trim().to_string())
            } else {
                Err(anyhow!(
                    "no ```toon fenced block and not a raw TOON document"
                ))
            }
        })
        .with_context(|| format!("extract TOON from {}", legacy.display()))?;

    let serialized = if is_delta_kind(&payload) {
        let mut doc: DeltaSpecDoc = BACKEND
            .parse_delta_spec(&payload, "migrate")
            .context("parse delta")?;
        doc.kind = "llman.sdd.delta".to_string();
        BACKEND.dump_delta_spec(&doc).context("serialize delta")?
    } else {
        let mut doc: MainSpecDoc = BACKEND
            .parse_main_spec(&payload, "migrate")
            .context("parse main spec")?;
        // Only valid_scope is carried over from the legacy frontmatter.
        if let Some(yaml) = frontmatter_yaml.as_deref() {
            merge_frontmatter_into_doc(&mut doc, yaml);
        }
        doc.kind = "llman.sdd.spec".to_string();
        BACKEND
            .dump_main_spec(&doc)
            .context("serialize main spec")?
    };

    if dry_run {
        println!("  would write: {}", display_rel(current));
        return Ok(MigrateOutcome::Migrated);
    }

    atomic_write_with_mode(current, serialized.as_bytes(), None)?;
    fs::remove_file(legacy)?;
    println!("  {} -> {}", display_rel(legacy), display_rel(current));
    Ok(MigrateOutcome::Migrated)
}

/// `.toon` already exists: skip if current, normalize if it carries dropped fields.
fn normalize_toon(current: &Path, dry_run: bool) -> Result<MigrateOutcome> {
    let content =
        fs::read_to_string(current).with_context(|| format!("read {}", current.display()))?;

    // Already current?
    if BACKEND.parse_main_spec_strict(&content, "migrate").is_ok()
        || BACKEND.parse_delta_spec_strict(&content, "migrate").is_ok()
    {
        return Ok(MigrateOutcome::AlreadyCurrent);
    }

    // Stale: try stripping dropped top-level keys, then re-encode.
    let stripped = strip_dropped_keys(&content);
    if stripped == content {
        // Not a dropped-field problem — leave it; validate will report the real error.
        println!(
            "  skip (not a known migration case): {}",
            display_rel(current)
        );
        return Ok(MigrateOutcome::AlreadyCurrent);
    }

    let serialized = if is_delta_kind(&stripped) {
        let mut doc: DeltaSpecDoc = BACKEND
            .parse_delta_spec(&stripped, "migrate")
            .context("parse delta")?;
        doc.kind = "llman.sdd.delta".to_string();
        BACKEND.dump_delta_spec(&doc).context("serialize delta")?
    } else {
        let mut doc: MainSpecDoc = BACKEND
            .parse_main_spec(&stripped, "migrate")
            .context("parse main spec")?;
        doc.kind = "llman.sdd.spec".to_string();
        BACKEND
            .dump_main_spec(&doc)
            .context("serialize main spec")?
    };

    if dry_run {
        println!("  would normalize: {}", display_rel(current));
        return Ok(MigrateOutcome::Migrated);
    }

    atomic_write_with_mode(current, serialized.as_bytes(), None)?;
    println!("  normalized: {}", display_rel(current));
    Ok(MigrateOutcome::Migrated)
}

/// Remove top-level `valid_commands[...]` / `evidence[...]` lines (standalone inline
/// single-column arrays). These keys are never nested under indented tabular rows,
/// so a leading-anchored match is safe.
fn strip_dropped_keys(content: &str) -> String {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !DROPPED_KEYS
                .iter()
                .any(|k| trimmed.starts_with(k) && trimmed[k.len()..].trim_start().starts_with('['))
        })
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
        let MigrateOutcome::Migrated = migrate_dir(&dirs[0], false, false).unwrap() else {
            panic!("expected Migrated");
        };

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
        let MigrateOutcome::Migrated = migrate_dir(&dirs[0], false, false).unwrap() else {
            panic!("expected Migrated");
        };

        let out = dirs[0].join(SPEC_FILE);
        let content = fs::read_to_string(&out).unwrap();
        assert!(content.contains("llman.sdd.delta"));
        assert!(!content.contains("```toon"));
    }

    #[test]
    fn normalizes_stale_toon_with_dropped_fields() {
        let dir = tempdir().unwrap();
        let specs = dir.path().join("llmanspec/specs/foo");
        // A .toon that carries the dropped valid_commands/evidence keys.
        write(
            &specs.join(SPEC_FILE),
            "kind: llman.sdd.spec\nname: foo\npurpose: \"x\"\nvalid_scope[1]: src\nvalid_commands[1]: \"cargo test\"\nevidence[1]: ci\nrequirements[1]{req_id,title,statement}:\n  r1,A,System MUST do a.\nscenarios[1]{req_id,id,given,when,then}:\n  r1,b,\"\",t happens,it works\n",
        );

        let dirs = collect_spec_dirs(&dir.path().join("llmanspec/specs")).unwrap();
        let MigrateOutcome::Migrated = migrate_dir(&dirs[0], false, false).unwrap() else {
            panic!("expected Migrated (normalized)");
        };
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
        let outcome = migrate_dir(&dirs[0], false, false).unwrap();
        assert!(matches!(outcome, MigrateOutcome::AlreadyCurrent));
    }

    #[test]
    fn strip_only_drops_targeted_top_level_keys() {
        // A requirement statement that mentions "evidence[" must survive.
        let content = "kind: llman.sdd.spec\nname: foo\nvalid_scope[1]: src\nvalid_commands[1]: cargo test\nevidence[1]: ci\nrequirements[1]{req_id,title,statement}:\n  r1,A,System MUST reference evidence[0] in prose.\n";
        let stripped = strip_dropped_keys(content);
        assert!(!stripped.contains("valid_commands"));
        // The indented requirement line mentioning evidence[ is preserved.
        assert!(stripped.contains("evidence[0] in prose"));
    }
}
