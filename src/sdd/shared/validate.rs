use crate::sdd::change::freeze::FREEZE_ARCHIVE_NAME;
use crate::sdd::project::config::{ArchiveConfig, BddConfig, load_required_config};
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::shared::discovery::{list_archived_changes, list_changes, list_specs};
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::shared::interactive::is_interactive;
use crate::sdd::shared::match_utils::nearest_matches;
use crate::sdd::spec::staleness::{StalenessEvaluator, StalenessInfo, evaluate_staleness};
use crate::sdd::spec::validation::{
    ChangeStage, ValidationIssue, ValidationLevel, ValidationReport, ValidationSummary,
    check_completeness_stage, check_dag_cycles, check_design_md, check_design_tasks_constraint,
    check_proposal_exists, check_proposal_frontmatter, check_tasks_completion, check_tasks_exists,
    determine_stage, has_spec_files, validate_change_delta_specs,
    validate_spec_content_with_frontmatter_and_bdd,
};
use anyhow::{Result, anyhow};
use inquire::Select;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn has_frozen_archive(root: &Path) -> bool {
    root.join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join("archive")
        .join(FREEZE_ARCHIVE_NAME)
        .exists()
}

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
    pub stage: Option<String>,
    pub no_interactive: bool,
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

fn parse_stage_override(value: Option<&str>) -> Option<ChangeStage> {
    match value?.to_lowercase().as_str() {
        "draft" => Some(ChangeStage::Draft),
        "spec" => Some(ChangeStage::Specified),
        "full" => Some(ChangeStage::Full),
        _ => None,
    }
}

pub fn run(args: ValidateArgs) -> Result<()> {
    let root = Path::new(".");
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    let archive_config = config.archive_config();
    let bdd_config = config.bdd.as_ref();

    let interactive = is_interactive(args.no_interactive);
    let type_override = normalize_type(args.item_type.as_deref());
    let stage_override = parse_stage_override(args.stage.as_deref());

    if args.all || args.changes || args.specs {
        let do_changes = args.all || args.changes;
        let do_specs = args.all || args.specs;
        run_bulk_validation(
            root,
            do_changes,
            do_specs,
            args.strict,
            args.json,
            args.compact_json,
            stage_override,
            &archive_config,
            bdd_config,
        )?;
        return Ok(());
    }

    if args.item.is_none() {
        if interactive {
            run_interactive_selector(
                root,
                args.strict,
                args.json,
                args.compact_json,
                stage_override,
                &archive_config,
                bdd_config,
            )?;
            return Ok(());
        }
        return Err(anyhow!(non_interactive_hint_message()));
    }

    let Some(item) = args.item.as_deref() else {
        return Err(anyhow!(non_interactive_hint_message()));
    };
    validate_direct(
        root,
        item,
        type_override,
        args.strict,
        args.json,
        args.compact_json,
        stage_override,
        &archive_config,
        bdd_config,
    )
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
    strict: bool,
    json: bool,
    compact_json: bool,
    stage_override: Option<ChangeStage>,
    archive_config: &ArchiveConfig,
    bdd_config: Option<&BddConfig>,
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
        run_bulk_validation(
            root,
            true,
            true,
            strict,
            json,
            compact_json,
            stage_override,
            archive_config,
            bdd_config,
        )?;
        return Ok(());
    }
    if choice == t!("sdd.validate.option_changes") {
        run_bulk_validation(
            root,
            true,
            false,
            strict,
            json,
            compact_json,
            stage_override,
            archive_config,
            bdd_config,
        )?;
        return Ok(());
    }
    if choice == t!("sdd.validate.option_specs") {
        run_bulk_validation(
            root,
            false,
            true,
            strict,
            json,
            compact_json,
            stage_override,
            archive_config,
            bdd_config,
        )?;
        return Ok(());
    }

    let changes = list_changes(root)?;
    let specs = list_specs(root)?;
    let archived_changes = list_archived_changes(root).unwrap_or_default();
    let mut items = Vec::new();
    items.extend(changes.iter().map(|id| format!("change/{id}")));
    items.extend(specs.iter().map(|id| format!("spec/{id}")));
    if items.is_empty() {
        return Err(anyhow!(t!("sdd.validate.no_items")));
    }
    let picked = Select::new(&t!("sdd.validate.pick_item"), items).prompt()?;
    let (item_type, id) = parse_prefixed_item(&picked)?;
    validate_by_type(
        root,
        item_type,
        &id,
        strict,
        json,
        compact_json,
        stage_override,
        &archived_changes,
        has_frozen_archive(root),
        archive_config,
        bdd_config,
    )
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

#[allow(clippy::too_many_arguments)]
fn validate_direct(
    root: &Path,
    item: &str,
    type_override: Option<ItemType>,
    strict: bool,
    json: bool,
    compact_json: bool,
    stage_override: Option<ChangeStage>,
    archive_config: &ArchiveConfig,
    bdd_config: Option<&BddConfig>,
) -> Result<()> {
    let changes = list_changes(root)?;
    let specs = list_specs(root)?;
    let archived_changes = list_archived_changes(root).unwrap_or_default();
    let is_change = changes.contains(&item.to_string());
    let is_spec = specs.contains(&item.to_string());

    // When --type change is specified, also accept directories that physically exist
    // even if not discovered (e.g., missing proposal.md — validation will report that).
    let change_dir_physical = root.join(LLMANSPEC_DIR_NAME).join("changes").join(item);
    let is_change_or_dir = is_change || change_dir_physical.is_dir();

    if let Some(ItemType::Change) = type_override
        && !is_change_or_dir
    {
        let suggestions = nearest_matches(item, &changes, 5);
        return Err(anyhow!(unknown_item_message(item, &suggestions)));
    }
    if let Some(ItemType::Spec) = type_override
        && !is_spec
    {
        let suggestions = nearest_matches(item, &specs, 5);
        return Err(anyhow!(unknown_item_message(item, &suggestions)));
    }

    let resolved_type = type_override.or(if is_change_or_dir {
        Some(ItemType::Change)
    } else if is_spec {
        Some(ItemType::Spec)
    } else {
        None
    });

    let Some(resolved_type) = resolved_type else {
        let suggestions = nearest_matches(item, &[changes, specs].concat(), 5);
        return Err(anyhow!(unknown_item_message(item, &suggestions)));
    };

    if type_override.is_none() && is_change_or_dir && is_spec {
        return Err(anyhow!(
            "{}\n{}",
            t!("sdd.validate.ambiguous_item", item = item),
            t!("sdd.validate.ambiguous_hint")
        ));
    }

    validate_by_type(
        root,
        resolved_type,
        item,
        strict,
        json,
        compact_json,
        stage_override,
        &archived_changes,
        has_frozen_archive(root),
        archive_config,
        bdd_config,
    )
}

fn compute_dag_issues_for_bulk(
    root: &Path,
    change_ids: &[String],
    archived_change_ids: &[String],
    has_frozen: bool,
) -> HashMap<String, Vec<ValidationIssue>> {
    let mut frontmatters = Vec::new();
    for id in change_ids {
        let change_dir = root.join(LLMANSPEC_DIR_NAME).join("changes").join(id);
        let (_, fm) =
            check_proposal_frontmatter(&change_dir, change_ids, archived_change_ids, has_frozen);
        frontmatters.push((id.clone(), fm));
    }
    check_dag_cycles(&frontmatters)
}

fn compute_dag_issues_for_single(
    root: &Path,
    change_id: &str,
    all_change_ids: &[String],
    archived_change_ids: &[String],
    has_frozen: bool,
) -> Vec<ValidationIssue> {
    let all_dag_issues =
        compute_dag_issues_for_bulk(root, all_change_ids, archived_change_ids, has_frozen);
    all_dag_issues.get(change_id).cloned().unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn validate_change_full(
    change_dir: &Path,
    all_change_ids: &[String],
    archived_change_ids: &[String],
    has_frozen: bool,
    strict: bool,
    stage_override: Option<ChangeStage>,
    dag_issues: &[ValidationIssue],
    archive_config: &ArchiveConfig,
) -> ValidationReport {
    let stage = stage_override.unwrap_or_else(|| determine_stage(change_dir));
    let mut issues = Vec::new();

    // Validate consistency when stage is forced via --stage
    if let Some(s) = stage_override {
        match s {
            ChangeStage::Draft => {
                if !change_dir.join("proposal.md").exists() {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: "proposal.md".to_string(),
                        message: "Stage forced to 'draft' but proposal.md is missing".to_string(),
                    });
                }
            }
            ChangeStage::Specified => {
                if !has_spec_files(&change_dir.join("specs")) {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: "specs".to_string(),
                        message: "Stage forced to 'spec' but specs/ is missing or empty"
                            .to_string(),
                    });
                }
            }
            ChangeStage::Full => {
                if !change_dir.join("tasks.md").exists() {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: "tasks.md".to_string(),
                        message: "Stage forced to 'full' but tasks.md is missing".to_string(),
                    });
                }
            }
            ChangeStage::Designed => {}
        }
    }

    // Stage-agnostic: always validate proposal existence and frontmatter
    issues.extend(check_proposal_exists(change_dir));
    issues.extend(
        check_proposal_frontmatter(change_dir, all_change_ids, archived_change_ids, has_frozen).0,
    );

    // Non-draft stages must have valid delta specs
    if stage != ChangeStage::Draft {
        let delta_report = validate_change_delta_specs(change_dir, strict);
        issues.extend(delta_report.issues);
    }

    // tasks.md without design.md is inconsistent at any stage
    issues.extend(check_design_tasks_constraint(change_dir));

    // Full stage: all artifacts present, validate task completion
    if stage == ChangeStage::Full {
        issues.extend(check_tasks_exists(change_dir));
        issues.extend(check_tasks_completion(
            change_dir,
            all_change_ids,
            archived_change_ids,
            has_frozen,
            archive_config,
        ));
        issues.extend(check_design_md(change_dir));
    }

    // Stage hint (always Info — stage reflects effective stage)
    issues.extend(check_completeness_stage(change_dir, strict, stage_override));

    issues.extend(dag_issues.to_vec());

    crate::sdd::spec::validation::build_report(issues, strict)
}

/// Merge feature_refs paths into the frontmatter's valid_scope for staleness checks.
/// This ensures that changes to .feature files are treated as spec-adjacent changes.
fn merge_feature_refs_into_scope(
    validation: &crate::sdd::spec::validation::SpecValidation,
    bdd_config: Option<&BddConfig>,
    _root: &Path,
) -> Option<crate::sdd::spec::validation::SpecFrontmatter> {
    let _ = bdd_config?;
    let frontmatter = validation.frontmatter.as_ref()?;
    let feature_refs = validation.feature_refs.as_ref()?;

    if feature_refs.is_empty() {
        return None;
    }

    let mut merged_scope = frontmatter.valid_scope.clone();
    for path in feature_refs {
        if !merged_scope.contains(path) {
            merged_scope.push(path.clone());
        }
    }

    Some(crate::sdd::spec::validation::SpecFrontmatter {
        valid_scope: merged_scope,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_by_type(
    root: &Path,
    item_type: ItemType,
    id: &str,
    strict: bool,
    json: bool,
    compact_json: bool,
    stage_override: Option<ChangeStage>,
    archived_change_ids: &[String],
    has_frozen: bool,
    archive_config: &ArchiveConfig,
    bdd_config: Option<&BddConfig>,
) -> Result<()> {
    let start = Instant::now();
    let (report, staleness) = match item_type {
        ItemType::Change => {
            validate_sdd_id(id, "change")?;
            let change_dir = root.join(LLMANSPEC_DIR_NAME).join("changes").join(id);
            let change_ids = list_changes(root).unwrap_or_default();
            let dag_issues = compute_dag_issues_for_single(
                root,
                id,
                &change_ids,
                archived_change_ids,
                has_frozen,
            );
            let report = validate_change_full(
                &change_dir,
                &change_ids,
                archived_change_ids,
                has_frozen,
                strict,
                stage_override,
                &dag_issues,
                archive_config,
            );
            (report, StalenessInfo::not_applicable())
        }
        ItemType::Spec => {
            validate_sdd_id(id, "spec")?;
            let spec_path = root
                .join(LLMANSPEC_DIR_NAME)
                .join("specs")
                .join(id)
                .join(SPEC_FILE);
            match fs::read_to_string(&spec_path) {
                Ok(content) => {
                    let validation = validate_spec_content_with_frontmatter_and_bdd(
                        &spec_path,
                        &content,
                        strict,
                        Some(root),
                        bdd_config,
                    );
                    // Merge feature_refs paths into staleness scope so that
                    // changes to .feature files are treated as spec-adjacent changes.
                    let merged_frontmatter =
                        merge_feature_refs_into_scope(&validation, bdd_config, root);
                    let staleness = evaluate_staleness(
                        root,
                        id,
                        &spec_path,
                        merged_frontmatter
                            .as_ref()
                            .or(validation.frontmatter.as_ref()),
                    );
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
        return Err(anyhow!("validation failed"));
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
        // Even when the change is valid, surface INFO/WARNING-level hints
        // (e.g. the stage hint for a draft, or a missing optional artifact).
        // These are guidance, not errors, and must not be swallowed by the
        // valid short-circuit (see r45).
        let guidance_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|issue| {
                issue.level == ValidationLevel::Info || issue.level == ValidationLevel::Warning
            })
            .collect();
        for issue in &guidance_issues {
            let label = match issue.level {
                ValidationLevel::Warning => "WARNING",
                ValidationLevel::Info => "INFO",
                ValidationLevel::Error => "ERROR",
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
    print_next_steps(item_type, &report.issues);
}

fn print_next_steps(item_type: ItemType, issues: &[ValidationIssue]) {
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

    // BDD-specific hints when feature_refs issues are detected
    let has_bdd_issues = issues.iter().any(|i| {
        i.message.contains(".feature")
            || i.message.contains("gherkin")
            || i.message.contains("bdd")
            || i.message.contains("feature_refs")
    });
    if has_bdd_issues {
        eprintln!("{}", t!("sdd.validate.bdd_next_step_feature"));
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

#[allow(clippy::too_many_arguments)]
fn run_bulk_validation(
    root: &Path,
    validate_changes: bool,
    validate_specs: bool,
    strict: bool,
    json: bool,
    compact_json: bool,
    stage_override: Option<ChangeStage>,
    archive_config: &ArchiveConfig,
    bdd_config: Option<&BddConfig>,
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

    let archived_changes = list_archived_changes(root).unwrap_or_default();
    let frozen = has_frozen_archive(root);

    // Pre-pass: compute DAG cycle issues for all changes
    let dag_issues_map = if validate_changes {
        compute_dag_issues_for_bulk(root, &changes, &archived_changes, frozen)
    } else {
        HashMap::new()
    };

    let all_change_ids: Vec<String> = changes.clone();

    for id in changes {
        let start = Instant::now();
        validate_sdd_id(&id, "change")?;
        let change_dir = root.join(LLMANSPEC_DIR_NAME).join("changes").join(&id);
        let dag_issues = dag_issues_map.get(&id).cloned().unwrap_or_default();
        let report = validate_change_full(
            &change_dir,
            &all_change_ids,
            &archived_changes,
            frozen,
            strict,
            stage_override,
            &dag_issues,
            archive_config,
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
    let staleness_evaluator = StalenessEvaluator::new(root);
    for id in specs {
        let start = Instant::now();
        validate_sdd_id(&id, "spec")?;
        let spec_path = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join(&id)
            .join(SPEC_FILE);
        match fs::read_to_string(&spec_path) {
            Ok(content) => {
                let validation = validate_spec_content_with_frontmatter_and_bdd(
                    &spec_path,
                    &content,
                    strict,
                    Some(root),
                    bdd_config,
                );
                let merged_frontmatter =
                    merge_feature_refs_into_scope(&validation, bdd_config, root);
                let staleness = staleness_evaluator.evaluate(
                    &id,
                    &spec_path,
                    merged_frontmatter
                        .as_ref()
                        .or(validation.frontmatter.as_ref()),
                    None,
                );
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
                for issue in &item.issues {
                    let label = match issue.level {
                        ValidationLevel::Error => "ERROR",
                        ValidationLevel::Warning => "WARNING",
                        ValidationLevel::Info => "INFO",
                    };
                    eprintln!(
                        "  {}",
                        t!(
                            "sdd.validate.issue_line",
                            label = label,
                            path = issue.path,
                            message = issue.message
                        )
                    );
                }
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
        return Err(anyhow!("validation failed"));
    }
    Ok(())
}

fn unknown_item_message(item: &str, suggestions: &[String]) -> String {
    let mut msg = t!("sdd.validate.unknown_item", item = item).to_string();
    if !suggestions.is_empty() {
        msg.push('\n');
        msg.push_str(&t!(
            "sdd.validate.did_you_mean",
            items = suggestions.join(", ")
        ));
    }
    msg
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

fn non_interactive_hint_message() -> String {
    [
        t!("sdd.validate.non_interactive.line1"),
        t!("sdd.validate.non_interactive.line2"),
        t!("sdd.validate.non_interactive.line3"),
        t!("sdd.validate.non_interactive.line4"),
        t!("sdd.validate.non_interactive.line5"),
        t!("sdd.validate.non_interactive.line6"),
    ]
    .join("\n")
}
