use crate::sdd::change::delta::{DeltaPlan, RequirementBlock, parse_delta_spec};
use crate::sdd::project::templates::TemplateStyle;
use crate::sdd::spec::ison::{parse_ison_document, split_frontmatter};
use crate::sdd::spec::ison_table::{
    expect_fields, expect_fields_any_of, extract_all_ison_fences, get_optional_string,
    get_required_string, parse_and_merge_fences,
};
use anyhow::{Result, anyhow};
use regex::Regex;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Deserialize)]
struct RawSpecDocument {
    version: Option<String>,
    kind: Option<String>,
    name: Option<String>,
    purpose: String,
    requirements: Vec<RawRequirement>,
}

#[derive(Debug, Deserialize)]
struct RawRequirement {
    statement: String,
    scenarios: Vec<RawScenario>,
}

#[derive(Debug, Deserialize)]
struct RawScenario {
    text: String,
}

pub fn parse_spec(content: &str, name: &str, style: TemplateStyle) -> Result<Spec> {
    match style {
        TemplateStyle::Legacy => parse_spec_legacy_json(content, name),
        TemplateStyle::New => parse_spec_table_object(content, name),
    }
}

fn parse_spec_legacy_json(content: &str, name: &str) -> Result<Spec> {
    let (_, body) = split_frontmatter(content);
    let raw: RawSpecDocument = parse_ison_document(&body, "spec")?;

    if let Some(kind) = raw.kind.as_ref()
        && kind != "llman.sdd.spec"
    {
        return Err(anyhow!(
            "spec kind must be `llman.sdd.spec`, got `{}`",
            kind
        ));
    }

    let requirements = raw
        .requirements
        .into_iter()
        .map(|item| Requirement {
            text: item.statement.trim().to_string(),
            scenarios: item
                .scenarios
                .into_iter()
                .map(|scenario| Scenario {
                    raw_text: scenario.text.trim().to_string(),
                })
                .collect(),
        })
        .collect();

    Ok(Spec {
        name: raw.name.unwrap_or_else(|| name.to_string()),
        overview: raw.purpose.trim().to_string(),
        requirements,
        metadata: SpecMetadata {
            version: raw.version.unwrap_or_else(|| "1.0.0".to_string()),
            format: "llman-sdd-ison".to_string(),
        },
    })
}

fn parse_spec_table_object(content: &str, name: &str) -> Result<Spec> {
    let context = format!("spec `{}`", name);
    let (_, body) = split_frontmatter(content);
    let fences = extract_all_ison_fences(&body, &context)?;

    for fence in &fences {
        let trimmed = fence.payload.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return Err(anyhow!(
                "{context}: legacy JSON detected in ```ison payload at line {}. \
`llman sdd` only supports canonical table/object ISON; try `llman sdd-legacy ...` or rewrite the payload to `object.spec` + `table.requirements` + `table.scenarios`.",
                fence.start_line
            ));
        }
    }

    let merged = parse_and_merge_fences(&fences, &context)?;
    let allowed_blocks = ["object.spec", "table.requirements", "table.scenarios"];
    for key in merged.blocks().keys() {
        if !allowed_blocks.contains(&key.as_str()) {
            return Err(anyhow!(
                "{context}: unknown canonical block `{}` (expected: {})",
                key,
                allowed_blocks.join(", ")
            ));
        }
    }

    let meta = merged
        .get("object", "spec")
        .ok_or_else(|| anyhow!("{context}: missing required block `object.spec`"))?;
    expect_fields_any_of(
        meta,
        &[
            &["version", "kind", "name", "purpose"][..],
            &["kind", "name", "purpose"][..],
        ],
        &context,
    )?;
    if meta.rows.len() != 1 {
        return Err(anyhow!(
            "{context}: `object.spec` must have exactly 1 row, got {}",
            meta.rows.len()
        ));
    }
    let meta_row = &meta.rows[0];
    let version = get_optional_string(meta_row, "version", &context)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "1.0.0".to_string());
    let kind = get_required_string(meta_row, "kind", &context, false)?
        .trim()
        .to_string();
    let feature_id = get_required_string(meta_row, "name", &context, false)?
        .trim()
        .to_string();
    let purpose = get_required_string(meta_row, "purpose", &context, false)?
        .trim()
        .to_string();

    if kind != "llman.sdd.spec" {
        return Err(anyhow!(
            "{context}: spec kind must be `llman.sdd.spec`, got `{}`",
            kind
        ));
    }

    let requirements_block = merged
        .get("table", "requirements")
        .ok_or_else(|| anyhow!("{context}: missing required block `table.requirements`"))?;
    expect_fields(
        requirements_block,
        &["req_id", "title", "statement"],
        &context,
    )?;

    let mut req_id_order: Vec<String> = Vec::new();
    let mut req_id_seen = std::collections::HashSet::new();
    let mut req_statement_by_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (idx, row) in requirements_block.rows.iter().enumerate() {
        let row_ctx = format!("{context}: table.requirements row {}", idx + 1);
        let req_id = get_required_string(row, "req_id", &row_ctx, false)?
            .trim()
            .to_string();
        let statement = get_required_string(row, "statement", &row_ctx, false)?
            .trim()
            .to_string();
        if !req_id_seen.insert(req_id.clone()) {
            return Err(anyhow!(
                "{context}: duplicate requirement `req_id` `{}`",
                req_id
            ));
        }
        req_id_order.push(req_id.clone());
        req_statement_by_id.insert(req_id, statement);
    }

    let scenarios_block = merged
        .get("table", "scenarios")
        .ok_or_else(|| anyhow!("{context}: missing required block `table.scenarios`"))?;
    expect_fields(
        scenarios_block,
        &["req_id", "id", "given", "when", "then"],
        &context,
    )?;

    let mut scenario_seen = std::collections::HashSet::new();
    let mut scenarios_by_req: std::collections::HashMap<String, Vec<Scenario>> =
        std::collections::HashMap::new();
    for (idx, row) in scenarios_block.rows.iter().enumerate() {
        let row_ctx = format!("{context}: table.scenarios row {}", idx + 1);
        let req_id = get_required_string(row, "req_id", &row_ctx, false)?
            .trim()
            .to_string();
        if !req_id_seen.contains(&req_id) {
            return Err(anyhow!(
                "{context}: scenario references unknown requirement `req_id` `{}`",
                req_id
            ));
        }
        let scenario_id = get_required_string(row, "id", &row_ctx, false)?
            .trim()
            .to_string();
        let key = format!("{}::{}", req_id, scenario_id);
        if !scenario_seen.insert(key) {
            return Err(anyhow!(
                "{context}: duplicate scenario `(req_id, id)` = (`{}`, `{}`)",
                req_id,
                scenario_id
            ));
        }

        let given = get_required_string(row, "given", &row_ctx, true)?
            .trim()
            .to_string();
        let when = get_required_string(row, "when", &row_ctx, false)?
            .trim()
            .to_string();
        let then = get_required_string(row, "then", &row_ctx, false)?
            .trim()
            .to_string();

        let raw_text = if given.is_empty() {
            format!("WHEN: {when}\nTHEN: {then}")
        } else {
            format!("GIVEN: {given}\nWHEN: {when}\nTHEN: {then}")
        };

        scenarios_by_req
            .entry(req_id)
            .or_default()
            .push(Scenario { raw_text });
    }

    let requirements = req_id_order
        .into_iter()
        .map(|req_id| Requirement {
            text: req_statement_by_id
                .remove(&req_id)
                .unwrap_or_default()
                .trim()
                .to_string(),
            scenarios: scenarios_by_req.remove(&req_id).unwrap_or_default(),
        })
        .collect();

    Ok(Spec {
        name: if feature_id.is_empty() {
            name.to_string()
        } else {
            feature_id
        },
        overview: purpose,
        requirements,
        metadata: SpecMetadata {
            version,
            format: "llman-sdd-ison".to_string(),
        },
    })
}

pub fn parse_change(
    content: &str,
    name: &str,
    change_dir: &Path,
    style: TemplateStyle,
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
            version: "1.0.0".to_string(),
            format: "openspec-change".to_string(),
        },
    })
}

fn parse_change_deltas(
    what_changes: &str,
    change_dir: &Path,
    style: TemplateStyle,
) -> Result<Vec<Delta>> {
    let spec_deltas = parse_delta_specs(change_dir, style)?;
    if !spec_deltas.is_empty() {
        return Ok(spec_deltas);
    }
    Ok(parse_simple_deltas(what_changes))
}

pub fn parse_delta_specs(change_dir: &Path, style: TemplateStyle) -> Result<Vec<Delta>> {
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
        let plan = parse_delta_spec(&content, style, &format!("delta spec `{}`", spec_name))?;
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
            description: format!("Add requirement: {}", block.name),
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
            description: format!("Modify requirement: {}", block.name),
            requirement: Some(requirement.clone()),
            requirements: Some(vec![requirement]),
            rename: None,
        });
    }
    for removed in &plan.removed {
        let text = removed
            .name
            .clone()
            .unwrap_or_else(|| removed.req_id.clone());
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
    Requirement {
        text: block.statement.clone(),
        scenarios: block
            .scenarios
            .iter()
            .map(|scenario| Scenario {
                raw_text: scenario.text.clone(),
            })
            .collect(),
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
