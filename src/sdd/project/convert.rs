//! Migrate legacy spec files (`.md` with YAML frontmatter + a fenced ```` ```toon ````
//! block) to standalone `.toon` documents.
//!
//! The runtime's own decode/encode is used so the migrated output is guaranteed to
//! round-trip (`dump_*` already re-decodes strictly). Folding the YAML frontmatter
//! into the TOON document keeps each spec a single self-describing file.

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

#[derive(Debug, Clone)]
pub struct ConvertArgs {
    pub dry_run: bool,
    pub force: bool,
}

pub fn run(args: ConvertArgs) -> Result<()> {
    let root = Path::new(".");
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);

    let main_targets = collect_legacy_specs(&llmanspec.join("specs"))?;
    let mut change_targets = Vec::new();
    let changes_dir = llmanspec.join("changes");
    if changes_dir.exists() {
        // changes/<id>/specs/* and changes/archive/<date>-<id>/specs/*
        for entry in walk_dirs(&changes_dir)? {
            if entry
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "specs")
                .unwrap_or(false)
            {
                change_targets.extend(collect_legacy_specs(&entry)?);
            }
        }
    }

    let total = main_targets.len() + change_targets.len();
    if total == 0 {
        println!("No legacy `spec.md` files found; nothing to convert.");
        return Ok(());
    }

    let mode = if args.dry_run {
        "DRY RUN"
    } else {
        "converting"
    };
    println!("{mode}: {total} spec file(s)");

    let mut migrated = 0usize;
    let mut errors = Vec::new();
    for target in main_targets.iter().chain(change_targets.iter()) {
        match migrate_one(target, args.dry_run, args.force) {
            Ok(true) => migrated += 1,
            Ok(false) => {} // skipped
            Err(e) => errors.push(format!("{}: {e}", target.legacy.display())),
        }
    }

    println!("Converted {migrated} file(s).");
    if !errors.is_empty() {
        eprintln!("Errors ({}):", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
        return Err(anyhow!(
            "conversion completed with {} error(s)",
            errors.len()
        ));
    }
    Ok(())
}

struct LegacySpec {
    legacy: PathBuf,
    /// Parent directory (the `<capability>/` dir); the new file is written next to it.
    dir: PathBuf,
}

/// Collect `<dir>/<capability>/spec.md` paths.
fn collect_legacy_specs(specs_root: &Path) -> Result<Vec<LegacySpec>> {
    let mut out = Vec::new();
    if !specs_root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(specs_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let legacy = entry.path().join(LEGACY_SPEC_FILE);
        if legacy.exists() {
            out.push(LegacySpec {
                legacy,
                dir: entry.path(),
            });
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

fn migrate_one(spec: &LegacySpec, dry_run: bool, force: bool) -> Result<bool> {
    let target = spec.dir.join(SPEC_FILE);
    if target.exists() && !force {
        println!("  skip (exists): {}", display_rel(&target));
        return Ok(false);
    }

    let content = fs::read_to_string(&spec.legacy)
        .with_context(|| format!("read {}", spec.legacy.display()))?;

    // Detect main vs delta by the `kind:` line in the fenced payload.
    let (frontmatter_yaml, body) = split_frontmatter(&content);

    // Body is either a bare fenced block (delta) or frontmatter + fence (main).
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
        .with_context(|| format!("extract TOON from {}", spec.legacy.display()))?;

    let serialized = if is_delta_kind(&payload) {
        let mut doc: DeltaSpecDoc = BACKEND
            .parse_delta_spec(&payload, "convert")
            .context("parse delta")?;
        doc.kind = "llman.sdd.delta".to_string();
        BACKEND.dump_delta_spec(&doc).context("serialize delta")?
    } else {
        let mut doc: MainSpecDoc = BACKEND
            .parse_main_spec(&payload, "convert")
            .context("parse main spec")?;
        // Fold the legacy YAML frontmatter into the document as validation meta.
        if let Some(yaml) = frontmatter_yaml.as_deref() {
            merge_frontmatter_into_doc(&mut doc, yaml);
        }
        doc.kind = "llman.sdd.spec".to_string();
        BACKEND
            .dump_main_spec(&doc)
            .context("serialize main spec")?
    };

    if dry_run {
        println!("  would write: {}", display_rel(&target));
        return Ok(true);
    }

    atomic_write_with_mode(&target, serialized.as_bytes(), None)?;
    fs::remove_file(&spec.legacy)?;
    println!(
        "  {} -> {}",
        display_rel(&spec.legacy),
        display_rel(&target)
    );
    Ok(true)
}

/// Pull the ```` ```toon ... ``` ```` fenced block's payload out of a markdown body.
///
/// Legacy spec files contain exactly one fenced block at the top level; any other
/// ```` ``` ```` sequences in the body live *inside* quoted TOON string values
/// (e.g. specs that document fence syntax itself). To stay robust against those,
/// we treat the **opening** ```` ```toon ```` line and the **final** ```` ``` ````
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

    // Closing fence = the last line that is exactly a fence terminator.
    let close = lines
        .iter()
        .rposition(|line| line.trim().starts_with("```") && line.trim() != "```toon");
    let close = close.ok_or_else(|| anyhow!("unterminated ```toon fenced block"))?;

    if close <= open {
        return Err(anyhow!("malformed ```toon fenced block"));
    }
    Ok(lines[open + 1..close].join("\n"))
}

/// Translate the legacy frontmatter keys into the document's validation meta fields,
/// Only `valid_scope` is carried over — `valid_commands` and `evidence` were
/// dropped (not functionally consumed); they are silently ignored on conversion.
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

/// Detect a delta spec from its `kind:` declaration (the first key), rather than
/// a naive substring search — spec text values may legitimately mention
/// `llman.sdd.delta`.
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

        let targets = collect_legacy_specs(&dir.path().join("llmanspec/specs")).unwrap();
        assert_eq!(targets.len(), 1);
        let migrated = migrate_one(&targets[0], false, false).unwrap();
        assert!(migrated);

        let out = targets[0].dir.join(SPEC_FILE);
        assert!(out.exists(), "spec.toon should be written");
        assert!(!legacy.exists(), "spec.md should be removed");
        let content = fs::read_to_string(&out).unwrap();
        // Only valid_scope is carried over; valid_commands/evidence are dropped.
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

        let targets =
            collect_legacy_specs(dir.path().join("llmanspec/changes/add/specs").as_path()).unwrap();
        assert_eq!(targets.len(), 1);
        let migrated = migrate_one(&targets[0], false, false).unwrap();
        assert!(migrated);

        let out = targets[0].dir.join(SPEC_FILE);
        let content = fs::read_to_string(&out).unwrap();
        assert!(content.contains("llman.sdd.delta"));
        assert!(!content.contains("```toon"));
    }

    #[test]
    fn skip_when_toon_exists_without_force() {
        let dir = tempdir().unwrap();
        let specs = dir.path().join("llmanspec/specs/foo");
        write(
            &specs.join(LEGACY_SPEC_FILE),
            "kind: llman.sdd.spec\nname: foo\n",
        );
        write(&specs.join(SPEC_FILE), "kind: llman.sdd.spec\nname: foo\n");

        let targets = collect_legacy_specs(&dir.path().join("llmanspec/specs")).unwrap();
        let migrated = migrate_one(&targets[0], false, false).unwrap();
        assert!(!migrated, "should skip when .toon exists");
    }
}
