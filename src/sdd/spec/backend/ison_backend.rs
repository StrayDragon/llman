use crate::sdd::project::config::SpecStyle;
use crate::sdd::spec::backend::{DumpOptions, SpecBackend};
use crate::sdd::spec::fence::extract_all_code_fences;
use crate::sdd::spec::ir::{
    DeltaOpEntry, DeltaSpecDoc, MainSpecDoc, RequirementEntry, ScenarioEntry,
};
use crate::sdd::spec::ison_table::render_ison_fence;
use crate::sdd::spec::ison_v1;
use anyhow::{Result, anyhow};

pub struct IsonBackend;

pub static BACKEND: IsonBackend = IsonBackend;

impl SpecBackend for IsonBackend {
    fn style(&self) -> SpecStyle {
        SpecStyle::Ison
    }

    fn parse_main_spec(&self, content: &str, context: &str) -> Result<MainSpecDoc> {
        ensure_expected_style_fences(content, self.style(), context)?;

        let parsed = ison_v1::parse_spec_body(content, context)?;
        Ok(MainSpecDoc {
            kind: parsed.meta.kind,
            name: parsed.meta.name,
            purpose: parsed.meta.purpose,
            requirements: parsed
                .requirements
                .into_iter()
                .map(|row| RequirementEntry {
                    req_id: row.req_id,
                    title: row.title,
                    statement: row.statement,
                })
                .collect(),
            scenarios: parsed
                .scenarios
                .into_iter()
                .map(|row| ScenarioEntry {
                    req_id: row.req_id,
                    id: row.id,
                    given: row.given,
                    when_: row.when,
                    then_: row.then,
                })
                .collect(),
        })
    }

    fn parse_delta_spec(&self, content: &str, context: &str) -> Result<DeltaSpecDoc> {
        ensure_expected_style_fences(content, self.style(), context)?;

        let parsed = ison_v1::parse_delta_body(content, context)?;
        Ok(DeltaSpecDoc {
            kind: parsed.meta.kind,
            ops: parsed
                .ops
                .into_iter()
                .map(|row| DeltaOpEntry {
                    op: row.op,
                    req_id: row.req_id,
                    title: row.title,
                    statement: row.statement,
                    from: row.from,
                    to: row.to,
                    name: row.name,
                })
                .collect(),
            op_scenarios: parsed
                .scenarios
                .into_iter()
                .map(|row| ScenarioEntry {
                    req_id: row.req_id,
                    id: row.id,
                    given: row.given,
                    when_: row.when,
                    then_: row.then,
                })
                .collect(),
        })
    }

    fn dump_main_spec(&self, doc: &MainSpecDoc, options: DumpOptions) -> Result<String> {
        let spec = ison_v1::CanonicalSpec {
            meta: ison_v1::SpecMeta {
                kind: doc.kind.clone(),
                name: doc.name.clone(),
                purpose: doc.purpose.clone(),
            },
            requirements: doc
                .requirements
                .iter()
                .cloned()
                .map(|req| ison_v1::RequirementRow {
                    req_id: req.req_id,
                    title: req.title,
                    statement: req.statement,
                })
                .collect(),
            scenarios: doc
                .scenarios
                .iter()
                .cloned()
                .map(|scenario| ison_v1::ScenarioRow {
                    req_id: scenario.req_id,
                    id: scenario.id,
                    given: scenario.given,
                    when: scenario.when_,
                    then: scenario.then_,
                })
                .collect(),
        };

        Ok(ison_v1::dump_spec_payload(&spec, options.pretty_ison))
    }

    fn dump_delta_spec(&self, doc: &DeltaSpecDoc, options: DumpOptions) -> Result<String> {
        let delta = ison_v1::CanonicalDelta {
            meta: ison_v1::DeltaMeta {
                kind: doc.kind.clone(),
            },
            ops: doc
                .ops
                .iter()
                .cloned()
                .map(|op| ison_v1::DeltaOpRow {
                    op: op.op,
                    req_id: op.req_id,
                    title: op.title,
                    statement: op.statement,
                    from: op.from,
                    to: op.to,
                    name: op.name,
                })
                .collect(),
            scenarios: doc
                .op_scenarios
                .iter()
                .cloned()
                .map(|scenario| ison_v1::ScenarioRow {
                    req_id: scenario.req_id,
                    id: scenario.id,
                    given: scenario.given,
                    when: scenario.when_,
                    then: scenario.then_,
                })
                .collect(),
        };

        Ok(ison_v1::dump_delta_payload(&delta, options.pretty_ison))
    }
}

fn ensure_expected_style_fences(content: &str, expected: SpecStyle, context: &str) -> Result<()> {
    let fences = extract_all_code_fences(content)?;
    let supported = ["ison", "toon", "yaml"];
    let style_fences = fences
        .iter()
        .filter(|f| supported.contains(&f.lang.as_str()))
        .collect::<Vec<_>>();

    let expected_lang = expected.as_str();
    if let Some(found) = style_fences
        .iter()
        .find(|f| f.lang.as_str() != expected_lang)
    {
        return Err(anyhow!(
            "{context}: spec style mismatch: expected ```{expected_lang} fence, found ```{} at line {}",
            found.lang,
            found.start_line
        ));
    }
    if style_fences
        .iter()
        .all(|f| f.lang.as_str() != expected_lang)
    {
        return Err(anyhow!("{context}: missing ```{expected_lang} code block"));
    }
    Ok(())
}

#[allow(dead_code)]
pub fn render_main_spec_fence(payload: &str) -> String {
    render_ison_fence(payload)
}
