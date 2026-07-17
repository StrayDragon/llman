//! Partitioned SSOT helpers: harness (`.feature`) + constraints (`spec.toon`).
//!
//! See `llmanspec/changes/add-sdd-bdd-partitioned-ssot/design.md`.

use crate::sdd::spec::ir::{FeatureDeltaDoc, MainSpecDoc, ScenarioEntry};
use crate::sdd::spec::validation::{ValidationIssue, ValidationLevel, discover_features};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
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
    let harness_ids: HashSet<&str> = harness.iter().map(|h| h.id.as_str()).collect();
    executable_toon_scenarios(doc)
        .into_iter()
        .filter(|s| {
            if !harness_ids.contains(s.id.as_str()) {
                return false;
            }
            // Dual-write if toon still carries full GWT text for an executable row.
            gwt_nonempty(&s.given, &s.when_, &s.then_)
        })
        .count()
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
    let dual = dual_write_count(doc, harness);
    if dual > 0 {
        let level = if strict {
            ValidationLevel::Error
        } else {
            ValidationLevel::Warning
        };
        issues.push(ValidationIssue {
            level,
            path: format!("{spec_name}/dual-write"),
            message: format!(
                "dual-write: {dual} executable scenario(s) still have GWT in both spec.toon and .feature; run `llman sdd project partition-migrate`"
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

pub fn parse_feature_delta(content: &str, context: &str) -> Result<FeatureDeltaDoc> {
    let doc: FeatureDeltaDoc = toon_format::decode_default(content.trim())
        .map_err(|err| anyhow::anyhow!("{context}: failed to parse feature_delta: {err}"))?;
    if doc.kind.trim() != "llman.sdd.feature_delta" {
        bail!(
            "{context}: kind must be `llman.sdd.feature_delta`, got `{}`",
            doc.kind.trim()
        );
    }
    Ok(doc)
}

pub fn load_feature_delta_file(path: &Path) -> Result<FeatureDeltaDoc> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read feature_delta {}", path.display()))?;
    parse_feature_delta(&content, &format!("feature_delta `{}`", path.display()))
}

/// Resolve the harness file path for a feature_delta under `spec_dir`.
///
/// Uses `delta.target` when non-empty (bare `*.feature` basename only); otherwise
/// `{capability}.feature`.
pub fn resolve_feature_delta_target_path(
    spec_dir: &Path,
    capability: &str,
    delta: &FeatureDeltaDoc,
) -> Result<std::path::PathBuf> {
    let file = sanitize_feature_delta_target(delta.target.trim(), capability)?;
    Ok(spec_dir.join(file))
}

/// Validate / normalize `feature_delta.target` to a bare `*.feature` filename.
pub fn sanitize_feature_delta_target(raw: &str, capability: &str) -> Result<String> {
    let t = raw.trim();
    if t.is_empty() {
        return Ok(format!("{capability}.feature"));
    }
    if t.contains('/') || t.contains('\\') || t.contains("..") {
        bail!(
            "feature_delta target must be a bare `*.feature` filename (no directories), got `{t}`"
        );
    }
    if !t.ends_with(".feature") {
        bail!("feature_delta target must end with `.feature`, got `{t}`");
    }
    Ok(t.to_string())
}

/// Display title for the Gherkin `Feature:` / `功能:` line (file stem of target).
pub fn feature_title_for_target(target_file: &str, capability: &str) -> String {
    std::path::Path::new(target_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(capability)
        .to_string()
}

/// Discover `<capability>.feature.delta.toon` under a change capability dir.
pub fn find_feature_delta_path(
    change_cap_dir: &Path,
    capability: &str,
) -> Option<std::path::PathBuf> {
    let p1 = change_cap_dir.join(format!("{capability}.feature.delta.toon"));
    if p1.exists() {
        return Some(p1);
    }
    let p2 = change_cap_dir.join("feature.delta.toon");
    if p2.exists() {
        return Some(p2);
    }
    None
}

fn keywords_for(
    lang: &str,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match lang {
        "zh-CN" | "zh-TW" | "zh" => ("功能", "场景", "假如", "当", "那么", "而且"),
        _ => ("Feature", "Scenario", "Given", "When", "Then", "And"),
    }
}

fn render_scenario_block(
    id: &str,
    req_id: Option<&str>,
    given: &str,
    when_: &str,
    then_: &str,
    lang: &str,
) -> String {
    let (_feat, scenario, given_kw, when_kw, then_kw, _and) = keywords_for(lang);
    let mut out = String::new();
    if let Some(rid) = req_id.filter(|s| !s.is_empty()) {
        out.push_str(&format!("  @req:{rid}\n"));
    }
    out.push_str(&format!("  {scenario}: {id}\n"));
    if !given.trim().is_empty() {
        out.push_str(&format!("    {given_kw} {}\n", given.trim()));
    }
    if !when_.trim().is_empty() {
        out.push_str(&format!("    {when_kw} {}\n", when_.trim()));
    }
    if !then_.trim().is_empty() {
        out.push_str(&format!("    {then_kw} {}\n", then_.trim()));
    }
    out
}

/// Apply feature_delta ops onto an existing `.feature` body. Returns new body.
///
/// `feature_title` is the Gherkin Feature/功能 name (usually the target file stem).
pub fn apply_feature_delta(
    existing: Option<&str>,
    feature_title: &str,
    delta: &FeatureDeltaDoc,
    lang: &str,
) -> Result<String> {
    let (feat_kw, scenario_kw, given_kw, when_kw, then_kw, _and) = keywords_for(lang);
    let mut map: HashMap<String, FeatureScenario> = HashMap::new();

    if let Some(body) = existing.filter(|s| !s.trim().is_empty()) {
        for sc in parse_feature_scenarios_content(body, lang)? {
            map.insert(sc.id.clone(), sc);
        }
    }

    for op in &delta.ops {
        match op.op.as_str() {
            "add" | "modify" => {
                if op.op == "modify" && !map.contains_key(&op.id) {
                    bail!("feature_delta modify: scenario `{}` not found", op.id);
                }
                if op.op == "add" && map.contains_key(&op.id) {
                    bail!("feature_delta add: scenario `{}` already exists", op.id);
                }
                let mut req_ids = Vec::new();
                if !op.req_id.trim().is_empty() {
                    req_ids.push(op.req_id.clone());
                }
                map.insert(
                    op.id.clone(),
                    FeatureScenario {
                        id: op.id.clone(),
                        given: op.given.clone(),
                        when_: op.when_.clone(),
                        then_: op.then_.clone(),
                        req_ids,
                        tags: if op.req_id.trim().is_empty() {
                            Vec::new()
                        } else {
                            vec![format!("req:{}", op.req_id)]
                        },
                    },
                );
            }
            "remove" => {
                if map.remove(&op.id).is_none() {
                    bail!("feature_delta remove: scenario `{}` not found", op.id);
                }
            }
            other => bail!("feature_delta: unsupported op `{other}`"),
        }
    }

    if map.is_empty() {
        return Ok(String::new());
    }

    let mut ids: Vec<String> = map.keys().cloned().collect();
    ids.sort();
    let mut out = String::new();
    out.push_str(&format!("# language: {lang}\n"));
    out.push_str("# managed by llman sdd (Partitioned SSOT harness)\n");
    out.push_str(&format!("{feat_kw}: {feature_title}\n"));
    for id in ids {
        let sc = map.get(&id).expect("id in map");
        let rid = sc.req_ids.first().map(|s| s.as_str());
        out.push('\n');
        out.push_str(&render_scenario_block(
            &sc.id, rid, &sc.given, &sc.when_, &sc.then_, lang,
        ));
        let _ = (scenario_kw, given_kw, when_kw, then_kw);
    }
    Ok(out)
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
    use crate::sdd::spec::ir::FeatureDeltaOp;

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

    #[test]
    fn feature_delta_add_modify_remove() {
        let delta = FeatureDeltaDoc {
            kind: "llman.sdd.feature_delta".into(),
            target: "demo.feature".into(),
            ops: vec![
                FeatureDeltaOp {
                    op: "add".into(),
                    id: "s1".into(),
                    req_id: "r1".into(),
                    given: "g".into(),
                    when_: "w".into(),
                    then_: "t".into(),
                },
                FeatureDeltaOp {
                    op: "modify".into(),
                    id: "s1".into(),
                    req_id: "r1".into(),
                    given: "g2".into(),
                    when_: "w2".into(),
                    then_: "t2".into(),
                },
            ],
        };
        // first apply add+modify in one doc — modify after add in same batch works via map
        let body = apply_feature_delta(None, "demo", &delta, "zh-CN").unwrap();
        assert!(body.contains("场景: s1"));
        assert!(body.contains("@req:r1"));
        assert!(body.contains("g2"));

        let delta_rm = FeatureDeltaDoc {
            kind: "llman.sdd.feature_delta".into(),
            target: "demo.feature".into(),
            ops: vec![FeatureDeltaOp {
                op: "remove".into(),
                id: "s1".into(),
                req_id: String::new(),
                given: String::new(),
                when_: String::new(),
                then_: String::new(),
            }],
        };
        let empty = apply_feature_delta(Some(&body), "demo", &delta_rm, "zh-CN").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn sanitize_target_defaults_and_rejects_paths() {
        assert_eq!(
            sanitize_feature_delta_target("", "cap").unwrap(),
            "cap.feature"
        );
        assert_eq!(
            sanitize_feature_delta_target("global-req-id.feature", "cap").unwrap(),
            "global-req-id.feature"
        );
        assert!(sanitize_feature_delta_target("../x.feature", "cap").is_err());
        assert!(sanitize_feature_delta_target("a/b.feature", "cap").is_err());
        assert!(sanitize_feature_delta_target("nope", "cap").is_err());
    }

    #[test]
    fn resolve_target_path_uses_delta_field() {
        let dir = std::path::Path::new("/tmp/specs/cap");
        let delta = FeatureDeltaDoc {
            kind: "llman.sdd.feature_delta".into(),
            target: "global-req-id.feature".into(),
            ops: vec![],
        };
        let p = resolve_feature_delta_target_path(dir, "cap", &delta).unwrap();
        assert_eq!(p, dir.join("global-req-id.feature"));
        assert_eq!(
            feature_title_for_target("global-req-id.feature", "cap"),
            "global-req-id"
        );
    }
}
