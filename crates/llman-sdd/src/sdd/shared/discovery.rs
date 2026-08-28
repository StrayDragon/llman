use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::spec::validation::discover_features;
use anyhow::{Result, bail};
use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Default max depth relative to `llmanspec/changes/` (depth 1 = direct children).
pub const DEFAULT_MAX_SCAN_DEPTH: usize = 8;

thread_local! {
    static MAX_SCAN_DEPTH: Cell<usize> = const { Cell::new(DEFAULT_MAX_SCAN_DEPTH) };
}

/// Effective scan depth for this thread (CLI may override via [`with_max_scan_depth`]).
pub(crate) fn effective_max_scan_depth() -> usize {
    MAX_SCAN_DEPTH.with(|c| c.get())
}

/// Run `f` with a temporary max scan depth, restoring the previous value afterwards.
pub(crate) fn with_max_scan_depth<F, R>(depth: usize, f: F) -> R
where
    F: FnOnce() -> R,
{
    MAX_SCAN_DEPTH.with(|c| {
        let prev = c.replace(depth);
        let out = f();
        c.set(prev);
        out
    })
}

/// One active change located under `llmanspec/changes/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChangeLoc {
    /// Leaf directory name (change id).
    pub(crate) id: String,
    /// Path relative to `llmanspec/changes/` (e.g. `c0` or `some_a/c0`).
    pub(crate) path: String,
}

impl ChangeLoc {
    pub(crate) fn abs_dir(&self, root: &Path) -> PathBuf {
        root.join(LLMANSPEC_DIR_NAME)
            .join("changes")
            .join(Path::new(&self.path))
    }
}

/// Recursively discover active changes (dirs with `proposal.md`).
///
/// Skips `archive/`, dot-directories, and symlinks. Does not recurse into a
/// directory once it is recognized as a change. Duplicate leaf ids → `Err`
/// listing conflicting relative paths.
pub(crate) fn discover_changes(root: &Path) -> Result<Vec<ChangeLoc>> {
    discover_changes_with_depth(root, effective_max_scan_depth())
}

pub(crate) fn discover_changes_with_depth(root: &Path, max_depth: usize) -> Result<Vec<ChangeLoc>> {
    if max_depth < 1 {
        bail!("max-scan-depth must be >= 1 (got {max_depth})");
    }
    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");
    if !changes_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut by_id: HashMap<String, Vec<String>> = HashMap::new();
    walk_changes(&changes_dir, "", 1, max_depth, &mut by_id)?;

    let mut conflicts: Vec<(String, Vec<String>)> = by_id
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(id, paths)| {
            let mut p = paths.clone();
            p.sort();
            (id.clone(), p)
        })
        .collect();
    if !conflicts.is_empty() {
        conflicts.sort_by(|a, b| a.0.cmp(&b.0));
        let detail = conflicts
            .iter()
            .map(|(id, paths)| {
                format!(
                    "  - {id}:\n{}",
                    paths
                        .iter()
                        .map(|p| format!("      {p}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "duplicate change id(s) under llmanspec/changes/ (leaf directory names must be unique):\n{detail}"
        );
    }

    let mut result: Vec<ChangeLoc> = by_id
        .into_iter()
        .map(|(id, mut paths)| {
            let path = paths.pop().expect("single path");
            ChangeLoc { id, path }
        })
        .collect();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}

fn walk_changes(
    dir: &Path,
    rel_prefix: &str,
    depth: usize,
    max_depth: usize,
    by_id: &mut HashMap<String, Vec<String>>,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        // Skip symlinks (do not follow).
        if file_type.is_symlink() {
            continue;
        }
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        // Top-level archive/ only (and never treat it as a group to recurse).
        if rel_prefix.is_empty() && name == "archive" {
            continue;
        }

        let rel = if rel_prefix.is_empty() {
            name.clone()
        } else {
            format!("{rel_prefix}/{name}")
        };
        let path = entry.path();
        if path.join("proposal.md").is_file() {
            by_id.entry(name).or_default().push(rel);
            // Do not recurse into a change directory.
            continue;
        }
        if depth < max_depth {
            walk_changes(&path, &rel, depth + 1, max_depth, by_id)?;
        }
    }
    Ok(())
}

/// Sorted leaf ids of active changes (uses effective max scan depth).
pub(crate) fn list_changes(root: &Path) -> Result<Vec<String>> {
    Ok(discover_changes(root)?.into_iter().map(|c| c.id).collect())
}

/// Absolute directory for an active change id (via discovery map).
pub(crate) fn resolve_change_dir(root: &Path, change_id: &str) -> Result<PathBuf> {
    let loc = resolve_change_loc(root, change_id)?;
    Ok(loc.abs_dir(root))
}

/// Relative path under `llmanspec/changes/` for an active change id.
pub(crate) fn resolve_change_rel_path(root: &Path, change_id: &str) -> Result<String> {
    Ok(resolve_change_loc(root, change_id)?.path)
}

/// Canonical directory for a change id under `llmanspec/changes/`.
/// Pure path builder — the change does not need to exist yet (`change new`);
/// to resolve an *existing* (possibly grouped) change use [`resolve_change_dir`].
pub(crate) fn change_dir(root: &Path, change_id: &str) -> PathBuf {
    root.join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join(change_id)
}

/// Canonical `proposal.md` path for a change id (pure path builder).
pub(crate) fn proposal_path(root: &Path, change_id: &str) -> PathBuf {
    change_dir(root, change_id).join("proposal.md")
}

pub(crate) fn resolve_change_loc(root: &Path, change_id: &str) -> Result<ChangeLoc> {
    let found = discover_changes(root)?;
    if let Some(loc) = found.into_iter().find(|c| c.id == change_id) {
        return Ok(loc);
    }
    bail!("change '{change_id}' not found under llmanspec/changes/");
}

pub(crate) fn extract_archived_change_id(dir_name: &str) -> Option<String> {
    if dir_name.len() <= 11 {
        return None;
    }
    let prefix = &dir_name[..10];
    let valid_date = prefix.chars().enumerate().all(|(i, c)| {
        (matches!(i, 0..=3 | 5..=6 | 8..=9) && c.is_ascii_digit())
            || (matches!(i, 4 | 7) && c == '-')
    });
    if !valid_date || dir_name.as_bytes().get(10) != Some(&b'-') {
        return None;
    }
    let change_id = &dir_name[11..];
    if change_id.is_empty() || change_id.starts_with('.') {
        return None;
    }
    Some(change_id.to_string())
}

pub(crate) fn list_archived_changes(root: &Path) -> Result<Vec<String>> {
    let archive_dir = root
        .join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join("archive");
    let mut result = Vec::new();
    let entries = match fs::read_dir(archive_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(result),
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(change_id) = extract_archived_change_id(&name) {
            result.push(change_id);
        }
    }

    result.sort();
    result.dedup();
    Ok(result)
}

pub(crate) fn list_specs(root: &Path) -> Result<Vec<String>> {
    let specs_dir = root.join(LLMANSPEC_DIR_NAME).join("specs");
    let mut result = Vec::new();
    let entries = match fs::read_dir(specs_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(result),
    };

    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let dir = entry.path();
        // Single-track (r131): a capability is a directory with exactly one
        // `.feature`. Legacy `spec.toon` still counts so the resolver can
        // point at `toon2features` instead of reporting "no such spec".
        if dir.join(SPEC_FILE).exists() || !discover_features(&dir).is_empty() {
            result.push(name);
        }
    }

    result.sort();
    Ok(result)
}

/// Outcome of resolving a user-provided change name to a canonical change id.
///
/// Carries the `via_prefix` flag (sourced from `prefix_resolve`, the single
/// source of truth for the "exact > prefix" rule) so callers can emit the
/// "'input' -> 'resolved' (prefix match)" hint mandated by cli spec r112
/// without re-deriving it (which would re-introduce the parallel-logic smell
/// that the r112 refactor removed).
#[derive(Debug, Clone)]
pub(crate) struct ResolvedChange {
    /// The canonical change id that the user's input resolved to.
    pub(crate) id: String,
    /// The (trimmed) user-provided input, kept for hint formatting.
    pub(crate) input: String,
    /// `false` for an exact match, `true` when `input` was a unique prefix of `id`.
    pub(crate) via_prefix: bool,
}

impl ResolvedChange {
    /// Returns the "'input' -> 'resolved' (prefix match)" hint when this was a
    /// prefix match, or `None` for an exact match (no hint is emitted then).
    pub(crate) fn prefix_hint(&self) -> Option<String> {
        if self.via_prefix {
            Some(format!(
                "{}\n",
                t!(
                    "sdd.prefix_match_hint",
                    input = self.input,
                    resolved = self.id
                )
            ))
        } else {
            None
        }
    }
}

/// Resolve a change name and emit the prefix-match hint to stderr (human output)
/// when the input was a prefix rather than an exact match. Returns the canonical
/// change id.
///
/// Convenience wrapper for commands that only produce human-readable output and
/// do not need the `via_prefix` flag for a JSON field. Commands that emit JSON
/// (`show`/`validate`/`status`) resolve directly so they can populate
/// `matchedViaPrefix`.
pub(crate) fn resolve_change_id_human(root: &Path, input: &str) -> Result<String> {
    let resolved = resolve_change_id(root, input)?;
    if let Some(hint) = resolved.prefix_hint() {
        eprint!("{hint}");
    }
    Ok(resolved.id)
}

/// Resolve a user-provided change name input to a canonical change id.
///
/// Resolution priority:
/// 1. Exact match against active changes (leaf id)
/// 2. Prefix match against active changes (leaf id prefix)
/// 3. Prefix match against archived changes (change id portion starts with `input`)
///
/// Returns the resolved change id (plus the `via_prefix` flag for the r112 hint)
/// on success. Errors with a descriptive message on multi-match (lists all
/// candidates) or no-match. Multi-match / not-found hints include relative paths
/// when available.
pub(crate) fn resolve_change_id(root: &Path, input: &str) -> Result<ResolvedChange> {
    use crate::sdd::shared::match_utils::{PrefixOutcome, prefix_resolve};

    let input = input.trim();
    if input.is_empty() {
        bail!("change id must not be empty");
    }

    let locs = discover_changes(root)?;
    let active: Vec<String> = locs.iter().map(|c| c.id.clone()).collect();
    let path_by_id: HashMap<&str, &str> = locs
        .iter()
        .map(|c| (c.id.as_str(), c.path.as_str()))
        .collect();
    let archived = list_archived_changes(root)?;

    let fmt_active = |id: &str| -> String {
        match path_by_id.get(id) {
            Some(p) if *p != id => format!("{id} ({p})"),
            _ => id.to_string(),
        }
    };

    // 1) Exact / prefix match against active changes (active takes priority)
    match prefix_resolve(input, &active) {
        PrefixOutcome::Single { id, via_prefix } => {
            return Ok(ResolvedChange {
                id: id.to_string(),
                input: input.to_string(),
                via_prefix,
            });
        }
        PrefixOutcome::Multiple(matches) => {
            let candidates = matches
                .iter()
                .map(|s| format!("  - {}", fmt_active(s)))
                .collect::<Vec<_>>()
                .join("\n");
            bail!(
                "change '{input}' matches multiple active changes:\n{candidates}\nDid you mean one of these?"
            );
        }
        PrefixOutcome::None => {}
    }

    // 2) Prefix match against archived changes (only when active had no match)
    match prefix_resolve(input, &archived) {
        PrefixOutcome::Single { id, via_prefix } => {
            return Ok(ResolvedChange {
                id: id.to_string(),
                input: input.to_string(),
                via_prefix,
            });
        }
        PrefixOutcome::Multiple(matches) => {
            let candidates = matches
                .iter()
                .map(|s| format!("  - {s}"))
                .collect::<Vec<_>>()
                .join("\n");
            bail!(
                "change '{input}' matches multiple archived changes:\n{candidates}\nDid you mean one of these?"
            );
        }
        PrefixOutcome::None => {}
    }

    // 3) No match at all
    let mut suggestions = Vec::new();
    suggestions.extend(active.iter().map(|id| fmt_active(id)));
    suggestions.extend(archived);
    let nearby = crate::sdd::shared::match_utils::nearest_matches(input, &suggestions, 5);
    if nearby.is_empty() {
        bail!("change '{input}' not found.");
    }
    bail!(
        "change '{input}' not found. Did you mean: {}?",
        nearby.join(", ")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_proposal(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("proposal.md"), "## Why\nx\n").unwrap();
    }

    #[test]
    fn discovers_flat_and_nested() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_proposal(&root.join("llmanspec/changes/flat-a"));
        write_proposal(&root.join("llmanspec/changes/grp/nested-b"));
        // group dir without proposal is not a change
        fs::create_dir_all(root.join("llmanspec/changes/grp")).unwrap();

        let locs = discover_changes(root).unwrap();
        let ids: Vec<_> = locs.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["flat-a", "nested-b"]);
        assert_eq!(
            locs.iter().find(|c| c.id == "nested-b").unwrap().path,
            "grp/nested-b"
        );
        assert_eq!(
            resolve_change_rel_path(root, "nested-b").unwrap(),
            "grp/nested-b"
        );
        assert!(
            resolve_change_dir(root, "nested-b")
                .unwrap()
                .ends_with("llmanspec/changes/grp/nested-b")
        );
    }

    #[test]
    fn duplicate_leaf_ids_error_lists_paths() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_proposal(&root.join("llmanspec/changes/a/dup"));
        write_proposal(&root.join("llmanspec/changes/b/dup"));
        let err = discover_changes(root).unwrap_err().to_string();
        assert!(err.contains("duplicate change id"));
        assert!(err.contains("a/dup"));
        assert!(err.contains("b/dup"));
    }

    #[test]
    fn max_depth_hides_deeper_changes() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_proposal(&root.join("llmanspec/changes/deep/x/y/z"));
        // depth 1: only direct children — deep is a group without proposal
        let none = discover_changes_with_depth(root, 1).unwrap();
        assert!(none.is_empty());
        // depth 4: changes/deep/x/y/z → z at depth 4
        let found = discover_changes_with_depth(root, 4).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "z");
        assert_eq!(found[0].path, "deep/x/y/z");
    }

    #[test]
    fn skips_archive_and_does_not_recurse_into_change() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_proposal(&root.join("llmanspec/changes/active"));
        write_proposal(&root.join("llmanspec/changes/active/nested-should-ignore"));
        write_proposal(&root.join("llmanspec/changes/archive/2026-01-01-old"));
        let locs = discover_changes(root).unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].id, "active");
    }

    #[test]
    fn rejects_depth_zero() {
        let tmp = TempDir::new().unwrap();
        assert!(discover_changes_with_depth(tmp.path(), 0).is_err());
    }

    #[test]
    fn with_max_scan_depth_scopes_effective_depth() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_proposal(&root.join("llmanspec/changes/g/c1"));
        with_max_scan_depth(1, || {
            assert!(list_changes(root).unwrap().is_empty());
        });
        with_max_scan_depth(2, || {
            assert_eq!(list_changes(root).unwrap(), vec!["c1".to_string()]);
        });
    }
}
