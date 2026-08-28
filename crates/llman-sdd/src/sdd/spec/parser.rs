use anyhow::{Result, anyhow};
use regex::Regex;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Scenario {
    #[serde(rename = "rawText")]
    pub(crate) raw_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Requirement {
    pub(crate) text: String,
    pub(crate) scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Change {
    pub(crate) name: String,
    pub(crate) why: String,
    #[serde(rename = "whatChanges")]
    pub(crate) what_changes: String,
    pub(crate) deltas: Vec<Delta>,
    pub(crate) metadata: ChangeMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChangeMetadata {
    pub(crate) format: String,
}

/// Part of the serialized `Change` shape; `deltas` is always empty since
/// delta specs were removed, but the JSON keys stay for output stability.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum DeltaOperation {
    Added,
    Modified,
    Removed,
    Renamed,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RenamePair {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Delta {
    pub(crate) spec: String,
    pub(crate) operation: DeltaOperation,
    pub(crate) description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) requirement: Option<Requirement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) requirements: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rename: Option<RenamePair>,
}

pub(crate) fn parse_change(content: &str, name: &str, _change_dir: &Path) -> Result<Change> {
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
