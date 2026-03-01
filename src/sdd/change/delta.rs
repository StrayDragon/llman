use crate::sdd::project::templates::TemplateStyle;
use crate::sdd::spec::ison::parse_ison_document;
use crate::sdd::spec::ison_table::{
    expect_fields, expect_fields_any_of, extract_all_ison_fences, get_optional_string,
    get_required_string, parse_and_merge_fences,
};
use anyhow::{Result, anyhow};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct ScenarioBlock {
    pub scenario_id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct RequirementBlock {
    pub req_id: String,
    pub name: String,
    pub statement: String,
    pub scenarios: Vec<ScenarioBlock>,
}

#[derive(Debug, Clone)]
pub struct RemovedRequirement {
    pub req_id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeltaPlan {
    pub added: Vec<RequirementBlock>,
    pub modified: Vec<RequirementBlock>,
    pub removed: Vec<RemovedRequirement>,
    pub renamed: Vec<RenamePair>,
}

#[derive(Debug, Clone)]
pub struct RenamePair {
    pub req_id: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize)]
struct RawDeltaDocument {
    kind: Option<String>,
    ops: Vec<RawDeltaOp>,
}

#[derive(Debug, Deserialize)]
struct RawScenario {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct RawDeltaOp {
    op: String,
    req_id: Option<String>,
    title: Option<String>,
    statement: Option<String>,
    scenarios: Option<Vec<RawScenario>>,
    from: Option<String>,
    to: Option<String>,
    name: Option<String>,
}

pub fn normalize_requirement_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn parse_delta_spec(content: &str, style: TemplateStyle, context: &str) -> Result<DeltaPlan> {
    match style {
        TemplateStyle::Legacy => parse_delta_spec_legacy_json(content, context),
        TemplateStyle::New => parse_delta_spec_table_object(content, context),
    }
}

fn parse_delta_spec_legacy_json(content: &str, context: &str) -> Result<DeltaPlan> {
    let raw: RawDeltaDocument = parse_ison_document(content, context)?;

    if let Some(kind) = raw.kind.as_ref()
        && kind != "llman.sdd.delta"
    {
        return Err(anyhow!(
            "{context}: delta spec kind must be `llman.sdd.delta`, got `{}`",
            kind
        ));
    }

    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();
    let mut renamed = Vec::new();

    for (index, op) in raw.ops.into_iter().enumerate() {
        let op_name = op.op.trim().to_ascii_lowercase();
        match op_name.as_str() {
            "add_requirement" | "added" | "add" => {
                added.push(parse_requirement_op(op, index, "add_requirement")?);
            }
            "modify_requirement" | "modified" | "modify" | "update" => {
                modified.push(parse_requirement_op(op, index, "modify_requirement")?);
            }
            "remove_requirement" | "removed" | "remove" => {
                let req_id = required_field(op.req_id, index, "remove_requirement.req_id")?;
                removed.push(RemovedRequirement {
                    req_id,
                    name: op.name.map(|v| normalize_requirement_name(&v)),
                });
            }
            "rename_requirement" | "renamed" | "rename" => {
                renamed.push(RenamePair {
                    req_id: required_field(op.req_id, index, "rename_requirement.req_id")?,
                    from: required_field(op.from, index, "rename_requirement.from")?,
                    to: required_field(op.to, index, "rename_requirement.to")?,
                });
            }
            _ => {
                return Err(anyhow!(
                    "delta spec op[{}]: unsupported op `{}`",
                    index,
                    op.op
                ));
            }
        }
    }

    Ok(DeltaPlan {
        added,
        modified,
        removed,
        renamed,
    })
}

fn parse_requirement_op(op: RawDeltaOp, index: usize, op_name: &str) -> Result<RequirementBlock> {
    let req_id = required_field(op.req_id, index, &format!("{op_name}.req_id"))?;
    let title = required_field(op.title, index, &format!("{op_name}.title"))?;
    let statement = required_field(op.statement, index, &format!("{op_name}.statement"))?;

    let scenarios = op
        .scenarios
        .unwrap_or_default()
        .into_iter()
        .map(|raw| ScenarioBlock {
            scenario_id: raw.id.trim().to_string(),
            text: raw.text.trim().to_string(),
        })
        .collect();

    Ok(RequirementBlock {
        req_id,
        name: normalize_requirement_name(&title),
        statement: statement.trim().to_string(),
        scenarios,
    })
}

fn required_field(value: Option<String>, index: usize, field_name: &str) -> Result<String> {
    let value = value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("delta spec op[{index}] missing required field `{field_name}`"))?;
    Ok(value)
}

fn parse_delta_spec_table_object(content: &str, context: &str) -> Result<DeltaPlan> {
    let fences = extract_all_ison_fences(content, context)?;
    for fence in &fences {
        let trimmed = fence.payload.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return Err(anyhow!(
                "{context}: legacy JSON detected in ```ison payload at line {}. \
`llman sdd` only supports canonical table/object ISON; try `llman sdd-legacy ...` or rewrite the payload to `object.delta` + `table.ops` + `table.op_scenarios`.",
                fence.start_line
            ));
        }
    }

    let merged = parse_and_merge_fences(&fences, context)?;
    let allowed_blocks = ["object.delta", "table.ops", "table.op_scenarios"];
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
        .get("object", "delta")
        .ok_or_else(|| anyhow!("{context}: missing required block `object.delta`"))?;
    expect_fields_any_of(meta, &[&["version", "kind"][..], &["kind"][..]], context)?;
    if meta.rows.len() != 1 {
        return Err(anyhow!(
            "{context}: `object.delta` must have exactly 1 row, got {}",
            meta.rows.len()
        ));
    }
    let meta_row = &meta.rows[0];
    let version = get_optional_string(meta_row, "version", context)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "1.0.0".to_string());
    let kind = get_required_string(meta_row, "kind", context, false)?
        .trim()
        .to_string();
    if version != "1.0.0" {
        return Err(anyhow!(
            "{context}: delta spec version must be `1.0.0`, got `{}`",
            version
        ));
    }
    if kind != "llman.sdd.delta" {
        return Err(anyhow!(
            "{context}: delta spec kind must be `llman.sdd.delta`, got `{}`",
            kind
        ));
    }

    let ops = merged
        .get("table", "ops")
        .ok_or_else(|| anyhow!("{context}: missing required block `table.ops`"))?;
    expect_fields(
        ops,
        &["op", "req_id", "title", "statement", "from", "to", "name"],
        context,
    )?;

    let mut added: Vec<RequirementBlock> = Vec::new();
    let mut modified: Vec<RequirementBlock> = Vec::new();
    let mut removed: Vec<RemovedRequirement> = Vec::new();
    let mut renamed: Vec<RenamePair> = Vec::new();

    for (idx, row) in ops.rows.iter().enumerate() {
        let row_ctx = format!("{context}: table.ops row {}", idx + 1);
        let op = get_required_string(row, "op", &row_ctx, false)?
            .trim()
            .to_ascii_lowercase();

        let ensure_null = |field: &str| -> Result<()> {
            if get_optional_string(row, field, &row_ctx)?.is_some() {
                return Err(anyhow!(
                    "{context}: `{}` must be `~` for op `{}` (row {})",
                    field,
                    op,
                    idx + 1
                ));
            }
            Ok(())
        };
        match op.as_str() {
            "add_requirement" => {
                ensure_null("from")?;
                ensure_null("to")?;
                ensure_null("name")?;
                let req_id = get_required_string(row, "req_id", &row_ctx, false)?
                    .trim()
                    .to_string();
                let title = get_required_string(row, "title", &row_ctx, false)?;
                let statement = get_required_string(row, "statement", &row_ctx, false)?;
                added.push(RequirementBlock {
                    req_id,
                    name: normalize_requirement_name(&title),
                    statement: statement.trim().to_string(),
                    scenarios: Vec::new(),
                });
            }
            "modify_requirement" => {
                ensure_null("from")?;
                ensure_null("to")?;
                ensure_null("name")?;
                let req_id = get_required_string(row, "req_id", &row_ctx, false)?
                    .trim()
                    .to_string();
                let title = get_required_string(row, "title", &row_ctx, false)?;
                let statement = get_required_string(row, "statement", &row_ctx, false)?;
                modified.push(RequirementBlock {
                    req_id,
                    name: normalize_requirement_name(&title),
                    statement: statement.trim().to_string(),
                    scenarios: Vec::new(),
                });
            }
            "remove_requirement" => {
                ensure_null("title")?;
                ensure_null("statement")?;
                ensure_null("from")?;
                ensure_null("to")?;
                let req_id = get_required_string(row, "req_id", &row_ctx, false)?
                    .trim()
                    .to_string();
                let name = get_optional_string(row, "name", &row_ctx)?
                    .map(|v| normalize_requirement_name(&v));
                removed.push(RemovedRequirement { req_id, name });
            }
            "rename_requirement" => {
                ensure_null("title")?;
                ensure_null("statement")?;
                ensure_null("name")?;
                let req_id = get_required_string(row, "req_id", &row_ctx, false)?
                    .trim()
                    .to_string();
                let from = get_required_string(row, "from", &row_ctx, false)?
                    .trim()
                    .to_string();
                let to = get_required_string(row, "to", &row_ctx, false)?
                    .trim()
                    .to_string();
                renamed.push(RenamePair { req_id, from, to });
            }
            _ => {
                return Err(anyhow!(
                    "{context}: unsupported op `{}` (expected add_requirement/modify_requirement/remove_requirement/rename_requirement)",
                    op
                ));
            }
        }
    }

    let scenarios = merged
        .get("table", "op_scenarios")
        .ok_or_else(|| anyhow!("{context}: missing required block `table.op_scenarios`"))?;
    expect_fields(
        scenarios,
        &["req_id", "id", "given", "when", "then"],
        context,
    )?;

    for (idx, row) in scenarios.rows.iter().enumerate() {
        let row_ctx = format!("{context}: table.op_scenarios row {}", idx + 1);
        let req_id = get_required_string(row, "req_id", &row_ctx, false)?
            .trim()
            .to_string();
        let scenario_id = get_required_string(row, "id", &row_ctx, false)?
            .trim()
            .to_string();
        let given = get_required_string(row, "given", &row_ctx, true)?
            .trim()
            .to_string();
        let when = get_required_string(row, "when", &row_ctx, false)?
            .trim()
            .to_string();
        let then = get_required_string(row, "then", &row_ctx, false)?
            .trim()
            .to_string();

        let text = if given.is_empty() {
            format!("WHEN: {when}\nTHEN: {then}")
        } else {
            format!("GIVEN: {given}\nWHEN: {when}\nTHEN: {then}")
        };

        let scenario = ScenarioBlock { scenario_id, text };

        if let Some(target) = added.iter_mut().find(|b| b.req_id == req_id) {
            target.scenarios.push(scenario);
            continue;
        }
        if let Some(target) = modified.iter_mut().find(|b| b.req_id == req_id) {
            target.scenarios.push(scenario);
            continue;
        }

        return Err(anyhow!(
            "{context}: op scenario references unknown or unsupported `req_id` `{}` (must match an add/modify op)",
            req_id
        ));
    }

    Ok(DeltaPlan {
        added,
        modified,
        removed,
        renamed,
    })
}
