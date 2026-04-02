use crate::sdd::project::config::SpecStyle;
use crate::sdd::spec::backend::backend_for_style;
use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
use crate::sdd::spec::ison::split_frontmatter;
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

pub fn parse_spec(content: &str, name: &str, style: SpecStyle) -> Result<Spec> {
    let context = format!("spec `{}`", name);
    let (_, body) = split_frontmatter(content);
    let backend = backend_for_style(style);
    let doc = backend.parse_main_spec(&body, &context)?;
    Ok(convert_main_doc_to_spec(&doc, name, style))
}

pub fn parse_change(
    content: &str,
    name: &str,
    change_dir: &Path,
    style: SpecStyle,
) -> Result<Change> {
    let why =
        extract_section(content, "Why").ok_or_else(|| anyhow!("Change must have a Why section"))?;
    let what_changes = extract_section(content, "What Changes")
        .ok_or_else(|| anyhow!("Change must have a What Changes section"))?;

    let deltas = parse_change_deltas(&what_changes, change_dir, style)?;

    Ok(Change {
        name: name.to_string(),
        why: why.trim().to_string(),
        what_changes: what_changes.trim().to_string(),
        deltas,
        metadata: ChangeMetadata {
            format: "openspec-change".to_string(),
        },
    })
}

fn parse_change_deltas(
    what_changes: &str,
    change_dir: &Path,
    style: SpecStyle,
) -> Result<Vec<Delta>> {
    let spec_deltas = parse_delta_specs(change_dir, style)?;
    if !spec_deltas.is_empty() {
        return Ok(spec_deltas);
    }
    Ok(parse_simple_deltas(what_changes))
}

pub fn parse_delta_specs(change_dir: &Path, style: SpecStyle) -> Result<Vec<Delta>> {
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
        let content = std::fs::read_to_string(&spec_file)?;
        let context = format!("delta spec `{}`", spec_name);
        let backend = backend_for_style(style);
        let doc = backend.parse_delta_spec(&content, &context)?;
        deltas.extend(convert_delta_doc_to_deltas(&spec_name, &doc)?);
    }

    Ok(deltas)
}

fn convert_main_doc_to_spec(doc: &MainSpecDoc, fallback_name: &str, style: SpecStyle) -> Spec {
    let mut scenarios_by_req: std::collections::HashMap<&str, Vec<Scenario>> =
        std::collections::HashMap::new();
    for scenario in &doc.scenarios {
        scenarios_by_req
            .entry(scenario.req_id.as_str())
            .or_default()
            .push(Scenario {
                raw_text: render_scenario_text(
                    scenario.given.trim(),
                    scenario.when_.trim(),
                    scenario.then_.trim(),
                ),
            });
    }

    let requirements = doc
        .requirements
        .iter()
        .map(|req| Requirement {
            text: req.statement.trim().to_string(),
            scenarios: scenarios_by_req
                .remove(req.req_id.as_str())
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    Spec {
        name: if doc.name.trim().is_empty() {
            fallback_name.to_string()
        } else {
            doc.name.trim().to_string()
        },
        overview: doc.purpose.trim().to_string(),
        requirements,
        metadata: SpecMetadata {
            format: format!("llman-sdd-{}", style.as_str()),
        },
    }
}

fn convert_delta_doc_to_deltas(spec_name: &str, doc: &DeltaSpecDoc) -> Result<Vec<Delta>> {
    let mut scenarios_by_req: std::collections::HashMap<&str, Vec<Scenario>> =
        std::collections::HashMap::new();
    for scenario in &doc.op_scenarios {
        scenarios_by_req
            .entry(scenario.req_id.as_str())
            .or_default()
            .push(Scenario {
                raw_text: render_scenario_text(
                    scenario.given.trim(),
                    scenario.when_.trim(),
                    scenario.then_.trim(),
                ),
            });
    }

    let mut deltas = Vec::new();
    for op in &doc.ops {
        let op_kind = op.op.trim().to_ascii_lowercase();
        match op_kind.as_str() {
            "add_requirement" => {
                let title = op
                    .title
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("delta spec `{}`: add_requirement op for req_id `{}` is missing `title`", spec_name, op.req_id))?;
                let statement = op
                    .statement
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("delta spec `{}`: add_requirement op for req_id `{}` is missing `statement`", spec_name, op.req_id))?;
                let requirement = Requirement {
                    text: statement.to_string(),
                    scenarios: scenarios_by_req
                        .remove(op.req_id.as_str())
                        .unwrap_or_default(),
                };
                deltas.push(Delta {
                    spec: spec_name.to_string(),
                    operation: DeltaOperation::Added,
                    description: format!("Add requirement: {}", title),
                    requirement: Some(requirement.clone()),
                    requirements: Some(vec![requirement]),
                    rename: None,
                });
            }
            "modify_requirement" => {
                let title = op
                    .title
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("delta spec `{}`: modify_requirement op for req_id `{}` is missing `title`", spec_name, op.req_id))?;
                let statement = op
                    .statement
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("delta spec `{}`: modify_requirement op for req_id `{}` is missing `statement`", spec_name, op.req_id))?;
                let requirement = Requirement {
                    text: statement.to_string(),
                    scenarios: scenarios_by_req
                        .remove(op.req_id.as_str())
                        .unwrap_or_default(),
                };
                deltas.push(Delta {
                    spec: spec_name.to_string(),
                    operation: DeltaOperation::Modified,
                    description: format!("Modify requirement: {}", title),
                    requirement: Some(requirement.clone()),
                    requirements: Some(vec![requirement]),
                    rename: None,
                });
            }
            "remove_requirement" => {
                let text = op
                    .name
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| op.req_id.trim())
                    .to_string();
                let requirement = Requirement {
                    text: text.clone(),
                    scenarios: Vec::new(),
                };
                deltas.push(Delta {
                    spec: spec_name.to_string(),
                    operation: DeltaOperation::Removed,
                    description: format!("Remove requirement: {}", text),
                    requirement: Some(requirement.clone()),
                    requirements: Some(vec![requirement]),
                    rename: None,
                });
            }
            "rename_requirement" => {
                let from = op
                    .from
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("delta spec `{}`: rename_requirement op for req_id `{}` is missing `from`", spec_name, op.req_id))?;
                let to = op
                    .to
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("delta spec `{}`: rename_requirement op for req_id `{}` is missing `to`", spec_name, op.req_id))?;
                deltas.push(Delta {
                    spec: spec_name.to_string(),
                    operation: DeltaOperation::Renamed,
                    description: format!("Rename requirement from \"{}\" to \"{}\"", from, to),
                    requirement: None,
                    requirements: None,
                    rename: Some(RenamePair {
                        from: from.to_string(),
                        to: to.to_string(),
                    }),
                });
            }
            other => {
                return Err(anyhow!(
                    "delta spec `{}`: unsupported op `{}` (expected add_requirement/modify_requirement/remove_requirement/rename_requirement)",
                    spec_name,
                    other
                ));
            }
        }
    }

    Ok(deltas)
}

fn render_scenario_text(given: &str, when_: &str, then_: &str) -> String {
    if given.trim().is_empty() {
        format!("WHEN: {when_}\nTHEN: {then_}")
    } else {
        format!("GIVEN: {given}\nWHEN: {when_}\nTHEN: {then_}")
    }
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
