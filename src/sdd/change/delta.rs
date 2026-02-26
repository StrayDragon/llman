use crate::sdd::spec::ison::parse_ison_document;
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

pub fn parse_delta_spec(content: &str) -> Result<DeltaPlan> {
    let raw: RawDeltaDocument = parse_ison_document(content, "delta spec")?;

    if let Some(kind) = raw.kind.as_ref()
        && kind != "llman.sdd.delta"
    {
        return Err(anyhow!(
            "delta spec kind must be `llman.sdd.delta`, got `{}`",
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
