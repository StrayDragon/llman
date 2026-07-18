//! Partitioned SSOT helpers: harness (`.feature`) + constraints (`spec.toon`).
//!
//! See `llmanspec/changes/add-sdd-bdd-partitioned-ssot/design.md`.

use crate::sdd::spec::ir::{MainSpecDoc, ScenarioEntry};
use crate::sdd::spec::validation::{ValidationIssue, ValidationLevel, discover_features};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Parsed harness scenario from a `.feature` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureScenario {
    pub id: String,
    pub given: String,
    pub when_: String,
    pub then_: String,
    /// req_ids extracted from `@req:<id>` tags (may be empty).
    pub req_ids: Vec<String>,
    pub tags: Vec<String>,
}

/// Spec morphology for list/show JSON.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Morphology {
    pub constraints_req_count: usize,
    pub non_executable_scenario_count: usize,
    pub harness_scenario_count: usize,
    pub req_link_coverage: f64,
    pub dual_write_count: usize,
}

/// Extract `@req:<id>` (and bare `req:<id>`) from Gherkin tags.
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

/// Parse a `.feature` file into harness scenarios (with `@req` tags).
pub fn parse_feature_scenarios(path: &Path, lang: &str) -> Result<Vec<FeatureScenario>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("read feature {}", path.display()))?;
    parse_feature_scenarios_content(&content, lang)
        .with_context(|| format!("parse feature {}", path.display()))
}

pub fn parse_feature_scenarios_content(content: &str, lang: &str) -> Result<Vec<FeatureScenario>> {
    let env = gherkin::GherkinEnv::new(lang)
        .with_context(|| format!("build gherkin env for language `{lang}`"))?;
    let parsed = gherkin::Feature::parse(content, env)?;
    let mut out = Vec::new();
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
        let req_ids = req_ids_from_tags(&sc.tags);
        out.push(FeatureScenario {
            id: sc.name.clone(),
            given: given.join("\n"),
            when_: when_.join("\n"),
            then_: then_.join("\n"),
            req_ids,
            tags: sc.tags.clone(),
        });
    }
    Ok(out)
}

/// Load all harness scenarios under a spec directory.
pub fn load_spec_harness(spec_dir: &Path, lang: &str) -> Result<Vec<FeatureScenario>> {
    let mut all = Vec::new();
    for path in discover_features(spec_dir) {
        match parse_feature_scenarios(&path, lang) {
            Ok(mut scs) => all.append(&mut scs),
            Err(e) => {
                // Caller may prefer soft-fail; propagate for migrate/apply.
                return Err(e).with_context(|| format!("load harness {}", path.display()));
            }
        }
    }
    Ok(all)
}

/// Parse a `.feature` file into spec-level
/// [`ScenarioNode`](crate::sdd::context::tree::ScenarioNode)s.
///
/// `req_id` is taken from the first `@req:<id>` tag when present; otherwise empty
/// (spec-level). Used by pageindex rebuild under Partitioned SSOT (feature wins).
pub fn parse_feature_file(
    path: &Path,
    lang: &str,
) -> Result<Vec<crate::sdd::context::tree::ScenarioNode>> {
    use crate::sdd::context::tree::ScenarioNode;
    let scenarios = parse_feature_scenarios(path, lang)?;
    Ok(scenarios
        .into_iter()
        .map(|sc| ScenarioNode {
            req_id: sc.req_ids.first().cloned().unwrap_or_default(),
            id: sc.id,
            given: sc.given,
            when_: sc.when_,
            then_: sc.then_,
        })
        .collect())
}

/// Soft-load harness: skip malformed files with a warning issue instead of failing.
pub fn load_spec_harness_soft(
    spec_dir: &Path,
    lang: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<FeatureScenario> {
    let mut all = Vec::new();
    for path in discover_features(spec_dir) {
        match parse_feature_scenarios(&path, lang) {
            Ok(mut scs) => all.append(&mut scs),
            Err(e) => {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    path: path.display().to_string(),
                    message: format!("failed to parse feature for partitioned checks: {e}"),
                });
            }
        }
    }
    all
}

fn gwt_nonempty(given: &str, when_: &str, then_: &str) -> bool {
    !given.trim().is_empty() || !when_.trim().is_empty() || !then_.trim().is_empty()
}

/// Executable toon scenarios (feature == true).
pub fn executable_toon_scenarios(doc: &MainSpecDoc) -> Vec<&ScenarioEntry> {
    doc.scenarios.iter().filter(|s| s.feature).collect()
}

pub fn non_executable_toon_scenarios(doc: &MainSpecDoc) -> Vec<&ScenarioEntry> {
    doc.scenarios.iter().filter(|s| !s.feature).collect()
}

/// Count dual-writes: same scenario id present as executable in toon and in harness
/// with non-empty GWT on both sides.
pub fn dual_write_count(doc: &MainSpecDoc, harness: &[FeatureScenario]) -> usize {
    dual_write_pairs(doc, harness).len()
}

/// Return each dual-write conflict as `(req_id, scenario_id)`. A dual-write exists
/// when an executable toon scenario (`feature == true`) carries non-empty GWT and
/// its id also appears in the harness. Listing concrete pairs lets agents locate
/// the offending rows without a second pass.
pub fn dual_write_pairs(doc: &MainSpecDoc, harness: &[FeatureScenario]) -> Vec<(String, String)> {
    let harness_ids: HashSet<&str> = harness.iter().map(|h| h.id.as_str()).collect();
    executable_toon_scenarios(doc)
        .into_iter()
        .filter(|s| {
            harness_ids.contains(s.id.as_str()) && gwt_nonempty(&s.given, &s.when_, &s.then_)
        })
        .map(|s| (s.req_id.clone(), s.id.clone()))
        .collect()
}

pub fn compute_morphology(doc: &MainSpecDoc, harness: &[FeatureScenario]) -> Morphology {
    let constraints_req_count = doc.requirements.len();
    let non_executable_scenario_count = non_executable_toon_scenarios(doc).len();
    let harness_scenario_count = harness.len();
    let dual = dual_write_count(doc, harness);

    let req_set: HashSet<&str> = doc.requirements.iter().map(|r| r.req_id.as_str()).collect();
    let linked: HashSet<&str> = harness
        .iter()
        .flat_map(|h| h.req_ids.iter().map(|s| s.as_str()))
        .filter(|id| req_set.contains(id))
        .collect();
    let req_link_coverage = if constraints_req_count == 0 {
        if harness_scenario_count == 0 {
            1.0
        } else {
            0.0
        }
    } else {
        linked.len() as f64 / constraints_req_count as f64
    };

    Morphology {
        constraints_req_count,
        non_executable_scenario_count,
        harness_scenario_count,
        req_link_coverage,
        dual_write_count: dual,
    }
}

/// Partitioned validate checks for one spec. Emits ERROR/WARNING issues.
pub fn validate_partitioned(
    spec_name: &str,
    doc: &MainSpecDoc,
    harness: &[FeatureScenario],
    strict: bool,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let req_ids: HashSet<&str> = doc.requirements.iter().map(|r| r.req_id.as_str()).collect();

    // @req links
    for sc in harness {
        if sc.req_ids.is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: format!("{spec_name}/harness/{}", sc.id),
                message: format!(
                    "harness scenario `{}` has no @req:<req_id> tag (Partitioned SSOT)",
                    sc.id
                ),
            });
            continue;
        }
        for rid in &sc.req_ids {
            if !req_ids.contains(rid.as_str()) {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    path: format!("{spec_name}/harness/{}/@req", sc.id),
                    message: format!(
                        "@req:{rid} on scenario `{}` has no matching requirement in spec.toon",
                        sc.id
                    ),
                });
            }
        }
    }

    // Dual-write
    let pairs = dual_write_pairs(doc, harness);
    let dual = pairs.len();
    if dual > 0 {
        let level = if strict {
            ValidationLevel::Error
        } else {
            ValidationLevel::Warning
        };
        let pairs_formatted = pairs
            .iter()
            .map(|(rid, sid)| format!("({rid}, {sid})"))
            .collect::<Vec<_>>()
            .join(", ");
        issues.push(ValidationIssue {
            level,
            path: format!("{spec_name}/dual-write"),
            message: format!(
                "dual-write: {dual} executable scenario(s) still have GWT in both spec.toon and .feature: [{pairs_formatted}]; run `llman sdd project migrate --kind partitioned`"
            ),
        });
    }

    // Non-executable toon ids must not appear in harness
    let harness_ids: HashSet<&str> = harness.iter().map(|h| h.id.as_str()).collect();
    for sc in non_executable_toon_scenarios(doc) {
        if harness_ids.contains(sc.id.as_str()) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{spec_name}/scenarios/{}", sc.id),
                message: format!(
                    "non-executable toon scenario `{}` must not appear in .feature",
                    sc.id
                ),
            });
        }
    }

    issues
}

/// Strip executable GWT from toon (keep feature:false only); return removed rows for migrate.
pub fn split_executable_from_toon(doc: &mut MainSpecDoc) -> Vec<ScenarioEntry> {
    let (keep, removed): (Vec<_>, Vec<_>) = doc.scenarios.drain(..).partition(|s| !s.feature);
    doc.scenarios = keep;
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_req_tags() {
        let tags = vec!["@req:ar1".into(), "smoke".into(), "req:ar2".into()];
        assert_eq!(req_ids_from_tags(&tags), vec!["ar1", "ar2"]);
    }

    #[test]
    fn parse_feature_with_req() {
        let body = "# language: zh-CN\n功能: demo\n  @req:r1\n  场景: happy\n    假如 a\n    当 b\n    那么 c\n";
        let scs = parse_feature_scenarios_content(body, "zh-CN").unwrap();
        assert_eq!(scs.len(), 1);
        assert_eq!(scs[0].id, "happy");
        assert_eq!(scs[0].req_ids, vec!["r1"]);
    }

    fn scenario(req_id: &str, id: &str, feature: bool, gwt_nonempty: bool) -> ScenarioEntry {
        let text = if gwt_nonempty { "g" } else { "" };
        ScenarioEntry {
            req_id: req_id.to_string(),
            id: id.to_string(),
            given: text.to_string(),
            when_: text.to_string(),
            then_: text.to_string(),
            feature,
        }
    }

    fn harness(id: &str, req_id: &str) -> FeatureScenario {
        FeatureScenario {
            id: id.to_string(),
            given: String::new(),
            when_: String::new(),
            then_: String::new(),
            req_ids: vec![req_id.to_string()],
            tags: vec![],
        }
    }

    fn doc_with(scenarios: Vec<ScenarioEntry>) -> MainSpecDoc {
        MainSpecDoc {
            kind: "llman.sdd.spec".to_string(),
            name: "t".to_string(),
            purpose: "t".to_string(),
            valid_scope: vec![],
            requirements: vec![],
            scenarios,
        }
    }

    #[test]
    fn dual_write_pairs_lists_conflicts() {
        // Two executable toon scenarios with GWT also exist in harness → both reported.
        let doc = doc_with(vec![
            scenario("r1", "happy", true, true),
            scenario("r12", "login-ok", true, true),
            // Non-executable (feature:false) → never dual-write even if id matches.
            scenario("r3", "doc-only", false, true),
            // Executable but not in harness → not dual-write.
            scenario("r4", "only-toon", true, true),
            // Executable, in harness, but GWT empty → not dual-write.
            scenario("r5", "empty-gwt", true, false),
        ]);
        let harness_scenarios = vec![
            harness("happy", "r1"),
            harness("login-ok", "r12"),
            harness("doc-only", "r3"),
            harness("empty-gwt", "r5"),
        ];
        let mut pairs = dual_write_pairs(&doc, &harness_scenarios);
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                ("r1".to_string(), "happy".to_string()),
                ("r12".to_string(), "login-ok".to_string())
            ]
        );
        assert_eq!(dual_write_count(&doc, &harness_scenarios), 2);
    }

    #[test]
    fn dual_write_pairs_empty_when_no_conflict() {
        let doc = doc_with(vec![scenario("r1", "happy", true, true)]);
        // harness empty → no overlap
        assert!(dual_write_pairs(&doc, &[]).is_empty());
        assert_eq!(dual_write_count(&doc, &[]), 0);
    }

    #[test]
    fn validate_partitioned_dual_write_message_lists_pairs() {
        let doc = doc_with(vec![scenario("r1", "happy", true, true)]);
        let harness_scenarios = vec![harness("happy", "r1")];
        let issues = validate_partitioned("sample", &doc, &harness_scenarios, true);
        let dual_issue = issues
            .iter()
            .find(|i| i.path == "sample/dual-write")
            .expect("dual-write issue present");
        assert!(dual_issue.message.contains("(r1, happy)"));
        assert!(dual_issue.message.contains("dual-write: 1"));
    }
}
