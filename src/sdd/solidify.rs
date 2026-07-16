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
/// [`ScenarioNode`](crate::sdd::context::tree::ScenarioNode)s (req_id empty).
///
/// Used by the pageindex index rebuild to embed `.feature` behavior details into
/// the tree. Mirrors `solidify_migrate::scenario_from_gherkin` but outputs the
/// tree node type directly and leaves `req_id` empty: Gherkin has no req concept,
/// so feature scenarios are surfaced at the spec level. `GherkinEnv` is not
/// `Clone`, so it is rebuilt here per file.
pub fn parse_feature_file(
    path: &Path,
    lang: &str,
) -> Result<Vec<crate::sdd::context::tree::ScenarioNode>> {
    use crate::sdd::context::tree::ScenarioNode;
    let content =
        fs::read_to_string(path).with_context(|| format!("read feature {}", path.display()))?;
    let env = gherkin::GherkinEnv::new(lang)
        .with_context(|| format!("build gherkin env for language `{lang}`"))?;
    let parsed = gherkin::Feature::parse(&content, env)
        .with_context(|| format!("parse feature {}", path.display()))?;
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
        out.push(ScenarioNode {
            req_id: String::new(),
            id: sc.name.clone(),
            given: given.join("\n"),
            when_: when_.join("\n"),
            then_: then_.join("\n"),
        });
    }
    Ok(out)
}

/// Localized Gherkin keyword set for a single language.
struct GherkinKeywords {
    language_directive: String,
    feature: &'static str,
    scenario: &'static str,
    given: &'static str,
    when_: &'static str,
    then_: &'static str,
    and: &'static str,
}

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
fn is_self_referencing(scenario: &ScenarioEntry) -> bool {
    let when = scenario.when_.trim();
    SELF_REF_PREFIXES.iter().any(|p| when.contains(p))
}

/// Decision for a single scenario during solidify.
enum Decision {
    /// Write this scenario into the `.feature` file.
    Write,
    /// Skip: stays in `spec.toon` as documentation. `reason` is human-facing.
    Skip { reason: &'static str },
}

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
struct CapabilityDelta {
    capability: String,
    scenarios: Vec<ScenarioEntry>,
    feature_path: PathBuf,
}

/// Discover delta spec.toon files under `<change_dir>/specs/<capability>/` and
/// load their `op_scenarios`. Capabilities with no delta file or no scenarios
/// are skipped.
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
}

/// Core solidify entry point. Reads the change's delta scenarios, filters by
/// `feature` field + self-reference check, and writes (or previews) one
/// `.feature` per capability.
///
/// Returns one [`SolidifyReport`] per capability that had any scenarios.
pub fn solidify_change(
    change_dir: &Path,
    root: &Path,
    dry_run: bool,
) -> Result<Vec<SolidifyReport>> {
    let config = load_required_config(&root.join(LLMANSPEC_DIR_NAME))?;
    let locale = config.locale.as_str();
    let lang = locale_to_gherkin_lang(Some(locale), config.bdd.as_ref());
    let kw = keywords_for(&lang);

    let deltas = load_change_deltas(change_dir, root)?;
    let mut reports = Vec::new();
    for delta in deltas {
        let mut to_write = Vec::new();
        let mut skipped = Vec::new();
        for sc in &delta.scenarios {
            match decide(sc) {
                Decision::Write => to_write.push(sc.clone()),
                Decision::Skip { reason } => skipped.push((sc.id.clone(), reason)),
            }
        }

        if to_write.is_empty() && skipped.is_empty() {
            continue;
        }

        if !to_write.is_empty() {
            let body = render_feature(&to_write, &delta.capability, &kw);
            if dry_run {
                println!(
                    "{}",
                    t!(
                        "sdd.solidify.dry_run_preview",
                        path = delta.feature_path.display()
                    )
                );
                println!("{body}");
            } else {
                if let Some(parent) = delta.feature_path.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("create feature dir {}", parent.display()))?;
                }
                atomic_write_with_mode(&delta.feature_path, body.as_bytes(), None)
                    .with_context(|| format!("write feature {}", delta.feature_path.display()))?;
            }
        }

        reports.push(SolidifyReport {
            capability: delta.capability,
            feature_path: delta.feature_path,
            written: to_write.len(),
            skipped,
        });
    }
    Ok(reports)
}

/// CLI entry: `llman sdd solidify <change-id> [--dry-run]`.
pub fn run(change_id: &str, dry_run: bool) -> Result<()> {
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
        // BDD-off: solidify is a no-op (nothing to generate).
        println!("{}", t!("sdd.solidify.bdd_off_noop"));
        return Ok(());
    }

    let change_dir = llmanspec_dir.join("changes").join(change_id);
    if !change_dir.exists() {
        bail!("{}", t!("sdd.solidify.change_not_found", id = change_id));
    }

    let reports = solidify_change(&change_dir, root, dry_run)?;
    if reports.is_empty() {
        println!("{}", t!("sdd.solidify.no_scenarios"));
        return Ok(());
    }

    let mut total_written = 0usize;
    let mut total_skipped = 0usize;
    for report in &reports {
        if dry_run {
            println!(
                "{}",
                t!(
                    "sdd.solidify.summary_dry_run",
                    capability = report.capability,
                    written = report.written,
                    skipped = report.skipped.len(),
                )
            );
        } else {
            println!(
                "{}",
                t!(
                    "sdd.solidify.summary_written",
                    capability = report.capability,
                    path = report.feature_path.display(),
                    written = report.written,
                    skipped = report.skipped.len(),
                )
            );
        }
        total_written += report.written;
        total_skipped += report.skipped.len();
        for (id, reason) in &report.skipped {
            println!(
                "  {}",
                t!("sdd.solidify.skipped_item", id = id, reason = reason)
            );
        }
    }
    println!(
        "{}",
        t!(
            "sdd.solidify.totals",
            written = total_written,
            skipped = total_skipped,
        )
    );
    Ok(())
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
