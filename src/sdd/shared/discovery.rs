use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use anyhow::Result;
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
        let spec_path = entry.path().join("spec.md");
        if spec_path.exists() {
            result.push(name);
        }
    }

    result.sort();
    Ok(result)
}
