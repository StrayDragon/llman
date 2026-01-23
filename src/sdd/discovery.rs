use crate::sdd::constants::LLMANSPEC_DIR_NAME;
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
