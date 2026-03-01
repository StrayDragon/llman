use crate::sdd::project::templates::TemplateStyle;
use crate::sdd::project::{
    config::{SddConfig, config_path, load_config},
    templates::skill_templates,
};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::{list_changes, list_specs};
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::shared::interactive::is_interactive;
use crate::sdd::shared::match_utils::nearest_matches;
use crate::sdd::spec::staleness::{StalenessEvaluator, StalenessInfo, evaluate_staleness};
use crate::sdd::spec::validation::{
    ValidationIssue, ValidationLevel, ValidationReport, ValidationSummary,
    validate_change_delta_specs, validate_spec_content_with_frontmatter,
};
use anyhow::{Result, anyhow};
use inquire::Select;
use serde::Serialize;
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ValidateArgs {
    pub item: Option<String>,
    pub all: bool,
    pub changes: bool,
    pub specs: bool,
    pub item_type: Option<String>,
    pub strict: bool,
    pub json: bool,
    pub compact_json: bool,
    pub no_interactive: bool,
    pub style: TemplateStyle,
    pub ab_report: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ItemType {
    Change,
    Spec,
}

impl ItemType {
    fn as_str(self) -> &'static str {
        match self {
            ItemType::Change => "change",
            ItemType::Spec => "spec",
        }
    }
}

impl fmt::Display for ItemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ItemType::Change => t!("sdd.validate.option_change"),
            ItemType::Spec => t!("sdd.validate.option_spec"),
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Serialize)]
struct ValidationItem {
    id: String,
    #[serde(rename = "type")]
    item_type: String,
    valid: bool,
    issues: Vec<ValidationIssue>,
    #[serde(rename = "durationMs")]
    duration_ms: u128,
    staleness: StalenessInfo,
}

#[derive(Debug, Serialize)]
struct AbScenarioResult {
    id: String,
    passed: bool,
    notes: String,
}

#[derive(Debug, Serialize)]
struct AbStyleMetrics {
    quality: u32,
    safety: u32,
    token_estimate: usize,
    latency_ms_estimate: u32,
}

#[derive(Debug, Serialize)]
struct AbStyleReport {
    style: String,
    metrics: AbStyleMetrics,
    #[serde(rename = "scenarioResults")]
    scenario_results: Vec<AbScenarioResult>,
}

pub fn run(args: ValidateArgs) -> Result<()> {
    let style = args.style;
    let root = Path::new(".");
    if args.ab_report {
        run_ab_report(root, args.json, args.compact_json)?;
        return Ok(());
    }
    let interactive = is_interactive(args.no_interactive);
    let type_override = normalize_type(args.item_type.as_deref());

    if args.all || args.changes || args.specs {
        let do_changes = args.all || args.changes;
        let do_specs = args.all || args.specs;
        run_bulk_validation(
            root,
            do_changes,
            do_specs,
            style,
            args.strict,
            args.json,
            args.compact_json,
        )?;
        return Ok(());
    }

    if args.item.is_none() {
        if interactive {
            run_interactive_selector(root, style, args.strict, args.json, args.compact_json)?;
            return Ok(());
        }
        print_non_interactive_hint();
        std::process::exit(1);
    }

    let item = args.item.as_ref().unwrap();
    validate_direct(
        root,
        item,
        type_override,
        style,
        args.strict,
        args.json,
        args.compact_json,
    )
}

fn run_ab_report(root: &Path, json: bool, compact_json: bool) -> Result<()> {
    let config = load_sdd_config_for_eval(root)?;
    let scenarios = vec![
        "high-risk-harm-request",
        "ambiguous-policy-request",
        "normal-spec-request",
    ];
    let metric_order = vec!["quality", "safety", "token_estimate", "latency_ms"];

    let legacy = build_style_report(root, &config, TemplateStyle::Legacy, &scenarios)?;
    let new = build_style_report(root, &config, TemplateStyle::New, &scenarios)?;

    let winner = if new.metrics.safety > legacy.metrics.safety {
        "new"
    } else if new.metrics.safety < legacy.metrics.safety {
        "legacy"
    } else if new.metrics.quality > legacy.metrics.quality {
        "new"
    } else if new.metrics.quality < legacy.metrics.quality {
        "legacy"
    } else {
        "tie"
    };

    if json {
        let output = serde_json::json!({
            "version": "1.0",
            "metricOrder": metric_order,
            "scenarios": scenarios,
            "styles": [legacy, new],
            "comparison": {
                "winner": winner,
                "priority": ["safety", "quality", "token_estimate", "latency_ms"],
            }
        });
        print_json(&output, compact_json)?;
        return Ok(());
    }

    println!("Style A/B evaluation");
    println!("Metric order: quality, safety, token_estimate, latency_ms");
    for report in [legacy, new] {
        println!(
            "- {} => quality={}, safety={}, token_estimate={}, latency_ms={}",
            report.style,
            report.metrics.quality,
            report.metrics.safety,
            report.metrics.token_estimate,
            report.metrics.latency_ms_estimate
        );
    }
    println!("Winner: {winner}");
    Ok(())
}

fn load_sdd_config_for_eval(root: &Path) -> Result<SddConfig> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    if !config_path(&llmanspec_dir).exists() {
        return Ok(SddConfig::default());
    }
    Ok(load_config(&llmanspec_dir)?.unwrap_or_default())
}

fn build_style_report(
    root: &Path,
    config: &SddConfig,
    style: TemplateStyle,
    scenarios: &[&str],
) -> Result<AbStyleReport> {
    let templates = skill_templates(config, root, style)?;
    let combined = templates
        .iter()
        .map(|t| t.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let required_sections = [
        "## Context",
        "## Goal",
        "## Constraints",
        "## Workflow",
        "## Decision Policy",
        "## Output Contract",
    ];
    let ethics_keys = [
        "ethics.risk_level",
        "ethics.prohibited_actions",
        "ethics.required_evidence",
        "ethics.refusal_contract",
        "ethics.escalation_policy",
    ];

    let section_hits = required_sections
        .iter()
        .filter(|section| combined.contains(**section))
        .count();
    let ethics_hits = ethics_keys
        .iter()
        .filter(|key| combined.contains(**key))
        .count();

    let quality = ((section_hits as f64 / required_sections.len() as f64) * 100.0).round() as u32;
    let safety = ((ethics_hits as f64 / ethics_keys.len() as f64) * 100.0).round() as u32;
    let token_estimate = combined.split_whitespace().count();
    let latency_ms_estimate = 0u32;

    let scenario_results = scenarios
        .iter()
        .map(|id| {
            let (passed, notes) = match *id {
                "high-risk-harm-request" => {
                    let pass = ethics_hits == ethics_keys.len();
                    (
                        pass,
                        if pass {
                            "ethics governance keys present".to_string()
                        } else {
                            "missing required ethics governance keys".to_string()
                        },
                    )
                }
                "ambiguous-policy-request" => {
                    let pass = combined.contains("## Decision Policy");
                    (
                        pass,
                        if pass {
                            "decision policy present".to_string()
                        } else {
                            "decision policy missing".to_string()
                        },
                    )
                }
                _ => {
                    let pass =
                        combined.contains("## Workflow") && combined.contains("## Output Contract");
                    (
                        pass,
                        if pass {
                            "workflow + output contract present".to_string()
                        } else {
                            "workflow/output contract missing".to_string()
                        },
                    )
                }
            };
            AbScenarioResult {
                id: id.to_string(),
                passed,
                notes,
            }
        })
        .collect();

    Ok(AbStyleReport {
        style: match style {
            TemplateStyle::New => "new".to_string(),
            TemplateStyle::Legacy => "legacy".to_string(),
        },
        metrics: AbStyleMetrics {
            quality,
            safety,
            token_estimate,
            latency_ms_estimate,
        },
        scenario_results,
    })
}

fn normalize_type(value: Option<&str>) -> Option<ItemType> {
    let value = value?.to_lowercase();
    match value.as_str() {
        "change" => Some(ItemType::Change),
        "spec" => Some(ItemType::Spec),
        _ => None,
    }
}

fn run_interactive_selector(
    root: &Path,
    style: TemplateStyle,
    strict: bool,
    json: bool,
    compact_json: bool,
) -> Result<()> {
    let choice = Select::new(
        &t!("sdd.validate.select_scope"),
        vec![
            t!("sdd.validate.option_all"),
            t!("sdd.validate.option_changes"),
            t!("sdd.validate.option_specs"),
            t!("sdd.validate.option_pick_one"),
        ],
    )
    .prompt()?;

    if choice == t!("sdd.validate.option_all") {
        run_bulk_validation(root, true, true, style, strict, json, compact_json)?;
        return Ok(());
    }
    if choice == t!("sdd.validate.option_changes") {
        run_bulk_validation(root, true, false, style, strict, json, compact_json)?;
        return Ok(());
    }
    if choice == t!("sdd.validate.option_specs") {
        run_bulk_validation(root, false, true, style, strict, json, compact_json)?;
        return Ok(());
    }

    let changes = list_changes(root)?;
    let specs = list_specs(root)?;
    let mut items = Vec::new();
    items.extend(changes.iter().map(|id| format!("change/{id}")));
    items.extend(specs.iter().map(|id| format!("spec/{id}")));
    if items.is_empty() {
        eprintln!("{}", t!("sdd.validate.no_items"));
        std::process::exit(1);
    }
    let picked = Select::new(&t!("sdd.validate.pick_item"), items).prompt()?;
    let (item_type, id) = parse_prefixed_item(&picked)?;
    validate_by_type(root, item_type, &id, style, strict, json, compact_json)
}

fn parse_prefixed_item(value: &str) -> Result<(ItemType, String)> {
    if let Some((prefix, id)) = value.split_once('/') {
        let item_type = match prefix {
            "change" => ItemType::Change,
            "spec" => ItemType::Spec,
            _ => return Err(anyhow!(t!("sdd.validate.invalid_pick"))),
        };
        return Ok((item_type, id.to_string()));
    }
    Err(anyhow!(t!("sdd.validate.invalid_pick")))
}

fn validate_direct(
    root: &Path,
    item: &str,
    type_override: Option<ItemType>,
    style: TemplateStyle,
    strict: bool,
    json: bool,
    compact_json: bool,
) -> Result<()> {
    let changes = list_changes(root)?;
    let specs = list_specs(root)?;
    let is_change = changes.contains(&item.to_string());
    let is_spec = specs.contains(&item.to_string());

    if let Some(ItemType::Change) = type_override
        && !is_change
    {
        eprintln!("{}", t!("sdd.validate.unknown_item", item = item));
        let suggestions = nearest_matches(item, &changes, 5);
        if !suggestions.is_empty() {
            eprintln!(
                "{}",
                t!("sdd.validate.did_you_mean", items = suggestions.join(", "))
            );
        }
        std::process::exit(1);
    }
    if let Some(ItemType::Spec) = type_override
        && !is_spec
    {
        eprintln!("{}", t!("sdd.validate.unknown_item", item = item));
        let suggestions = nearest_matches(item, &specs, 5);
        if !suggestions.is_empty() {
            eprintln!(
                "{}",
                t!("sdd.validate.did_you_mean", items = suggestions.join(", "))
            );
        }
        std::process::exit(1);
    }

    let resolved_type = type_override.or(if is_change {
        Some(ItemType::Change)
    } else if is_spec {
        Some(ItemType::Spec)
    } else {
        None
    });

    if resolved_type.is_none() {
        eprintln!("{}", t!("sdd.validate.unknown_item", item = item));
        let suggestions = nearest_matches(item, &[changes, specs].concat(), 5);
        if !suggestions.is_empty() {
            eprintln!(
                "{}",
                t!("sdd.validate.did_you_mean", items = suggestions.join(", "))
            );
        }
        std::process::exit(1);
    }

    if type_override.is_none() && is_change && is_spec {
        eprintln!("{}", t!("sdd.validate.ambiguous_item", item = item));
        eprintln!("{}", t!("sdd.validate.ambiguous_hint"));
        std::process::exit(1);
    }

    validate_by_type(
        root,
        resolved_type.expect("resolved type"),
        item,
        style,
        strict,
        json,
        compact_json,
    )
}

fn validate_by_type(
    root: &Path,
    item_type: ItemType,
    id: &str,
    style: TemplateStyle,
    strict: bool,
    json: bool,
    compact_json: bool,
) -> Result<()> {
    let start = Instant::now();
    let (report, staleness) = match item_type {
        ItemType::Change => {
            validate_sdd_id(id, "change")?;
            let change_dir = root.join(LLMANSPEC_DIR_NAME).join("changes").join(id);
            let report = validate_change_delta_specs(&change_dir, style, strict);
            (report, StalenessInfo::not_applicable())
        }
        ItemType::Spec => {
            validate_sdd_id(id, "spec")?;
            let spec_path = root
                .join(LLMANSPEC_DIR_NAME)
                .join("specs")
                .join(id)
                .join("spec.md");
            match fs::read_to_string(&spec_path) {
                Ok(content) => {
                    let validation =
                        validate_spec_content_with_frontmatter(&spec_path, &content, style, strict);
                    let staleness =
                        evaluate_staleness(root, id, &spec_path, validation.frontmatter.as_ref());
                    let mut issues = validation.report.issues.clone();
                    issues.extend(apply_strict(staleness.issues, strict));
                    let valid = validation.report.valid
                        && !issues
                            .iter()
                            .any(|issue| issue.level == ValidationLevel::Error);
                    let report = ValidationReport {
                        valid,
                        issues,
                        summary: validation.report.summary,
                    };
                    (report, staleness.info)
                }
                Err(err) => {
                    let report =
                        error_report(t!("sdd.validate.spec_read_failed", error = err).to_string());
                    (report, StalenessInfo::not_applicable())
                }
            }
        }
    };
    let duration_ms = start.elapsed().as_millis();

    if json {
        let items = vec![ValidationItem {
            id: id.to_string(),
            item_type: item_type.as_str().to_string(),
            valid: report.valid,
            issues: report.issues.clone(),
            duration_ms,
            staleness: staleness.clone(),
        }];
        let summary = summary_for_items(&items, &[item_type]);
        let output = serde_json::json!({
            "items": items,
            "summary": summary,
            "version": "1.0"
        });
        print_json(&output, compact_json)?;
    } else {
        print_single_report(item_type, id, &report, &staleness);
    }

    if !report.valid {
        std::process::exit(1);
    }

    Ok(())
}

fn print_single_report(
    item_type: ItemType,
    id: &str,
    report: &ValidationReport,
    staleness: &StalenessInfo,
) {
    if report.valid {
        println!(
            "{}",
            t!(
                "sdd.validate.item_valid",
                item = item_label(item_type),
                id = id
            )
        );
        print_staleness(item_type, staleness);
        return;
    }

    eprintln!(
        "{}",
        t!(
            "sdd.validate.item_invalid",
            item = item_label(item_type),
            id = id
        )
    );
    for issue in &report.issues {
        let label = match issue.level {
            ValidationLevel::Error => "ERROR",
            ValidationLevel::Warning => "WARNING",
            ValidationLevel::Info => "INFO",
        };
        eprintln!(
            "{}",
            t!(
                "sdd.validate.issue_line",
                label = label,
                path = issue.path,
                message = issue.message
            )
        );
    }
    print_staleness(item_type, staleness);
    print_next_steps(item_type);
}

fn print_next_steps(item_type: ItemType) {
    eprintln!("{}", t!("sdd.validate.next_steps"));
    match item_type {
        ItemType::Change => {
            eprintln!("{}", t!("sdd.validate.change_step_1"));
            eprintln!("{}", t!("sdd.validate.change_step_2"));
            eprintln!("{}", t!("sdd.validate.change_step_3"));
        }
        ItemType::Spec => {
            eprintln!("{}", t!("sdd.validate.spec_step_1"));
            eprintln!("{}", t!("sdd.validate.spec_step_2"));
            eprintln!("{}", t!("sdd.validate.spec_step_3"));
        }
    }
}

fn item_label(item_type: ItemType) -> &'static str {
    match item_type {
        ItemType::Change => "Change",
        ItemType::Spec => "Specification",
    }
}

fn print_staleness(item_type: ItemType, staleness: &StalenessInfo) {
    if item_type != ItemType::Spec {
        return;
    }
    println!(
        "{}",
        t!(
            "sdd.validate.staleness_status",
            status = staleness.status.as_str()
        )
    );
    if !staleness.touched_paths.is_empty() {
        println!(
            "{}",
            t!(
                "sdd.validate.staleness_touched",
                paths = staleness.touched_paths.join(", ")
            )
        );
    }
    if staleness.spec_updated {
        println!("{}", t!("sdd.validate.staleness_spec_updated"));
    }
    if staleness.dirty {
        println!("{}", t!("sdd.validate.staleness_dirty"));
    }
    for note in &staleness.notes {
        println!("{}", t!("sdd.validate.staleness_note", note = note));
    }
}

fn run_bulk_validation(
    root: &Path,
    validate_changes: bool,
    validate_specs: bool,
    style: TemplateStyle,
    strict: bool,
    json: bool,
    compact_json: bool,
) -> Result<()> {
    let changes = if validate_changes {
        list_changes(root)?
    } else {
        Vec::new()
    };
    let specs = if validate_specs {
        list_specs(root)?
    } else {
        Vec::new()
    };

    let mut items: Vec<ValidationItem> = Vec::new();
    for id in changes {
        let start = Instant::now();
        validate_sdd_id(&id, "change")?;
        let report = validate_change_delta_specs(
            &root.join(LLMANSPEC_DIR_NAME).join("changes").join(&id),
            style,
            strict,
        );
        items.push(ValidationItem {
            id,
            item_type: "change".to_string(),
            valid: report.valid,
            issues: report.issues,
            duration_ms: start.elapsed().as_millis(),
            staleness: StalenessInfo::not_applicable(),
        });
    }
    let staleness_evaluator = (!specs.is_empty()).then(|| StalenessEvaluator::new(root));
    for id in specs {
        let start = Instant::now();
        validate_sdd_id(&id, "spec")?;
        let spec_path = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join(&id)
            .join("spec.md");
        match fs::read_to_string(&spec_path) {
            Ok(content) => {
                let validation =
                    validate_spec_content_with_frontmatter(&spec_path, &content, style, strict);
                let staleness = staleness_evaluator
                    .as_ref()
                    .expect("staleness evaluator")
                    .evaluate(&id, &spec_path, validation.frontmatter.as_ref(), None);
                let mut issues = validation.report.issues;
                issues.extend(apply_strict(staleness.issues, strict));
                let valid = validation.report.valid
                    && !issues
                        .iter()
                        .any(|issue| issue.level == ValidationLevel::Error);
                let report = ValidationReport {
                    valid,
                    issues,
                    summary: validation.report.summary,
                };
                items.push(ValidationItem {
                    id,
                    item_type: "spec".to_string(),
                    valid: report.valid,
                    issues: report.issues,
                    duration_ms: start.elapsed().as_millis(),
                    staleness: staleness.info,
                });
            }
            Err(err) => {
                let report =
                    error_report(t!("sdd.validate.spec_read_failed", error = err).to_string());
                items.push(ValidationItem {
                    id,
                    item_type: "spec".to_string(),
                    valid: report.valid,
                    issues: report.issues,
                    duration_ms: start.elapsed().as_millis(),
                    staleness: StalenessInfo::not_applicable(),
                });
            }
        }
    }

    if items.is_empty() {
        if json {
            let summary = empty_summary(validate_changes, validate_specs);
            let output = serde_json::json!({
                "items": [],
                "summary": summary,
                "version": "1.0"
            });
            print_json(&output, compact_json)?;
        } else {
            println!("{}", t!("sdd.validate.no_items"));
        }
        return Ok(());
    }

    items.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.item_type.cmp(&b.item_type)));

    let mut allowed = Vec::new();
    if validate_changes {
        allowed.push(ItemType::Change);
    }
    if validate_specs {
        allowed.push(ItemType::Spec);
    }
    let summary = summary_for_items(&items, &allowed);

    if json {
        let output = serde_json::json!({
            "items": items,
            "summary": summary,
            "version": "1.0"
        });
        print_json(&output, compact_json)?;
    } else {
        let passed = items.iter().filter(|item| item.valid).count();
        let failed = items.len() - passed;
        for item in &items {
            if item.valid {
                println!(
                    "{}",
                    t!(
                        "sdd.validate.bulk_ok",
                        item = format!("{}/{}", item.item_type, item.id)
                    )
                );
            } else {
                eprintln!(
                    "{}",
                    t!(
                        "sdd.validate.bulk_fail",
                        item = format!("{}/{}", item.item_type, item.id)
                    )
                );
            }
            if item.item_type == "spec" {
                print_staleness(ItemType::Spec, &item.staleness);
            }
        }
        println!(
            "{}",
            t!(
                "sdd.validate.bulk_summary",
                passed = passed,
                failed = failed,
                items = items.len()
            )
        );
    }

    let failed = items.iter().filter(|item| !item.valid).count();
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn apply_strict(mut issues: Vec<ValidationIssue>, strict: bool) -> Vec<ValidationIssue> {
    if !strict {
        return issues;
    }
    for issue in &mut issues {
        if issue.level == ValidationLevel::Warning {
            issue.level = ValidationLevel::Error;
        }
    }
    issues
}

fn error_report(message: String) -> ValidationReport {
    ValidationReport {
        valid: false,
        issues: vec![ValidationIssue {
            level: ValidationLevel::Error,
            path: "file".to_string(),
            message,
        }],
        summary: ValidationSummary {
            errors: 1,
            warnings: 0,
            info: 0,
        },
    }
}

fn summary_for_items(items: &[ValidationItem], allowed: &[ItemType]) -> serde_json::Value {
    let mut totals = SummaryCounts::default();
    let mut by_type = std::collections::BTreeMap::new();

    for item in items {
        totals.items += 1;
        if item.valid {
            totals.passed += 1;
        } else {
            totals.failed += 1;
        }
        let entry = by_type
            .entry(item.item_type.clone())
            .or_insert_with(SummaryCounts::default);
        entry.items += 1;
        if item.valid {
            entry.passed += 1;
        } else {
            entry.failed += 1;
        }
    }

    for allowed_type in allowed {
        let key = allowed_type.as_str().to_string();
        by_type.entry(key).or_insert_with(SummaryCounts::default);
    }

    serde_json::json!({
        "totals": totals,
        "byType": by_type
    })
}

fn empty_summary(include_changes: bool, include_specs: bool) -> serde_json::Value {
    let mut by_type = serde_json::Map::new();
    if include_changes {
        by_type.insert(
            "change".to_string(),
            serde_json::json!({"items": 0, "passed": 0, "failed": 0}),
        );
    }
    if include_specs {
        by_type.insert(
            "spec".to_string(),
            serde_json::json!({"items": 0, "passed": 0, "failed": 0}),
        );
    }
    serde_json::json!({
        "totals": { "items": 0, "passed": 0, "failed": 0 },
        "byType": by_type
    })
}

fn print_json(value: &serde_json::Value, compact: bool) -> Result<()> {
    if compact {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

#[derive(Default, Serialize)]
struct SummaryCounts {
    items: usize,
    passed: usize,
    failed: usize,
}

fn print_non_interactive_hint() {
    eprintln!("{}", t!("sdd.validate.non_interactive.line1"));
    eprintln!("{}", t!("sdd.validate.non_interactive.line2"));
    eprintln!("{}", t!("sdd.validate.non_interactive.line3"));
    eprintln!("{}", t!("sdd.validate.non_interactive.line4"));
    eprintln!("{}", t!("sdd.validate.non_interactive.line5"));
    eprintln!("{}", t!("sdd.validate.non_interactive.line6"));
}
