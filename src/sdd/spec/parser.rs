use crate::sdd::change::delta::{DeltaPlan, RequirementBlock, parse_delta_spec};
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
    pub version: String,
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
    pub version: String,
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

pub fn parse_spec(content: &str, name: &str) -> Result<Spec> {
    let purpose = extract_section(content, "Purpose")
        .ok_or_else(|| anyhow!("Spec must have a Purpose section"))?;
    let requirements_section = extract_section(content, "Requirements")
        .ok_or_else(|| anyhow!("Spec must have a Requirements section"))?;

    let requirements = parse_requirement_blocks(&requirements_section);

    Ok(Spec {
        name: name.to_string(),
        overview: purpose.trim().to_string(),
        requirements,
        metadata: SpecMetadata {
            version: "1.0.0".to_string(),
            format: "openspec".to_string(),
        },
    })
}

pub fn parse_change(content: &str, name: &str, change_dir: &Path) -> Result<Change> {
    let why =
        extract_section(content, "Why").ok_or_else(|| anyhow!("Change must have a Why section"))?;
    let what_changes = extract_section(content, "What Changes")
        .ok_or_else(|| anyhow!("Change must have a What Changes section"))?;

    let deltas = parse_change_deltas(&what_changes, change_dir)?;

    Ok(Change {
        name: name.to_string(),
        why: why.trim().to_string(),
        what_changes: what_changes.trim().to_string(),
        deltas,
        metadata: ChangeMetadata {
            version: "1.0.0".to_string(),
            format: "openspec-change".to_string(),
        },
    })
}

fn parse_change_deltas(what_changes: &str, change_dir: &Path) -> Result<Vec<Delta>> {
    let spec_deltas = parse_delta_specs(change_dir)?;
    if !spec_deltas.is_empty() {
        return Ok(spec_deltas);
    }
    Ok(parse_simple_deltas(what_changes))
}

pub fn parse_delta_specs(change_dir: &Path) -> Result<Vec<Delta>> {
    let mut deltas = Vec::new();
    let specs_dir = change_dir.join("specs");
    if !specs_dir.exists() {
        return Ok(deltas);
    }

    for entry in std::fs::read_dir(specs_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let spec_name = entry.file_name().to_string_lossy().to_string();
        let spec_file = entry.path().join("spec.md");
        if !spec_file.exists() {
            continue;
        }
        let content = std::fs::read_to_string(spec_file)?;
        let plan = parse_delta_spec(&content)?;
        deltas.extend(convert_plan_to_deltas(&spec_name, &plan));
    }

    Ok(deltas)
}

fn convert_plan_to_deltas(spec_name: &str, plan: &DeltaPlan) -> Vec<Delta> {
    let mut deltas = Vec::new();
    for block in &plan.added {
        let requirement = requirement_from_block(block);
        deltas.push(Delta {
            spec: spec_name.to_string(),
            operation: DeltaOperation::Added,
            description: format!("Add requirement: {}", requirement.text),
            requirement: Some(requirement.clone()),
            requirements: Some(vec![requirement]),
            rename: None,
        });
    }
    for block in &plan.modified {
        let requirement = requirement_from_block(block);
        deltas.push(Delta {
            spec: spec_name.to_string(),
            operation: DeltaOperation::Modified,
            description: format!("Modify requirement: {}", requirement.text),
            requirement: Some(requirement.clone()),
            requirements: Some(vec![requirement]),
            rename: None,
        });
    }
    for name in &plan.removed {
        let requirement = Requirement {
            text: name.clone(),
            scenarios: Vec::new(),
        };
        deltas.push(Delta {
            spec: spec_name.to_string(),
            operation: DeltaOperation::Removed,
            description: format!("Remove requirement: {}", name),
            requirement: Some(requirement.clone()),
            requirements: Some(vec![requirement]),
            rename: None,
        });
    }
    for rename in &plan.renamed {
        deltas.push(Delta {
            spec: spec_name.to_string(),
            operation: DeltaOperation::Renamed,
            description: format!(
                "Rename requirement from \"{}\" to \"{}\"",
                rename.from, rename.to
            ),
            requirement: None,
            requirements: None,
            rename: Some(RenamePair {
                from: rename.from.clone(),
                to: rename.to.clone(),
            }),
        });
    }

    deltas
}

fn requirement_from_block(block: &RequirementBlock) -> Requirement {
    let mut requirement_text = block.name.clone();
    let lines: Vec<&str> = block.raw.lines().collect();
    let mut body_lines = Vec::new();
    for line in lines.iter().skip(1) {
        if line.trim_start().starts_with('#') {
            break;
        }
        body_lines.push(*line);
    }
    let direct_content = body_lines.join("\n").trim().to_string();
    if let Some(first_line) = direct_content.lines().find(|l| !l.trim().is_empty()) {
        requirement_text = first_line.trim().to_string();
    }

    let scenarios = parse_scenarios_from_block(&lines);

    Requirement {
        text: requirement_text,
        scenarios,
    }
}

fn parse_scenarios_from_block(lines: &[&str]) -> Vec<Scenario> {
    let mut scenarios = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in lines.iter().skip(1) {
        if line.trim_start().starts_with("#### ") {
            if !current.is_empty() {
                scenarios.push(Scenario {
                    raw_text: current.join("\n").trim().to_string(),
                });
                current.clear();
            }
            continue;
        }
        if !line.trim().is_empty() {
            current.push((*line).to_string());
        }
    }
    if !current.is_empty() {
        scenarios.push(Scenario {
            raw_text: current.join("\n").trim().to_string(),
        });
    }
    scenarios
}

fn parse_simple_deltas(what_changes: &str) -> Vec<Delta> {
    let mut deltas = Vec::new();
    let re = Regex::new(r"^\s*-\s*\*\*([^*:]+)\*\*:\s*(.+)$").expect("regex");
    for line in what_changes.lines() {
        let caps = match re.captures(line) {
            Some(caps) => caps,
            None => continue,
        };
        let spec_name = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        let description = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
        if spec_name.is_empty() || description.is_empty() {
            continue;
        }

        let lower = description.to_lowercase();
        let operation = if lower.contains("rename") {
            DeltaOperation::Renamed
        } else if lower.contains("add") || lower.contains("create") || lower.contains("new") {
            DeltaOperation::Added
        } else if lower.contains("remove") || lower.contains("delete") {
            DeltaOperation::Removed
        } else {
            DeltaOperation::Modified
        };

        deltas.push(Delta {
            spec: spec_name.to_string(),
            operation,
            description: description.to_string(),
            requirement: None,
            requirements: None,
            rename: None,
        });
    }
    deltas
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

fn parse_requirement_blocks(section_body: &str) -> Vec<Requirement> {
    let lines: Vec<&str> = section_body.lines().collect();
    let mut requirements = Vec::new();
    let mut i = 0;
    let header_re = Regex::new(r"^###\s*Requirement:\s*(.+)\s*$").expect("regex");

    while i < lines.len() {
        let line = lines[i];
        let caps = match header_re.captures(line) {
            Some(caps) => caps,
            None => {
                i += 1;
                continue;
            }
        };
        let name = caps
            .get(1)
            .map(|m| m.as_str().trim())
            .unwrap_or("")
            .to_string();
        i += 1;

        let mut block_lines = vec![line.to_string()];
        while i < lines.len() {
            if header_re.is_match(lines[i]) || lines[i].trim_start().starts_with("## ") {
                break;
            }
            block_lines.push(lines[i].to_string());
            i += 1;
        }

        let block_refs: Vec<&str> = block_lines.iter().map(|l| l.as_str()).collect();
        let block = RequirementBlock {
            header_line: block_refs[0].to_string(),
            name: name.clone(),
            raw: block_lines.join("\n").trim_end().to_string(),
        };
        let requirement = requirement_from_block(&block);
        requirements.push(requirement);
    }

    requirements
}
