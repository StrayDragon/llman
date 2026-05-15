use crate::sdd::spec::backend::{BACKEND, SpecBackend};
use crate::sdd::spec::fence::render_code_fence;
use anyhow::{Result, anyhow};

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

pub fn normalize_requirement_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn parse_delta_spec(content: &str, context: &str) -> Result<DeltaPlan> {
    let doc = BACKEND.parse_delta_spec(content, context)?;

    let mut added: Vec<RequirementBlock> = Vec::new();
    let mut modified: Vec<RequirementBlock> = Vec::new();
    let mut removed: Vec<RemovedRequirement> = Vec::new();
    let mut renamed: Vec<RenamePair> = Vec::new();

    for op in &doc.ops {
        let op_type = op.op.trim().to_ascii_lowercase();
        match op_type.as_str() {
            "add_requirement" => {
                let req_id = op.req_id.trim().to_string();
                let title = op.title.as_deref().unwrap_or("").trim().to_string();
                let statement = op.statement.as_deref().unwrap_or("").trim().to_string();
                added.push(RequirementBlock {
                    req_id,
                    name: normalize_requirement_name(&title),
                    statement,
                    scenarios: Vec::new(),
                });
            }
            "modify_requirement" => {
                let req_id = op.req_id.trim().to_string();
                let title = op.title.as_deref().unwrap_or("").trim().to_string();
                let statement = op.statement.as_deref().unwrap_or("").trim().to_string();
                modified.push(RequirementBlock {
                    req_id,
                    name: normalize_requirement_name(&title),
                    statement,
                    scenarios: Vec::new(),
                });
            }
            "remove_requirement" => {
                let req_id = op.req_id.trim().to_string();
                let name = op.name.as_deref().map(normalize_requirement_name);
                removed.push(RemovedRequirement { req_id, name });
            }
            "rename_requirement" => {
                let req_id = op.req_id.trim().to_string();
                let from = op.from.as_deref().unwrap_or("").trim().to_string();
                let to = op.to.as_deref().unwrap_or("").trim().to_string();
                renamed.push(RenamePair { req_id, from, to });
            }
            _ => {
                return Err(anyhow!(
                    "{context}: unsupported op `{}` (expected add_requirement/modify_requirement/remove_requirement/rename_requirement)",
                    op_type
                ));
            }
        }
    }

    for scenario in &doc.op_scenarios {
        let req_id = scenario.req_id.trim().to_string();
        let scenario_id = scenario.id.trim().to_string();
        let given = scenario.given.trim().to_string();
        let when = scenario.when_.trim().to_string();
        let then = scenario.then_.trim().to_string();

        let text = if given.is_empty() {
            format!("WHEN: {when}\nTHEN: {then}")
        } else {
            format!("GIVEN: {given}\nWHEN: {when}\nTHEN: {then}")
        };

        let sc = ScenarioBlock { scenario_id, text };

        if let Some(target) = added.iter_mut().find(|b| b.req_id == req_id) {
            target.scenarios.push(sc);
            continue;
        }
        if let Some(target) = modified.iter_mut().find(|b| b.req_id == req_id) {
            target.scenarios.push(sc);
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

/// Re-serialize a delta spec document back to a fenced markdown block.
pub fn render_delta_spec(doc: &crate::sdd::spec::ir::DeltaSpecDoc) -> Result<String> {
    let payload = BACKEND.dump_delta_spec(doc)?;
    Ok(render_code_fence("toon", &payload))
}
