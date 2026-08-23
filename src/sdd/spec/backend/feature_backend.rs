//! Single-track feature-as-spec backend: one `.feature` file per capability is
//! the only spec artifact (spec-format r131-r136).
//!
//! File anatomy:
//! ```text
//! # language: zh-CN          ← optional Gherkin language header
//! # capability: <name>       ← required (r133)
//! # purpose: <one-liner>     ← required (r133)
//! # scope: src/a, src/b      ← required (r133); drives staleness
//!
//! 功能: <title>
//!   @req:<id> @human         ← constraint layer (rule; statement = description)
//!   场景: <title>
//!     <free-text statement lines>
//!
//!   @req:<id> @executable    ← acceptance layer (runner-bound)
//!   场景: <scenario-id>
//!     假如/当/那么 …
//! ```
//!
//! Scenarios nested inside Gherkin `Rule:` blocks are rejected: rstest-bdd's
//! `scenarios!` macro silently skips them, so accepting them here would hide
//! executable scenarios from the runner (design D1).

use crate::sdd::spec::backend::SpecBackend;
use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc, RequirementEntry, ScenarioEntry};
use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

pub struct FeatureBackend;

/// Tier of a scenario under the single-track grammar (r132).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioTier {
    /// Human-owned constraint (`@human`); never runner-bound.
    Constraint,
    /// Runner-bound acceptance (`@executable`).
    Acceptance,
    /// Explicit manual-review waiver (`@human @manual`).
    Manual,
}

/// Richly parsed scenario retaining everything the lock-hash needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichScenario {
    pub name: String,
    pub description: Option<String>,
    pub given: Vec<String>,
    pub when_: Vec<String>,
    pub then_: Vec<String>,
    pub req_ids: Vec<String>,
    pub tags: Vec<String>,
    pub tier: Option<ScenarioTier>,
}

impl RichScenario {
    pub fn has_tag(&self, tag: &str) -> bool {
        let want = tag.trim_start_matches('@');
        self.tags
            .iter()
            .any(|t| t.trim().trim_start_matches('@').eq_ignore_ascii_case(want))
    }
}

/// Fully parsed single-track spec file.
#[derive(Debug, Clone)]
pub struct ParsedFeatureSpec {
    pub name: String,
    pub purpose: String,
    pub valid_scope: Vec<String>,
    pub feature_title: String,
    pub scenarios: Vec<RichScenario>,
    /// Raw file content (for lock hashing continuity checks).
    pub raw: String,
}

impl ParsedFeatureSpec {
    /// Constraint scenarios (`@human`), including manual waivers.
    pub fn rule_scenarios(&self) -> impl Iterator<Item = &RichScenario> {
        self.scenarios
            .iter()
            .filter(|sc| sc.tier == Some(ScenarioTier::Constraint))
    }

    /// Acceptance scenarios (`@executable`).
    pub fn acceptance_scenarios(&self) -> impl Iterator<Item = &RichScenario> {
        self.scenarios
            .iter()
            .filter(|sc| sc.tier == Some(ScenarioTier::Acceptance))
    }
}

impl FeatureBackend {
    /// Parse single-track feature content with explicit Gherkin language.
    pub fn parse_content(&self, content: &str, context: &str) -> Result<ParsedFeatureSpec> {
        let (headers, _body_offset) = parse_header_comments(content);
        let name = headers.capability.ok_or_else(|| {
            anyhow!("{context}: missing `# capability:` header comment (spec-format r133)")
        })?;
        let purpose = headers.purpose.unwrap_or_default();
        let valid_scope: Vec<String> = headers
            .scope
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let lang = detect_language(content);
        let env = gherkin::GherkinEnv::new(&lang)
            .map_err(|err| anyhow!("{context}: build gherkin env for `{lang}`: {err}"))?;
        let parsed = gherkin::Feature::parse(content, env)
            .map_err(|err| anyhow!("{context}: failed to parse Gherkin: {err}"))?;

        // Rule-blocked scenarios would be silently skipped by the runner macros.
        for rule in &parsed.rules {
            if !rule.scenarios.is_empty() {
                return Err(anyhow!(
                    "{context}: scenarios inside Gherkin `Rule:` blocks are not supported \
                     (rstest-bdd skips them silently); keep all scenarios top-level"
                ));
            }
        }

        let mut scenarios = Vec::new();
        for sc in &parsed.scenarios {
            let mut given = Vec::new();
            let mut when_ = Vec::new();
            let mut then_ = Vec::new();
            for step in &sc.steps {
                match step.ty {
                    gherkin::StepType::Given => given.push(step.value.clone()),
                    gherkin::StepType::When => when_.push(step.value.clone()),
                    gherkin::StepType::Then => then_.push(step.value.clone()),
                }
            }
            let tags = sc.tags.clone();
            let tier = classify_tier(&tags).map_err(|msg| anyhow!("{context}: {msg}"))?;
            scenarios.push(RichScenario {
                name: sc.name.clone(),
                description: sc.description.clone(),
                given,
                when_,
                then_,
                req_ids: req_ids_from_tags(&tags),
                tags,
                tier,
            });
        }

        Ok(ParsedFeatureSpec {
            name,
            purpose,
            valid_scope,
            feature_title: parsed.name,
            scenarios,
            raw: content.to_string(),
        })
    }
}

struct HeaderComments {
    capability: Option<String>,
    purpose: Option<String>,
    scope: Option<String>,
}

/// Split leading `# key: value` comment headers from the Gherkin body.
/// Returns the headers and the byte offset where the body starts (unused by
/// callers today; the Gherkin parser tolerates the comment block).
fn parse_header_comments(content: &str) -> (HeaderComments, usize) {
    let mut capability = None;
    let mut purpose = None;
    let mut scope = None;
    let mut offset = 0usize;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("# language:") {
            offset += line.len() + 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('#') {
            let rest = rest.trim();
            if let Some(v) = rest.strip_prefix("capability:") {
                capability = Some(v.trim().to_string());
            } else if let Some(v) = rest.strip_prefix("purpose:") {
                purpose = Some(v.trim().to_string());
            } else if let Some(v) = rest.strip_prefix("scope:") {
                scope = Some(v.trim().to_string());
            }
            offset += line.len() + 1;
            continue;
        }
        break;
    }
    (
        HeaderComments {
            capability,
            purpose,
            scope,
        },
        offset,
    )
}

fn detect_language(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# language:") {
            return rest.trim().to_string();
        }
        break;
    }
    "en".to_string()
}

/// Classify a scenario tier from its tags; errors on reserved-tag misuse (r132).
fn classify_tier(tags: &[String]) -> Result<Option<ScenarioTier>> {
    let has = |want: &str| {
        tags.iter().any(|t| {
            t.trim()
                .trim_start_matches('@')
                .eq_ignore_ascii_case(want.trim_start_matches('@'))
        })
    };
    let human = has("human");
    let executable = has("executable");
    let manual = has("manual");
    if human && executable {
        return Err(anyhow!(
            "scenario cannot be both @human (constraint) and @executable (acceptance)"
        ));
    }
    if manual && !human {
        return Err(anyhow!(
            "@manual requires @human (manual review waives a constraint rule)"
        ));
    }
    if human && manual {
        Ok(Some(ScenarioTier::Manual))
    } else if human {
        Ok(Some(ScenarioTier::Constraint))
    } else if executable {
        Ok(Some(ScenarioTier::Acceptance))
    } else {
        Ok(None)
    }
}

/// Extract `@req:<id>` tags (deduplicated, order-preserving).
pub fn req_ids_from_tags(tags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        let t = tag.trim().trim_start_matches('@');
        if let Some(rest) = t.strip_prefix("req:") {
            let id = rest.trim();
            if !id.is_empty() && !out.iter().any(|x| x == id) {
                out.push(id.to_string());
            }
        }
    }
    out
}

/// Normalized lock-hash input lines for one scenario (design D4):
/// id(name), description, and each step prefixed by its type, all
/// right-trimmed. Whitespace inside lines is preserved verbatim.
pub fn normalized_hash_lines(sc: &RichScenario) -> Vec<String> {
    let mut lines = vec![format!("scenario: {}", sc.name)];
    if let Some(desc) = sc.description.as_deref() {
        for l in desc.lines() {
            lines.push(format!("desc: {}", l.trim_end()));
        }
    }
    for (label, steps) in [
        ("given", &sc.given),
        ("when", &sc.when_),
        ("then", &sc.then_),
    ] {
        for s in steps {
            lines.push(format!("{label}: {}", s.trim_end()));
        }
    }
    lines
}

/// SHA-256 hex of the normalized lock-hash lines (design D4).
pub fn lock_hash(sc: &RichScenario) -> String {
    let mut hasher = Sha256::new();
    for line in normalized_hash_lines(sc) {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Statement for the IR requirement row: description verbatim, or synthesized
/// from steps when the author relied purely on Given/When/Then decomposition.
pub fn rule_statement(sc: &RichScenario) -> String {
    if let Some(desc) = sc.description.as_deref()
        && !desc.trim().is_empty()
    {
        return desc.trim().to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    if !sc.given.is_empty() {
        parts.push(format!("假如 {}", sc.given.join("；")));
    }
    if !sc.when_.is_empty() {
        parts.push(format!("当 {}", sc.when_.join("；")));
    }
    if !sc.then_.is_empty() {
        parts.push(format!("那么 {}", sc.then_.join("；")));
    }
    parts.join("；")
}

impl SpecBackend for FeatureBackend {
    fn parse_main_spec(&self, content: &str, context: &str) -> Result<MainSpecDoc> {
        let parsed = self.parse_content(content, context)?;
        Ok(parsed_to_doc(&parsed))
    }

    fn parse_main_spec_strict(&self, content: &str, context: &str) -> Result<MainSpecDoc> {
        // Strict mode adds nothing beyond the base grammar today: header
        // completeness and tag grammar already fail hard in `parse_content`.
        self.parse_main_spec(content, context)
    }

    fn parse_delta_spec(&self, _content: &str, context: &str) -> Result<DeltaSpecDoc> {
        Err(anyhow!(
            "{context}: delta specs were removed by the single-track format"
        ))
    }

    fn parse_delta_spec_strict(&self, content: &str, context: &str) -> Result<DeltaSpecDoc> {
        self.parse_delta_spec(content, context)
    }

    /// Deterministically render a main spec back to single-track gherkin
    /// (canonical form: zh-CN keywords, requirements as `@human` rules).
    fn dump_main_spec(&self, doc: &MainSpecDoc) -> Result<String> {
        let mut out = String::new();
        let _ = writeln!(out, "# language: zh-CN");
        let _ = writeln!(out, "# capability: {}", doc.name.trim());
        let _ = writeln!(out, "# purpose: {}", doc.purpose.trim());
        if !doc.valid_scope.is_empty() {
            let _ = writeln!(out, "# scope: {}", doc.valid_scope.join(", "));
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "功能: {}", doc.name.trim());

        for req in &doc.requirements {
            let _ = writeln!(out);
            let _ = writeln!(out, "  @req:{} @human", req.req_id);
            let _ = writeln!(out, "  场景: {}", req.title);
            for line in req.statement.lines() {
                let _ = writeln!(out, "    {line}");
            }
            if req.statement.is_empty() {
                let _ = writeln!(out, "    （约束陈述待补充）");
            }
        }
        for sc in &doc.scenarios {
            if !sc.feature {
                continue;
            }
            let _ = writeln!(out);
            let _ = writeln!(out, "  @req:{} @executable", sc.req_id);
            let _ = writeln!(out, "  场景: {}", sc.id);
            for (kw, field) in [("假如", &sc.given), ("当", &sc.when_), ("那么", &sc.then_)] {
                if !field.trim().is_empty() {
                    for line in field.lines() {
                        let _ = writeln!(out, "    {kw} {line}");
                    }
                }
            }
        }
        Ok(out)
    }

    fn dump_delta_spec(&self, _doc: &DeltaSpecDoc) -> Result<String> {
        Err(anyhow!(
            "delta specs were removed by the single-track format"
        ))
    }
}

/// Project a parsed single-track spec onto the stable IR.
///
/// - `@human` scenarios become `requirements[]` rows (statement from
///   description or synthesized from steps).
/// - Only `@executable` scenarios land in `scenarios[]` (`feature: true`).
pub fn parsed_to_doc(parsed: &ParsedFeatureSpec) -> MainSpecDoc {
    let requirements = parsed
        .rule_scenarios()
        .filter_map(|sc| {
            sc.req_ids.first().map(|rid| RequirementEntry {
                req_id: rid.clone(),
                title: sc.name.clone(),
                statement: rule_statement(sc),
            })
        })
        .collect();
    let scenarios = parsed
        .acceptance_scenarios()
        .map(|sc| ScenarioEntry {
            req_id: sc.req_ids.first().cloned().unwrap_or_default(),
            id: sc.name.clone(),
            given: sc.given.join("\n"),
            when_: sc.when_.join("\n"),
            then_: sc.then_.join("\n"),
            feature: true,
        })
        .collect();
    MainSpecDoc {
        kind: "llman.sdd.spec".to_string(),
        name: parsed.name.clone(),
        purpose: parsed.purpose.clone(),
        valid_scope: parsed.valid_scope.clone(),
        requirements,
        scenarios,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdd::spec::backend::FEATURE_BACKEND;

    const ZH_SAMPLE: &str = "\
# language: zh-CN
# capability: sample-cap
# purpose: 单轨格式样例
# scope: src/a, src/b

功能: 样例能力

  @req:r1 @human
  场景: start 门禁
    工作区在默认分支且有未提交变更时
    系统 MUST 拒绝 change start。

  @req:r1 @executable
  场景: dirty-tree-start-rejected
    假如 已初始化 sdd 项目且 bdd 配置为 \"on\"
    当 运行 llman sdd change start x
    那么 退出码非零

  场景: 无标签的普通场景
    假如 前置
    当 动作
    那么 结果
";

    #[test]
    fn parses_zh_sample_headers_and_tiers() {
        let parsed = FEATURE_BACKEND.parse_content(ZH_SAMPLE, "test").unwrap();
        assert_eq!(parsed.name, "sample-cap");
        assert_eq!(parsed.purpose, "单轨格式样例");
        assert_eq!(parsed.valid_scope, vec!["src/a", "src/b"]);
        assert_eq!(parsed.scenarios.len(), 3);

        let rules: Vec<_> = parsed.rule_scenarios().collect();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "start 门禁");
        assert_eq!(rules[0].req_ids, vec!["r1"]);
        assert!(rules[0].description.as_deref().unwrap().contains("MUST"));

        let acc: Vec<_> = parsed.acceptance_scenarios().collect();
        assert_eq!(acc.len(), 1);
        assert_eq!(acc[0].name, "dirty-tree-start-rejected");

        // IR projection: requirement + executable scenario only.
        let doc = FEATURE_BACKEND.parse_main_spec(ZH_SAMPLE, "test").unwrap();
        assert_eq!(doc.requirements.len(), 1);
        assert_eq!(doc.requirements[0].req_id, "r1");
        assert_eq!(doc.requirements[0].title, "start 门禁");
        assert!(doc.requirements[0].statement.contains("MUST"));
        assert_eq!(doc.scenarios.len(), 1);
        assert!(doc.scenarios[0].feature);
        assert_eq!(doc.scenarios[0].id, "dirty-tree-start-rejected");
    }

    #[test]
    fn missing_capability_header_fails() {
        let content = "# purpose: x\n\n功能: t\n  场景: s\n    假如 a\n";
        let err = FEATURE_BACKEND
            .parse_content(content, "ctx")
            .unwrap_err()
            .to_string();
        assert!(err.contains("# capability:"), "got: {err}");
    }

    #[test]
    fn human_and_executable_are_mutually_exclusive() {
        let content = "\
# language: zh-CN
# capability: cap
# purpose: p
# scope: src

功能: t
  @req:r9 @human @executable
  场景: bad
    假如 a
";
        let err = FEATURE_BACKEND
            .parse_content(content, "ctx")
            .unwrap_err()
            .to_string();
        assert!(err.contains("@human"), "got: {err}");
    }

    #[test]
    fn rule_block_scenarios_are_rejected() {
        // Note: the official zh-CN Gherkin dictionary keeps `Rule` in English
        // (`规则` is NOT a keyword and parses as description text).
        let content = "\
# language: zh-CN
# capability: cap
# purpose: p
# scope: src

功能: t
  Rule: 分组
    @executable
    场景: nested
      假如 a
";
        let err = FEATURE_BACKEND
            .parse_content(content, "ctx")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Rule") || err.contains("规则"),
            "should reject Rule-blocked scenarios, got: {err}"
        );
    }

    #[test]
    fn lock_hash_is_stable_and_whitespace_tolerant() {
        let a = FEATURE_BACKEND.parse_content(ZH_SAMPLE, "t").unwrap();
        let mut tweaked = ZH_SAMPLE.replace("那么 退出码非零", "那么 退出码非零   ");
        tweaked.push('\n');
        let b = FEATURE_BACKEND.parse_content(&tweaked, "t").unwrap();

        let ha = lock_hash(&a.scenarios[1]);
        let hb = lock_hash(&b.scenarios[1]);
        assert_eq!(ha, hb, "trailing whitespace must not change the hash");

        let mutated = ZH_SAMPLE.replace("dirty-tree-start-rejected", "dirty-tree-start-blocked");
        let c = FEATURE_BACKEND.parse_content(&mutated, "t").unwrap();
        assert_ne!(
            ha,
            lock_hash(&c.scenarios[1]),
            "renames must change the hash"
        );
        assert_eq!(ha.len(), 64, "sha256 hex length");
    }

    #[test]
    fn manual_waiver_tier() {
        let content = "\
# capability: cap
# purpose: p
# scope: src

Feature: t
  @req:r2 @human @manual
  Scenario: waived rule
    Style consistency is reviewed by humans only.
";
        let parsed = FEATURE_BACKEND.parse_content(content, "t").unwrap();
        assert_eq!(
            parsed.scenarios[0].tier,
            Some(ScenarioTier::Manual),
            "@manual requires @human"
        );
    }

    #[test]
    fn dump_roundtrips_through_parse() {
        let doc = FEATURE_BACKEND.parse_main_spec(ZH_SAMPLE, "t").unwrap();
        let dumped = FEATURE_BACKEND.dump_main_spec(&doc).unwrap();
        let reparsed = FEATURE_BACKEND
            .parse_main_spec(&dumped, "roundtrip")
            .unwrap();
        assert_eq!(doc.name, reparsed.name);
        assert_eq!(doc.requirements, reparsed.requirements);
        assert_eq!(doc.scenarios, reparsed.scenarios);
    }
}
