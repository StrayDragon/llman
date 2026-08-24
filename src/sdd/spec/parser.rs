use anyhow::{Result, anyhow};
use regex::Regex;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Scenario {
    #[serde(rename = "rawText")]
    pub raw_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Requirement {
    pub text: String,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Spec {
    pub name: String,
    pub overview: String,
    pub requirements: Vec<Requirement>,
    pub metadata: SpecMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecMetadata {
    pub format: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Change {
    pub name: String,
    pub why: String,
    #[serde(rename = "whatChanges")]
    pub what_changes: String,
    pub deltas: Vec<Delta>,
    pub metadata: ChangeMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangeMetadata {
    pub format: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeltaOperation {
    Added,
    Modified,
    Removed,
    Renamed,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenamePair {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Delta {
    pub spec: String,
    pub operation: DeltaOperation,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<Requirement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rename: Option<RenamePair>,
}

pub fn parse_change(content: &str, name: &str, _change_dir: &Path) -> Result<Change> {
    let why =
        extract_section(content, "Why").ok_or_else(|| anyhow!("Change must have a Why section"))?;
    let what_changes = extract_section(content, "What Changes")
        .ok_or_else(|| anyhow!("Change must have a What Changes section"))?;

    Ok(Change {
        name: name.to_string(),
        why: why.trim().to_string(),
        what_changes: what_changes.trim().to_string(),
        deltas: Vec::new(),
        metadata: ChangeMetadata {
            format: "openspec-change".to_string(),
        },
    })
}

fn extract_section(content: &str, title: &str) -> Option<String> {
    let normalized = content.replace("\r\n", "\n").replace("\r", "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let header_re = Regex::new(r"^##\s+(.+)$").expect("regex");

    let mut start_index = None;
    for (idx, line) in lines.iter().enumerate() {
        if let Some(caps) = header_re.captures(line) {
            let header = caps.get(1)?.as_str().trim();
            if header.eq_ignore_ascii_case(title) {
                start_index = Some(idx + 1);
                break;
            }
        }
    }

    let start = start_index?;
    let mut collected = Vec::new();
    for line in lines.iter().skip(start) {
        if header_re.is_match(line) {
            break;
        }
        collected.push(*line);
    }
    Some(collected.join("\n").trim().to_string())
}
