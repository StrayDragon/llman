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

use crate::sdd::spec::ir::{MainSpecDoc, RequirementEntry, ScenarioEntry};
use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

pub(crate) struct FeatureBackend;

/// Process-wide singleton (single-track spec-format r131).
pub(crate) static FEATURE_BACKEND: FeatureBackend = FeatureBackend;

/// Tier of a scenario under the single-track grammar (r132).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScenarioTier {
    /// Human-owned constraint (`@human`); never runner-bound.
    Constraint,
    /// Runner-bound acceptance (`@executable`).
    Acceptance,
    /// Explicit manual-review waiver (`@human @manual`).
    Manual,
}

/// Richly parsed scenario retaining everything the lock-hash needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RichScenario {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) given: Vec<String>,
    pub(crate) when_: Vec<String>,
    pub(crate) then_: Vec<String>,
    pub(crate) req_ids: Vec<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) tier: Option<ScenarioTier>,
}

/// Gherkin keyword set for a supported language.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GherkinKw {
    pub(crate) feature: &'static str,
    pub(crate) scenario: &'static str,
    pub(crate) given: &'static str,
    pub(crate) when: &'static str,
    pub(crate) then: &'static str,
}

/// Keywords for the parsed/rendered language (zh-CN or English).
pub(crate) fn keywords_for(lang: &str) -> GherkinKw {
    if lang.starts_with("zh") {
        GherkinKw {
            feature: "功能",
            scenario: "场景",
            given: "假如",
            when: "当",
            then: "那么",
        }
    } else {
        GherkinKw {
            feature: "Feature",
            scenario: "Scenario",
            given: "Given",
            when: "When",
            then: "Then",
        }
    }
}

impl ScenarioTier {
    /// Whether scenarios of this tier are locked for agent edits (r135).
    pub(crate) fn is_locked(self) -> bool {
        matches!(self, ScenarioTier::Constraint | ScenarioTier::Manual)
    }
}

/// Fully parsed single-track spec file.
#[derive(Debug, Clone)]
pub(crate) struct ParsedFeatureSpec {
    pub(crate) name: String,
    pub(crate) purpose: String,
    pub(crate) valid_scope: Vec<String>,
    pub(crate) feature_title: String,
    pub(crate) scenarios: Vec<RichScenario>,
}

impl ParsedFeatureSpec {
    /// Locked-rule scenarios (`@human`), including `@manual` waivers.
    pub(crate) fn rule_scenarios(&self) -> impl Iterator<Item = &RichScenario> {
        self.scenarios.iter().filter(|sc| {
            matches!(
                sc.tier,
                Some(ScenarioTier::Constraint | ScenarioTier::Manual)
            )
        })
    }
}

impl ParsedFeatureSpec {
    /// Acceptance scenarios (`@executable`).
    pub(crate) fn acceptance_scenarios(&self) -> impl Iterator<Item = &RichScenario> {
        self.scenarios
            .iter()
            .filter(|sc| sc.tier == Some(ScenarioTier::Acceptance))
    }
}

impl FeatureBackend {
    /// Parse single-track feature content with explicit Gherkin language.
    pub(crate) fn parse_content(&self, content: &str, context: &str) -> Result<ParsedFeatureSpec> {
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

        // Language resolution: explicit `# language:` header wins; otherwise
        // try English then fall back to Chinese keywords (most llman-authored
        // specs are zh-CN even when the header is omitted).
        let header_lang = detect_language(content);
        let mut candidates: Vec<String> = vec![header_lang.clone(), "zh-CN".to_string()];
        candidates.dedup();
        let mut parsed: Option<(gherkin::Feature, String)> = None;
        let mut last_err = String::new();
        for lang in &candidates {
            match gherkin::GherkinEnv::new(lang)
                .map_err(|err| anyhow!("{context}: gherkin env `{lang}`: {err}"))
                .and_then(|env| {
                    gherkin::Feature::parse(content, env)
                        .map_err(|err| anyhow!("{context}: failed to parse Gherkin: {err}"))
                }) {
                Ok(feature) => {
                    parsed = Some((feature, lang.clone()));
                    break;
                }
                Err(err) => last_err = err.to_string(),
            }
        }
        let Some((parsed_feature, _lang)) = parsed else {
            return Err(anyhow!("{last_err}"));
        };
        let parsed = parsed_feature;

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

pub(crate) fn detect_language(content: &str) -> String {
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
pub(crate) fn req_ids_from_tags(tags: &[String]) -> Vec<String> {
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
pub(crate) fn normalized_hash_lines(sc: &RichScenario) -> Vec<String> {
    let mut lines = vec![format!("scenario: {}", sc.name)];
    for rid in &sc.req_ids {
        lines.push(format!("req: {rid}"));
    }
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
pub(crate) fn lock_hash(sc: &RichScenario) -> String {
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
pub(crate) fn rule_statement(sc: &RichScenario) -> String {
    if let Some(desc) = sc.description.as_deref()
        && !desc.trim().is_empty()
    {
        // Strip the renderer's `- ` bullet prefixes so dump→parse round-trips.
        let stripped: Vec<&str> = desc
            .lines()
            .map(|l| l.trim().strip_prefix("- ").unwrap_or(l.trim()))
            .collect();
        return stripped.join("\n");
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

impl FeatureBackend {
    pub(crate) fn parse_main_spec(&self, content: &str, context: &str) -> Result<MainSpecDoc> {
        let parsed = self.parse_content(content, context)?;
        Ok(parsed_to_doc(&parsed))
    }

    /// Deterministically render a main spec back to single-track gherkin
    /// (canonical form: zh-CN keywords, requirements as `@human` rules).
    pub(crate) fn dump_main_spec(&self, doc: &MainSpecDoc) -> Result<String> {
        self.dump_main_spec_lang(doc, "zh-CN")
    }

    /// Language-aware variant of [`FeatureBackend::dump_main_spec`]: keywords
    /// and the `# language:` header follow `lang` (see [`keywords_for`]).
    pub(crate) fn dump_main_spec_lang(&self, doc: &MainSpecDoc, lang: &str) -> Result<String> {
        let kw = keywords_for(lang);
        let mut out = String::new();
        let _ = writeln!(out, "# language: {lang}");
        let _ = writeln!(out, "# capability: {}", doc.name.trim());
        let _ = writeln!(out, "# purpose: {}", doc.purpose.trim());
        if !doc.valid_scope.is_empty() {
            let _ = writeln!(out, "# scope: {}", doc.valid_scope.join(", "));
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "{}: {}", kw.feature, doc.name.trim());

        for req in &doc.requirements {
            let _ = writeln!(out);
            let _ = writeln!(out, "  @req:{} @human", req.req_id);
            let _ = writeln!(out, "  {}: {}", kw.scenario, req.title);
            // Bullet-prefix each statement line: free text starting with a
            // Gherkin keyword (e.g. a line beginning with `场景`/`当`) would
            // otherwise be parsed as structure and break the file.
            let statement = if req.statement.is_empty() {
                "（约束陈述待补充）"
            } else {
                req.statement.as_str()
            };
            for line in statement.lines() {
                let _ = writeln!(out, "    - {line}");
            }
        }
        for sc in &doc.scenarios {
            if !sc.feature {
                continue;
            }
            let _ = writeln!(out);
            let _ = writeln!(out, "  @req:{} @executable", sc.req_id);
            let _ = writeln!(out, "  {}: {}", kw.scenario, sc.id);
            // Collapse multi-line step values to one physical line: bare
            // continuation lines would re-parse as bogus steps.
            for (kw_str, field) in [
                (kw.given, &sc.given),
                (kw.when, &sc.when_),
                (kw.then, &sc.then_),
            ] {
                let value = field
                    .split('\n')
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !value.is_empty() {
                    let _ = writeln!(out, "    {kw_str} {value}");
                }
            }
        }
        Ok(out)
    }
}

/// Rule-tier morphology for `list --specs` / `show` (spec-format r134).
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuleMorphology {
    pub(crate) rule_count: usize,
    pub(crate) rule_enforced_count: usize,
    pub(crate) rule_manual_count: usize,
    pub(crate) rule_pending_count: usize,
    pub(crate) acceptance_count: usize,
    pub(crate) orphan_acceptance_count: usize,
}

/// Map rule id -> number of `@executable` scenarios linked to it.
pub(crate) fn acceptance_index(
    parsed: &ParsedFeatureSpec,
) -> std::collections::HashMap<String, usize> {
    let mut idx: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for acc in parsed.acceptance_scenarios() {
        for rid in &acc.req_ids {
            *idx.entry(rid.clone()).or_insert(0) += 1;
        }
    }
    idx
}

/// Compute the three-tier coverage counts from a parsed spec.
pub(crate) fn compute_rule_morphology(parsed: &ParsedFeatureSpec) -> RuleMorphology {
    let rules: Vec<&RichScenario> = parsed.rule_scenarios().collect();
    let rule_count = rules.len();
    let mut enforced = 0usize;
    let mut manual = 0usize;
    let mut pending = 0usize;
    let idx = acceptance_index(parsed);
    for sc in &rules {
        let has_acceptance = sc
            .req_ids
            .iter()
            .any(|r| idx.get(r).copied().unwrap_or(0) > 0);
        match (sc.tier, has_acceptance) {
            (Some(ScenarioTier::Manual), _) => manual += 1,
            (_, true) => enforced += 1,
            _ => pending += 1,
        }
    }
    let orphan = parsed
        .acceptance_scenarios()
        .filter(|acc| acc.req_ids.is_empty())
        .count();
    RuleMorphology {
        rule_count,
        rule_enforced_count: enforced,
        rule_manual_count: manual,
        rule_pending_count: pending,
        acceptance_count: parsed.acceptance_scenarios().count(),
        orphan_acceptance_count: orphan,
    }
}

/// Project a parsed single-track spec onto the stable IR.///
/// - `@human` scenarios become `requirements[]` rows (statement from
///   description or synthesized from steps).
/// - Only `@executable` scenarios land in `scenarios[]` (`feature: true`).
pub(crate) fn parsed_to_doc(parsed: &ParsedFeatureSpec) -> MainSpecDoc {
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

    #[test]
    fn dump_main_spec_lang_renders_keyword_sets() {
        let doc = FEATURE_BACKEND.parse_main_spec(ZH_SAMPLE, "t").unwrap();
        let zh = FEATURE_BACKEND.dump_main_spec_lang(&doc, "zh-CN").unwrap();
        assert!(zh.contains("# language: zh-CN"));
        assert!(zh.contains("功能: "));
        assert!(zh.contains("场景: "));
        assert!(zh.contains("假如 "));
        // dump_main_spec stays the zh-CN canonical form.
        assert_eq!(FEATURE_BACKEND.dump_main_spec(&doc).unwrap(), zh);

        let en = FEATURE_BACKEND.dump_main_spec_lang(&doc, "en").unwrap();
        assert!(en.contains("# language: en"));
        assert!(en.contains("Feature: "));
        assert!(en.contains("Scenario: "));
        assert!(en.contains("Given "));
        // English output must re-parse to the same IR.
        let reparsed = FEATURE_BACKEND.parse_main_spec(&en, "en").unwrap();
        assert_eq!(reparsed.requirements, doc.requirements);
        assert_eq!(reparsed.scenarios, doc.scenarios);
    }
}
