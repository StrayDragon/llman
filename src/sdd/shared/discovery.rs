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

/// Resolve a user-provided change name input to a canonical change id.
///
/// Resolution priority:
/// 1. Exact match against active changes (`llmanspec/changes/<input>/proposal.md` exists)
/// 2. Prefix match against active changes (directory name starts with `input`)
/// 3. Prefix match against archived changes (change id portion starts with `input`)
///
/// Returns the resolved change id on success. Errors with a descriptive message on
/// multi-match (lists all candidates) or no-match.
pub fn resolve_change_id(root: &Path, input: &str) -> Result<String> {
    let input = input.trim();
    if input.is_empty() {
        bail!("change id must not be empty");
    }

    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");

    // 1) Exact match against active changes
    let proposal_path = changes_dir.join(input).join("proposal.md");
    if proposal_path.exists() {
        return Ok(input.to_string());
    }

    // 2) Prefix match against active changes
    let active = list_changes(root)?;
    let active_prefix_matches: Vec<&String> =
        active.iter().filter(|id| id.starts_with(input)).collect();

    if active_prefix_matches.len() == 1 {
        return Ok(active_prefix_matches[0].clone());
    }
    if active_prefix_matches.len() > 1 {
        let candidates = active_prefix_matches
            .iter()
            .map(|s| format!("  - {s}"))
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "change '{input}' matches multiple active changes:\n{candidates}\nDid you mean one of these?"
        );
    }

    // 3) Prefix match against archived changes
    let archive_dir = changes_dir.join("archive");
    let mut archived_matches: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&archive_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if let Some(change_id) = extract_archived_change_id(&dir_name) {
                if change_id.starts_with(input) {
                    archived_matches.push(change_id);
                }
            }
        }
    }

    archived_matches.sort();
    archived_matches.dedup();

    if archived_matches.len() == 1 {
        return Ok(archived_matches.remove(0));
    }
    if archived_matches.len() > 1 {
        let candidates = archived_matches
            .iter()
            .map(|s| format!("  - {s}"))
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "change '{input}' matches multiple archived changes:\n{candidates}\nDid you mean one of these?"
        );
    }

    // 4) No match at all
    let mut suggestions = Vec::new();
    suggestions.extend(active);
    suggestions.extend(list_archived_changes(root)?);
    let nearby = crate::sdd::shared::match_utils::nearest_matches(input, &suggestions, 5);
    if nearby.is_empty() {
        bail!("change '{input}' not found.");
    }
    bail!(
        "change '{input}' not found. Did you mean: {}?",
        nearby.join(", ")
    );
}
