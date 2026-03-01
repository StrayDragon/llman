use crate::sdd::spec::ison_table::{
    dumps_canonical, expect_fields, expect_fields_any_of, extract_all_ison_fences,
    get_optional_string, get_required_string, parse_and_merge_fences,
};
use anyhow::{Result, anyhow};
use ison_rs::{Block, Document, FieldInfo, Row, Value};

pub const V1_VERSION: &str = "1.0.0";
pub const SPEC_KIND: &str = "llman.sdd.spec";
pub const DELTA_KIND: &str = "llman.sdd.delta";

#[derive(Debug, Clone)]
pub struct SpecMeta {
    pub version: String,
    pub kind: String,
    pub name: String,
    pub purpose: String,
}

#[derive(Debug, Clone)]
pub struct RequirementRow {
    pub req_id: String,
    pub title: String,
    pub statement: String,
}

#[derive(Debug, Clone)]
pub struct ScenarioRow {
    pub req_id: String,
    pub id: String,
    pub given: String,
    pub when: String,
    pub then: String,
}

#[derive(Debug, Clone)]
pub struct CanonicalSpec {
    pub meta: SpecMeta,
    pub requirements: Vec<RequirementRow>,
    pub scenarios: Vec<ScenarioRow>,
}

#[derive(Debug, Clone)]
pub struct DeltaMeta {
    pub version: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct DeltaOpRow {
    pub op: String,
    pub req_id: String,
    pub title: Option<String>,
    pub statement: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CanonicalDelta {
    pub meta: DeltaMeta,
    pub ops: Vec<DeltaOpRow>,
    pub scenarios: Vec<ScenarioRow>,
}

pub fn parse_spec_body(content: &str, context: &str) -> Result<CanonicalSpec> {
    let fences = extract_all_ison_fences(content, context)?;
    for fence in &fences {
        let trimmed = fence.payload.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return Err(anyhow!(
                "{context}: legacy JSON detected in ```ison payload at line {}. \
Use `llman sdd-legacy ...` or rewrite to canonical table/object ISON blocks.",
                fence.start_line
            ));
        }
    }

    let merged = parse_and_merge_fences(&fences, context)?;
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

    let meta_block = merged
        .get("object", "spec")
        .ok_or_else(|| anyhow!("{context}: missing required block `object.spec`"))?;
    expect_fields_any_of(
        meta_block,
        &[
            &["version", "kind", "name", "purpose"][..],
            &["kind", "name", "purpose"][..],
        ],
        context,
    )?;
    if meta_block.rows.len() != 1 {
        return Err(anyhow!(
            "{context}: `object.spec` must have exactly 1 row, got {}",
            meta_block.rows.len()
        ));
    }
    let meta_row = &meta_block.rows[0];
    let version = get_optional_string(meta_row, "version", context)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| V1_VERSION.to_string());
    let kind = get_required_string(meta_row, "kind", context, false)?
        .trim()
        .to_string();
    let name = get_required_string(meta_row, "name", context, false)?
        .trim()
        .to_string();
    let purpose = get_required_string(meta_row, "purpose", context, false)?
        .trim()
        .to_string();

    if version != V1_VERSION {
        return Err(anyhow!(
            "{context}: spec version must be `{}`, got `{}`",
            V1_VERSION,
            version
        ));
    }
    if kind != SPEC_KIND {
        return Err(anyhow!(
            "{context}: spec kind must be `{}`, got `{}`",
            SPEC_KIND,
            kind
        ));
    }

    let req_block = merged
        .get("table", "requirements")
        .ok_or_else(|| anyhow!("{context}: missing required block `table.requirements`"))?;
    expect_fields(req_block, &["req_id", "title", "statement"], context)?;

    let mut requirements: Vec<RequirementRow> = Vec::new();
    let mut req_id_seen = std::collections::HashSet::new();
    for (idx, row) in req_block.rows.iter().enumerate() {
        let row_ctx = format!("{context}: table.requirements row {}", idx + 1);
        let req_id = get_required_string(row, "req_id", &row_ctx, false)?
            .trim()
            .to_string();
        let title = get_required_string(row, "title", &row_ctx, false)?
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
        requirements.push(RequirementRow {
            req_id,
            title,
            statement,
        });
    }

    let scenario_block = merged
        .get("table", "scenarios")
        .ok_or_else(|| anyhow!("{context}: missing required block `table.scenarios`"))?;
    expect_fields(
        scenario_block,
        &["req_id", "id", "given", "when", "then"],
        context,
    )?;

    let mut scenarios: Vec<ScenarioRow> = Vec::new();
    let mut scenario_key_seen = std::collections::HashSet::new();
    for (idx, row) in scenario_block.rows.iter().enumerate() {
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
        let id = get_required_string(row, "id", &row_ctx, false)?
            .trim()
            .to_string();
        let key = format!("{}::{}", req_id, id);
        if !scenario_key_seen.insert(key) {
            return Err(anyhow!(
                "{context}: duplicate scenario `(req_id, id)` = (`{}`, `{}`)",
                req_id,
                id
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

        scenarios.push(ScenarioRow {
            req_id,
            id,
            given,
            when,
            then,
        });
    }

    Ok(CanonicalSpec {
        meta: SpecMeta {
            version,
            kind,
            name,
            purpose,
        },
        requirements,
        scenarios,
    })
}

pub fn parse_delta_body(content: &str, context: &str) -> Result<CanonicalDelta> {
    let fences = extract_all_ison_fences(content, context)?;
    for fence in &fences {
        let trimmed = fence.payload.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return Err(anyhow!(
                "{context}: legacy JSON detected in ```ison payload at line {}. \
Use `llman sdd-legacy ...` or rewrite to canonical table/object ISON blocks.",
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

    let meta_block = merged
        .get("object", "delta")
        .ok_or_else(|| anyhow!("{context}: missing required block `object.delta`"))?;
    expect_fields_any_of(
        meta_block,
        &[&["version", "kind"][..], &["kind"][..]],
        context,
    )?;
    if meta_block.rows.len() != 1 {
        return Err(anyhow!(
            "{context}: `object.delta` must have exactly 1 row, got {}",
            meta_block.rows.len()
        ));
    }
    let meta_row = &meta_block.rows[0];
    let version = get_optional_string(meta_row, "version", context)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| V1_VERSION.to_string());
    let kind = get_required_string(meta_row, "kind", context, false)?
        .trim()
        .to_string();
    if version != V1_VERSION {
        return Err(anyhow!(
            "{context}: delta version must be `{}`, got `{}`",
            V1_VERSION,
            version
        ));
    }
    if kind != DELTA_KIND {
        return Err(anyhow!(
            "{context}: delta kind must be `{}`, got `{}`",
            DELTA_KIND,
            kind
        ));
    }

    let ops_block = merged
        .get("table", "ops")
        .ok_or_else(|| anyhow!("{context}: missing required block `table.ops`"))?;
    expect_fields(
        ops_block,
        &["op", "req_id", "title", "statement", "from", "to", "name"],
        context,
    )?;

    let mut ops: Vec<DeltaOpRow> = Vec::new();
    for (idx, row) in ops_block.rows.iter().enumerate() {
        let row_ctx = format!("{context}: table.ops row {}", idx + 1);
        let op = get_required_string(row, "op", &row_ctx, false)?
            .trim()
            .to_ascii_lowercase();
        let req_id = get_required_string(row, "req_id", &row_ctx, false)?
            .trim()
            .to_string();

        let title = get_optional_string(row, "title", &row_ctx)?.map(|v| v.trim().to_string());
        let statement =
            get_optional_string(row, "statement", &row_ctx)?.map(|v| v.trim().to_string());
        let from = get_optional_string(row, "from", &row_ctx)?.map(|v| v.trim().to_string());
        let to = get_optional_string(row, "to", &row_ctx)?.map(|v| v.trim().to_string());
        let name = get_optional_string(row, "name", &row_ctx)?.map(|v| v.trim().to_string());

        ops.push(DeltaOpRow {
            op,
            req_id,
            title,
            statement,
            from,
            to,
            name,
        });
    }

    let scenario_block = merged
        .get("table", "op_scenarios")
        .ok_or_else(|| anyhow!("{context}: missing required block `table.op_scenarios`"))?;
    expect_fields(
        scenario_block,
        &["req_id", "id", "given", "when", "then"],
        context,
    )?;

    let mut scenarios: Vec<ScenarioRow> = Vec::new();
    for (idx, row) in scenario_block.rows.iter().enumerate() {
        let row_ctx = format!("{context}: table.op_scenarios row {}", idx + 1);
        let req_id = get_required_string(row, "req_id", &row_ctx, false)?
            .trim()
            .to_string();
        let id = get_required_string(row, "id", &row_ctx, false)?
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
        scenarios.push(ScenarioRow {
            req_id,
            id,
            given,
            when,
            then,
        });
    }

    Ok(CanonicalDelta {
        meta: DeltaMeta { version, kind },
        ops,
        scenarios,
    })
}

pub fn dump_spec_payload(spec: &CanonicalSpec, align_columns: bool) -> String {
    let mut doc = Document::new();
    doc.blocks.push(block_object_spec(spec));
    doc.blocks.push(block_requirements(&spec.requirements));
    doc.blocks
        .push(block_scenarios("scenarios", &spec.scenarios));
    dumps_canonical(&doc, align_columns)
}

pub fn dump_delta_payload(delta: &CanonicalDelta, align_columns: bool) -> String {
    let mut doc = Document::new();
    doc.blocks.push(block_object_delta(delta));
    doc.blocks.push(block_ops(&delta.ops));
    doc.blocks
        .push(block_scenarios("op_scenarios", &delta.scenarios));
    dumps_canonical(&doc, align_columns)
}

fn block_with_fields(kind: &str, name: &str, fields: &[&str]) -> Block {
    let mut block = Block::new(kind, name);
    block.fields = fields.iter().map(|f| (*f).to_string()).collect();
    block.field_info = fields.iter().map(|f| FieldInfo::new(*f)).collect();
    block
}

fn block_object_spec(spec: &CanonicalSpec) -> Block {
    let mut block = block_with_fields("object", "spec", &["kind", "name", "purpose"]);
    let mut row = Row::new();
    row.insert("kind".to_string(), Value::String(spec.meta.kind.clone()));
    row.insert("name".to_string(), Value::String(spec.meta.name.clone()));
    row.insert(
        "purpose".to_string(),
        Value::String(spec.meta.purpose.clone()),
    );
    block.rows.push(row);
    block
}

fn block_object_delta(delta: &CanonicalDelta) -> Block {
    let mut block = block_with_fields("object", "delta", &["kind"]);
    let mut row = Row::new();
    row.insert("kind".to_string(), Value::String(delta.meta.kind.clone()));
    block.rows.push(row);
    block
}

fn block_requirements(requirements: &[RequirementRow]) -> Block {
    let mut block = block_with_fields("table", "requirements", &["req_id", "title", "statement"]);
    for req in requirements {
        let mut row = Row::new();
        row.insert("req_id".to_string(), Value::String(req.req_id.clone()));
        row.insert("title".to_string(), Value::String(req.title.clone()));
        row.insert(
            "statement".to_string(),
            Value::String(req.statement.clone()),
        );
        block.rows.push(row);
    }
    block
}

fn block_scenarios(name: &str, scenarios: &[ScenarioRow]) -> Block {
    let mut block = block_with_fields("table", name, &["req_id", "id", "given", "when", "then"]);
    for scenario in scenarios {
        let mut row = Row::new();
        row.insert("req_id".to_string(), Value::String(scenario.req_id.clone()));
        row.insert("id".to_string(), Value::String(scenario.id.clone()));
        row.insert("given".to_string(), Value::String(scenario.given.clone()));
        row.insert("when".to_string(), Value::String(scenario.when.clone()));
        row.insert("then".to_string(), Value::String(scenario.then.clone()));
        block.rows.push(row);
    }
    block
}

fn block_ops(ops: &[DeltaOpRow]) -> Block {
    let mut block = block_with_fields(
        "table",
        "ops",
        &["op", "req_id", "title", "statement", "from", "to", "name"],
    );
    for op in ops {
        let mut row = Row::new();
        row.insert("op".to_string(), Value::String(op.op.clone()));
        row.insert("req_id".to_string(), Value::String(op.req_id.clone()));
        row.insert(
            "title".to_string(),
            op.title
                .as_ref()
                .map(|v| Value::String(v.clone()))
                .unwrap_or(Value::Null),
        );
        row.insert(
            "statement".to_string(),
            op.statement
                .as_ref()
                .map(|v| Value::String(v.clone()))
                .unwrap_or(Value::Null),
        );
        row.insert(
            "from".to_string(),
            op.from
                .as_ref()
                .map(|v| Value::String(v.clone()))
                .unwrap_or(Value::Null),
        );
        row.insert(
            "to".to_string(),
            op.to
                .as_ref()
                .map(|v| Value::String(v.clone()))
                .unwrap_or(Value::Null),
        );
        row.insert(
            "name".to_string(),
            op.name
                .as_ref()
                .map(|v| Value::String(v.clone()))
                .unwrap_or(Value::Null),
        );
        block.rows.push(row);
    }
    block
}
