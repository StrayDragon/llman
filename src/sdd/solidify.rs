//! BDD-on solidify: serialize a change's delta `op_scenarios` into executable
//! `.feature` files under `llmanspec/specs/<capability>/`.
//!
//! Design (see `llmanspec/changes/add-bdd-solidify-workflow/design.md`):
//! - `spec.toon` is the SSOT; `.feature` files are the executable subset.
//! - A scenario's `feature: false` field keeps it in `spec.toon` as docs only.
//! - Self-referencing scenarios (whose `when` invokes `llman sdd
//!   validate|archive|solidify`) are skipped to avoid recursive test nesting.
//! - Framework-agnostic: no `tests/bdd_steps.rs` scanning, no step-pattern
//!   analysis. Whether a scenario is *executable* is decided at runtime by
//!   `bdd.run_command`, not by solidify.

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::{BddConfig, load_required_config};
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::spec::backend::{BACKEND, SpecBackend};
use crate::sdd::spec::ir::ScenarioEntry;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

/// Subcommands whose presence in a scenario `when` makes it self-referencing:
/// solidifying it into a `.feature` would make the BDD runner spawn a nested
/// `llman sdd validate|archive|solidify` (and, transitively, another test run).
const SELF_REF_PREFIXES: &[&str] = &[
    "llman sdd validate",
    "llman sdd archive",
    "llman sdd solidify",
];

/// Map a config locale to a Gherkin parsing language, mirroring
/// `spec::validation::locale_to_gherkin_lang`. `bdd.default_language` wins;
/// `zh-Hans*` → `zh-CN`; else passthrough; `None` → `"en"`.
pub fn locale_to_gherkin_lang(locale: Option<&str>, bdd_config: Option<&BddConfig>) -> String {
    if let Some(bdd) = bdd_config
        && let Some(lang) = &bdd.default_language
        && !lang.trim().is_empty()
    {
        return lang.clone();
    }
    match locale.map(str::trim).filter(|l| !l.is_empty()) {
        Some(l) if l.starts_with("zh-Hans") => "zh-CN".to_string(),
        Some(l) => l.to_string(),
        None => "en".to_string(),
    }
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
    use crate::sdd::spec::partitioned::parse_feature_scenarios;
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

/// Localized Gherkin keyword set for a single language.
#[allow(dead_code)]
struct GherkinKeywords {
    language_directive: String,
    feature: &'static str,
    scenario: &'static str,
    given: &'static str,
    when_: &'static str,
    then_: &'static str,
    and: &'static str,
}

#[allow(dead_code)]
fn keywords_for(lang: &str) -> GherkinKeywords {
    let (feature, scenario, given, when_, then_, and) = match lang {
        "zh-CN" | "zh-TW" | "zh" => ("功能", "场景", "假如", "当", "那么", "而且"),
        // Default to English keywords for any other language.
        _ => ("Feature", "Scenario", "Given", "When", "Then", "And"),
    };
    GherkinKeywords {
        language_directive: format!("# language: {lang}"),
        feature,
        scenario,
        given,
        when_,
        then_,
        and,
    }
}

/// Whether a scenario's `when` step is self-referencing (invokes an llman sdd
/// subcommand that would recurse). See `SELF_REF_PREFIXES`.
#[allow(dead_code)]
fn is_self_referencing(scenario: &ScenarioEntry) -> bool {
    let when = scenario.when_.trim();
    SELF_REF_PREFIXES.iter().any(|p| when.contains(p))
}

/// Decision for a single scenario during solidify.
#[allow(dead_code)]
enum Decision {
    /// Write this scenario into the `.feature` file.
    Write,
    /// Skip: stays in `spec.toon` as documentation. `reason` is human-facing.
    Skip { reason: &'static str },
}

#[allow(dead_code)]
fn decide(scenario: &ScenarioEntry) -> Decision {
    if !scenario.feature {
        Decision::Skip {
            reason: "feature=false — stays in spec.toon",
        }
    } else if is_self_referencing(scenario) {
        Decision::Skip {
            reason: "self-referencing (llman sdd validate|archive|solidify) — stays in spec.toon",
        }
    } else {
        Decision::Write
    }
}

/// A capability's delta scenarios read from a change, plus the target feature
/// path they would solidify into.
#[allow(dead_code)]
struct CapabilityDelta {
    capability: String,
    scenarios: Vec<ScenarioEntry>,
    feature_path: PathBuf,
}

/// Discover delta spec.toon files under `<change_dir>/specs/<capability>/` and
/// load their `op_scenarios`. Capabilities with no delta file or no scenarios
/// are skipped.
#[allow(dead_code)]
fn load_change_deltas(change_dir: &Path, root: &Path) -> Result<Vec<CapabilityDelta>> {
    let mut deltas = Vec::new();
    let change_specs_dir = change_dir.join("specs");
    if !change_specs_dir.exists() {
        return Ok(deltas);
    }
    for entry in fs::read_dir(&change_specs_dir)
        .with_context(|| format!("read change specs dir {}", change_specs_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let capability = entry.file_name().to_string_lossy().to_string();
        let source = entry.path().join(SPEC_FILE);
        if !source.exists() {
            continue;
        }
        let content = fs::read_to_string(&source)
            .with_context(|| format!("read delta {}", source.display()))?;
        let doc = BACKEND
            .parse_delta_spec(&content, &format!("change delta `{capability}`"))
            .with_context(|| format!("parse delta {}", source.display()))?;
        if doc.op_scenarios.is_empty() {
            continue;
        }
        let feature_path = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join(&capability)
            .join(format!("{capability}.feature"));
        deltas.push(CapabilityDelta {
            capability,
            scenarios: doc.op_scenarios,
            feature_path,
        });
    }
    Ok(deltas)
}

/// Render a single scenario into Gherkin text (without leading/trailing blank
/// lines). Multi-line `given`/`when`/`then` fields split into `And` steps after
/// the first keyword line.
#[allow(dead_code)]
fn render_scenario(scenario: &ScenarioEntry, kw: &GherkinKeywords) -> String {
    let mut lines = Vec::new();
    lines.push(format!("  {}: {}", kw.scenario, scenario.id.trim()));

    // GIVEN (may be empty — emit the keyword only when there is content).
    append_steps(&mut lines, kw.given, kw.and, scenario.given.trim(), 4);
    // WHEN (required by validation, always non-empty).
    append_steps(&mut lines, kw.when_, kw.and, scenario.when_.trim(), 4);
    // THEN (required by validation, always non-empty).
    append_steps(&mut lines, kw.then_, kw.and, scenario.then_.trim(), 4);

    lines.join("\n")
}

/// Push step lines for a field split on newlines. The first line uses `first_kw`
/// (e.g. 那么), subsequent lines use `rest_kw` (e.g. 而且). Empty fields emit
/// nothing. `indent` is the number of leading spaces.
#[allow(dead_code)]
fn append_steps(
    lines: &mut Vec<String>,
    first_kw: &str,
    rest_kw: &str,
    field: &str,
    indent: usize,
) {
    let pad = " ".repeat(indent);
    for (i, line) in field
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let kw = if i == 0 { first_kw } else { rest_kw };
        lines.push(format!("{pad}{kw} {line}"));
    }
}

/// Render a full `.feature` file body for a capability from the given scenarios.
/// `purpose` is used as the Feature name fallback when no better title exists.
#[allow(dead_code)]
fn render_feature(scenarios: &[ScenarioEntry], capability: &str, kw: &GherkinKeywords) -> String {
    let mut out = String::new();
    out.push_str(&kw.language_directive);
    out.push('\n');
    out.push_str(&format!(
        "# generated by `llman sdd solidify` from spec.toon ({capability})"
    ));
    out.push('\n');
    out.push_str(&format!("{}: {capability}\n", kw.feature));
    for (i, sc) in scenarios.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&render_scenario(sc, kw));
        out.push('\n');
    }
    out
}

/// A summary of what solidify did (or would do in dry-run) for one capability.
#[derive(Debug, Clone)]
pub struct SolidifyReport {
    pub capability: String,
    pub feature_path: PathBuf,
    pub written: usize,
    pub skipped: Vec<(String, &'static str)>, // (scenario id, reason)
    pub consistency_ok: bool,
    pub consistency_issues: usize,
}

/// Partitioned solidify: consistency gate (+ optional `--write-stubs` from
/// `feature_delta`). Does **not** project toon `op_scenarios` GWT over `.feature`.
pub fn solidify_change(
    change_dir: &Path,
    root: &Path,
    dry_run: bool,
    write_stubs: bool,
) -> Result<Vec<SolidifyReport>> {
    use crate::sdd::spec::backend::{BACKEND, SpecBackend};
    use crate::sdd::spec::partitioned::{
        apply_feature_delta, find_feature_delta_path, load_feature_delta_file,
        load_spec_harness_soft, validate_partitioned,
    };
    use crate::sdd::spec::validation::ValidationLevel;

    let config = load_required_config(&root.join(LLMANSPEC_DIR_NAME))?;
    let locale = config.locale.as_str();
    let lang = locale_to_gherkin_lang(Some(locale), config.bdd.as_ref());

    let change_specs = change_dir.join("specs");
    let mut reports = Vec::new();
    if !change_specs.exists() {
        return Ok(reports);
    }

    for entry in fs::read_dir(&change_specs)
        .with_context(|| format!("read change specs {}", change_specs.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let capability = entry.file_name().to_string_lossy().to_string();
        let main_spec = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join(&capability)
            .join(SPEC_FILE);
        let feature_path = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join(&capability)
            .join(format!("{capability}.feature"));
        let spec_dir = feature_path.parent().unwrap().to_path_buf();

        let mut written = 0usize;
        let mut skipped = Vec::new();

        // Optional stubs from feature_delta only (never from toon op_scenarios).
        if write_stubs && let Some(delta_path) = find_feature_delta_path(&entry.path(), &capability)
        {
            let delta = load_feature_delta_file(&delta_path)?;
            let existing = if feature_path.exists() {
                Some(fs::read_to_string(&feature_path)?)
            } else {
                None
            };
            let existing_ids: std::collections::HashSet<String> = if let Some(ref body) = existing {
                crate::sdd::spec::partitioned::parse_feature_scenarios_content(body, &lang)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| s.id)
                    .collect()
            } else {
                std::collections::HashSet::new()
            };
            let stub_ops: Vec<_> = delta
                .ops
                .iter()
                .filter(|op| op.op == "add" && !existing_ids.contains(&op.id))
                .cloned()
                .collect();
            if !stub_ops.is_empty() {
                let stub_delta = crate::sdd::spec::ir::FeatureDeltaDoc {
                    kind: delta.kind.clone(),
                    target: delta.target.clone(),
                    ops: stub_ops,
                };
                match apply_feature_delta(existing.as_deref(), &capability, &stub_delta, &lang) {
                    Ok(body) if !body.is_empty() => {
                        written = stub_delta.ops.len();
                        if dry_run {
                            println!(
                                "{}",
                                t!(
                                    "sdd.solidify.dry_run_preview",
                                    path = feature_path.display()
                                )
                            );
                            println!("{body}");
                        } else {
                            if let Some(parent) = feature_path.parent() {
                                fs::create_dir_all(parent)?;
                            }
                            atomic_write_with_mode(&feature_path, body.as_bytes(), None)?;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        skipped.push(("stubs".into(), "stub write failed"));
                        eprintln!("solidify --write-stubs: {e}");
                    }
                }
            }
        }

        // Consistency against main spec (if present).
        let mut consistency_issues = 0usize;
        let mut consistency_ok = true;
        if main_spec.exists() {
            let content = fs::read_to_string(&main_spec)?;
            let doc = BACKEND.parse_main_spec(&content, &format!("spec `{capability}`"))?;
            let mut soft = Vec::new();
            let harness = load_spec_harness_soft(&spec_dir, &lang, &mut soft);
            let issues = validate_partitioned(&capability, &doc, &harness, true);
            consistency_issues = issues.len() + soft.len();
            consistency_ok = issues.iter().all(|i| i.level != ValidationLevel::Error)
                && soft.iter().all(|i| i.level != ValidationLevel::Error);
            for issue in soft.iter().chain(issues.iter()) {
                let level = match issue.level {
                    ValidationLevel::Error => "ERROR",
                    ValidationLevel::Warning => "WARNING",
                    ValidationLevel::Info => "INFO",
                };
                println!("  [{level}] {}: {}", issue.path, issue.message);
            }
        }

        reports.push(SolidifyReport {
            capability,
            feature_path,
            written,
            skipped,
            consistency_ok,
            consistency_issues,
        });
    }
    Ok(reports)
}

/// CLI entry: `llman sdd solidify <change-id> [--dry-run] [--write-stubs]`.
pub fn run(change_id: &str, dry_run: bool, write_stubs: bool) -> Result<()> {
    let root = Path::new(".");
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_dir.exists() {
        bail!(
            "no llmanspec directory found at {}",
            llmanspec_dir.display()
        );
    }
    let config = load_required_config(&llmanspec_dir)?;
    if config.bdd.is_none() {
        println!("{}", t!("sdd.solidify.bdd_off_noop"));
        return Ok(());
    }

    let change_dir = llmanspec_dir.join("changes").join(change_id);
    if !change_dir.exists() {
        bail!("{}", t!("sdd.solidify.change_not_found", id = change_id));
    }

    let reports = solidify_change(&change_dir, root, dry_run, write_stubs)?;
    if reports.is_empty() {
        println!("{}", t!("sdd.solidify.consistency_ok"));
        return Ok(());
    }

    let mut all_ok = true;
    for report in &reports {
        if report.consistency_ok {
            println!(
                "{}",
                t!(
                    "sdd.solidify.consistency_cap_ok",
                    capability = report.capability
                )
            );
        } else {
            all_ok = false;
            println!(
                "{}",
                t!(
                    "sdd.solidify.consistency_cap_fail",
                    capability = report.capability,
                    count = report.consistency_issues
                )
            );
        }
        if report.written > 0 {
            println!(
                "  stubs: {} -> {}",
                report.written,
                report.feature_path.display()
            );
        }
    }
    if all_ok {
        println!("{}", t!("sdd.solidify.consistency_ok"));
        Ok(())
    } else {
        bail!("{}", t!("sdd.solidify.consistency_failed"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdd::spec::ir::ScenarioEntry;

    fn sc(id: &str, when: &str, feature: bool) -> ScenarioEntry {
        ScenarioEntry {
            req_id: "r1".to_string(),
            id: id.to_string(),
            given: "context".to_string(),
            when_: when.to_string(),
            then_: "result".to_string(),
            feature,
        }
    }

    #[test]
    fn self_referencing_when_is_detected() {
        assert!(is_self_referencing(&sc(
            "a",
            "运行 llman sdd validate foo",
            true
        )));
        assert!(is_self_referencing(&sc(
            "a",
            "run llman sdd archive bar",
            true
        )));
        assert!(is_self_referencing(&sc("a", "llman sdd solidify x", true)));
    }

    #[test]
    fn non_self_referencing_when_passes() {
        assert!(!is_self_referencing(&sc("a", "运行 llman sdd show", true)));
        assert!(!is_self_referencing(&sc("a", "user runs llman x cc", true)));
        // Bare `validate` must NOT match (only the `llman sdd validate` form does).
        assert!(!is_self_referencing(&sc("a", "the validator runs", true)));
    }

    #[test]
    fn feature_false_always_skips() {
        // Even a perfectly executable scenario is skipped when feature=false.
        assert!(matches!(
            decide(&sc("a", "run llman x cc", false)),
            Decision::Skip { .. }
        ));
    }

    #[test]
    fn feature_true_executable_writes() {
        assert!(matches!(
            decide(&sc("a", "run llman x cc", true)),
            Decision::Write
        ));
    }

    #[test]
    fn feature_true_self_ref_skips() {
        assert!(matches!(
            decide(&sc("a", "llman sdd validate x", true)),
            Decision::Skip { .. }
        ));
    }

    #[test]
    fn zh_hans_maps_to_zh_cn() {
        assert_eq!(locale_to_gherkin_lang(Some("zh-Hans"), None), "zh-CN");
    }

    #[test]
    fn render_zh_cn_scenario_uses_localized_keywords() {
        let kw = keywords_for("zh-CN");
        let s = ScenarioEntry {
            req_id: "r1".to_string(),
            id: "happy".to_string(),
            given: "上下文".to_string(),
            when_: "用户运行 llman".to_string(),
            then_: "结果为 A".to_string(),
            feature: true,
        };
        let rendered = render_scenario(&s, &kw);
        assert!(rendered.contains("场景: happy"));
        assert!(rendered.contains("假如 上下文"));
        assert!(rendered.contains("当 用户运行 llman"));
        assert!(rendered.contains("那么 结果为 A"));
    }

    #[test]
    fn render_then_multiline_splits_into_and() {
        let kw = keywords_for("en");
        let s = ScenarioEntry {
            req_id: "r1".to_string(),
            id: "happy".to_string(),
            given: String::new(),
            when_: "trigger".to_string(),
            then_: "result A\nresult B".to_string(),
            feature: true,
        };
        let rendered = render_scenario(&s, &kw);
        assert!(rendered.contains("Then result A"));
        assert!(rendered.contains("And result B"));
        // Empty given emits no Given line.
        assert!(!rendered.contains("Given"));
    }

    #[test]
    fn render_feature_has_language_directive_and_feature_header() {
        let kw = keywords_for("zh-CN");
        let s = sc("s1", "w", true);
        let body = render_feature(&[s], "cli", &kw);
        assert!(body.starts_with("# language: zh-CN"));
        assert!(body.contains("# generated by `llman sdd solidify`"));
        assert!(body.contains("功能: cli"));
    }

    #[test]
    fn parse_feature_file_extracts_scenarios() {
        let tmp = tempfile::TempDir::new().unwrap();
        let fpath = tmp.path().join("a.feature");
        std::fs::write(
            &fpath,
            "# language: zh-CN\n\
             功能: 示例\n\
             \n\
             场景: happy path\n\
             假如 一个前置条件\n\
             当 执行某动作\n\
             那么 得到某结果\n",
        )
        .unwrap();
        let nodes = parse_feature_file(&fpath, "zh-CN").unwrap();
        assert_eq!(nodes.len(), 1);
        let s = &nodes[0];
        assert_eq!(s.id, "happy path");
        assert_eq!(s.given, "一个前置条件");
        assert_eq!(s.when_, "执行某动作");
        assert_eq!(s.then_, "得到某结果");
        // Feature scenarios are spec-level: req_id is empty.
        assert!(s.req_id.is_empty(), "feature scenario req_id must be empty");
    }

    #[test]
    fn parse_feature_file_empty_when_no_scenarios() {
        let tmp = tempfile::TempDir::new().unwrap();
        let fpath = tmp.path().join("empty.feature");
        std::fs::write(&fpath, "# language: zh-CN\n功能: 空\n").unwrap();
        let nodes = parse_feature_file(&fpath, "zh-CN").unwrap();
        assert!(nodes.is_empty(), "no scenarios → empty vec");
    }
}
