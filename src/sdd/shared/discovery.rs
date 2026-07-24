use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use anyhow::{Result, bail};
use std::fs;
use std::path::Path;

pub fn list_changes(root: &Path) -> Result<Vec<String>> {
    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");
    let mut result = Vec::new();
    let entries = match fs::read_dir(changes_dir) {
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
        if name.starts_with('.') || name == "archive" {
            continue;
        }
        let proposal_path = entry.path().join("proposal.md");
        if proposal_path.exists() {
            result.push(name);
        }
    }

    result.sort();
    Ok(result)
}

pub fn extract_archived_change_id(dir_name: &str) -> Option<String> {
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

pub fn list_archived_changes(root: &Path) -> Result<Vec<String>> {
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

pub fn list_specs(root: &Path) -> Result<Vec<String>> {
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
        let spec_path = entry.path().join(SPEC_FILE);
        if spec_path.exists() {
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
pub struct ResolvedChange {
    /// The canonical change id that the user's input resolved to.
    pub id: String,
    /// The (trimmed) user-provided input, kept for hint formatting.
    pub input: String,
    /// `false` for an exact match, `true` when `input` was a unique prefix of `id`.
    pub via_prefix: bool,
}

impl ResolvedChange {
    /// Returns the "'input' -> 'resolved' (prefix match)" hint when this was a
    /// prefix match, or `None` for an exact match (no hint is emitted then).
    pub fn prefix_hint(&self) -> Option<String> {
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
pub fn resolve_change_id_human(root: &Path, input: &str) -> Result<String> {
    let resolved = resolve_change_id(root, input)?;
    if let Some(hint) = resolved.prefix_hint() {
        eprint!("{hint}");
    }
    Ok(resolved.id)
}

/// Resolve a user-provided change name input to a canonical change id.
///
/// Resolution priority:
/// 1. Exact match against active changes (`llmanspec/changes/<input>/proposal.md` exists)
/// 2. Prefix match against active changes (directory name starts with `input`)
/// 3. Prefix match against archived changes (change id portion starts with `input`)
///
/// Returns the resolved change id (plus the `via_prefix` flag for the r112 hint)
/// on success. Errors with a descriptive message on multi-match (lists all
/// candidates) or no-match.
pub fn resolve_change_id(root: &Path, input: &str) -> Result<ResolvedChange> {
    use crate::sdd::shared::match_utils::{PrefixOutcome, prefix_resolve};

    let input = input.trim();
    if input.is_empty() {
        bail!("change id must not be empty");
    }

    let active = list_changes(root)?;
    let archived = list_archived_changes(root)?;

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
                .map(|s| format!("  - {s}"))
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
    suggestions.extend(active);
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
