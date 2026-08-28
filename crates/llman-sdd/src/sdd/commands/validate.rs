use crate::sdd::change::freeze::FREEZE_ARCHIVE_NAME;
use crate::sdd::project::config::{ArchiveConfig, BddConfig, load_required_config};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::{
    list_archived_changes, list_changes, list_specs, resolve_change_dir,
};
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::shared::interactive::is_interactive;
use crate::sdd::shared::json::print_json;
use crate::sdd::shared::match_utils::nearest_matches;
use crate::sdd::shared::types::{ItemType, normalize_type};
use crate::sdd::spec::staleness::{StalenessEvaluator, StalenessInfo, evaluate_staleness};
use crate::sdd::spec::validation::{
    ChangeStage, ValidationIssue, ValidationLevel, ValidationReport, ValidationSummary,
    check_completeness_stage, check_dag_cycles, check_design_md, check_design_tasks_constraint,
    check_proposal_exists, check_proposal_frontmatter, check_tasks_completion, check_tasks_exists,
    determine_stage, has_spec_files, validate_spec_content,
};
use anyhow::{Result, anyhow};
use inquire::Select;
use serde::Serialize;
use std::collections::HashMap;
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
pub(crate) struct ValidateArgs {
    pub(crate) item: Option<String>,
    pub(crate) all: bool,
    pub(crate) changes: bool,
    pub(crate) specs: bool,
    pub(crate) item_type: Option<String>,
    pub(crate) strict: bool,
    pub(crate) json: bool,
    pub(crate) compact_json: bool,
    pub(crate) stage: Option<String>,
    pub(crate) no_interactive: bool,
    /// Run the BDD check command after fast validation (BDD-on spec only).
    /// Default: enabled when bdd.run_command is configured.
    pub(crate) check: bool,
    /// Skip BDD runner execution even when bdd.run_command is configured.
    pub(crate) no_check: bool,
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
    /// r112: true when the change id was resolved via a prefix match. Only
    /// meaningful for change items; always false for specs / interactive picks.
    #[serde(rename = "matchedViaPrefix", default)]
    matched_via_prefix: bool,
}

fn parse_stage_override(value: Option<&str>) -> Option<ChangeStage> {
    match value?.to_lowercase().as_str() {
        "draft" => Some(ChangeStage::Draft),
        // Legacy "spec" / "specified" inputs map to Designed (Specified stage
        // is removed under the unified three-state flow, r93).
        "spec" | "specified" => Some(ChangeStage::Designed),
        "designed" => Some(ChangeStage::Designed),
        "full" => Some(ChangeStage::Full),
        _ => None,
    }
}

/// Compute effective check mode: when BDD is configured, auto-run by default;
/// `--no-check` explicitly disables it. `--check` is accepted as a no-op alias.
/// When BDD is off and `--check` is passed, emit an INFO later via validate.
/// Returns `(effective_check_mode, deprecation_info_needed)`.
/// `deprecation_info_needed` is true when `--check` was passed but BDD is off.
fn resolve_check_mode(bdd_configured: bool, check_flag: bool, no_check: bool) -> (bool, bool) {
    if no_check {
        return (false, false);
    }
    if bdd_configured {
        return (true, false); // auto-run; --check is a harmless no-op alias
    }
    // BDD-off: --check is misleading, flag for INFO message.
    (false, check_flag)
}

/// Run validation against the project rooted at `root`.
///
/// Most internal helpers already take `root: &Path`; this entrypoint threads
/// it explicitly so that in-process callers (e.g. `run_finalize`, `run_checkpoint`)
/// can validate a TempDir fixture without mutating process cwd. The CLI boundary
/// in `command.rs` passes `Path::new(".")` to preserve the cwd-implicit behavior.
pub(crate) fn run(root: &Path, args: ValidateArgs) -> Result<()> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    let archive_config = config.archive_config();
    let bdd_config = config.bdd.as_ref();
    let locale = config.locale.clone();

    // Managed skill metadata must match project BDD mode before any other gate.
    crate::sdd::project::skill_consistency::check_installed_skills_bdd_mode(root, &config)?;

    let interactive = is_interactive(args.no_interactive);
    let type_override = normalize_type(args.item_type.as_deref());
    let stage_override = parse_stage_override(args.stage.as_deref());
    let (check_mode, check_deprecated) =
        resolve_check_mode(bdd_config.is_some(), args.check, args.no_check);

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
            &locale,
            check_mode,
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
                &locale,
                check_mode,
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
        &locale,
        check_mode,
        check_deprecated,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_interactive_selector(
    root: &Path,
    strict: bool,
    json: bool,
    compact_json: bool,
    stage_override: Option<ChangeStage>,
    archive_config: &ArchiveConfig,
    bdd_config: Option<&BddConfig>,
    locale: &str,
    check_mode: bool,
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
            locale,
            check_mode,
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
            locale,
            check_mode,
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
            locale,
            check_mode,
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
        locale,
        check_mode,
        false, // interactive user can't pass --check
        false, // interactive pick is never a prefix match
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
    locale: &str,
    check_mode: bool,
    check_deprecated: bool,
) -> Result<()> {
    let specs = list_specs(root)?;
    let is_spec = specs.contains(&item.to_string());
    let mut resolved_change_id: Option<String> = None;
    // Whether the change id was resolved via a prefix match (r112 hint + JSON
    // `matchedViaPrefix`). Only meaningful when resolving a change.
    let mut matched_via_prefix = false;

    // When --type change is specified, use prefix-aware resolution.
    // When no type, exact spec match takes priority (cli spec r112) so a spec
    // id is not hijacked by a change whose id starts with it.
    match type_override {
        Some(ItemType::Change) => {
            let resolved = crate::sdd::shared::discovery::resolve_change_id(root, item)?;
            matched_via_prefix = resolved.via_prefix;
            resolved_change_id = Some(resolved.id);
        }
        Some(ItemType::Spec) => {}
        None => {
            if !is_spec
                && let Ok(resolved) = crate::sdd::shared::discovery::resolve_change_id(root, item)
            {
                matched_via_prefix = resolved.via_prefix;
                resolved_change_id = Some(resolved.id);
            }
        }
    }

    let archived_changes = list_archived_changes(root).unwrap_or_default();
    let is_change = resolved_change_id.is_some();

    // Spec type-override: still do exact match
    if let Some(ItemType::Spec) = type_override
        && !is_spec
    {
        let suggestions = nearest_matches(item, &specs, 5);
        return Err(anyhow!(unknown_item_message(item, &suggestions)));
    }

    let resolved_type = type_override.or(if is_change {
        Some(ItemType::Change)
    } else if is_spec {
        Some(ItemType::Spec)
    } else {
        None
    });

    let Some(resolved_type) = resolved_type else {
        let mut candidates = Vec::new();
        candidates.extend(list_changes(root)?);
        candidates.extend(specs);
        let suggestions = nearest_matches(item, &candidates, 5);
        return Err(anyhow!(unknown_item_message(item, &suggestions)));
    };

    if type_override.is_none() && is_change && is_spec {
        return Err(anyhow!(
            "{}\n{}",
            t!("sdd.validate.ambiguous_item", item = item),
            t!("sdd.validate.ambiguous_hint")
        ));
    }

    let resolved_id = resolved_change_id.as_deref().unwrap_or(item);
    // r112: emit the "'input' -> 'resolved' (prefix match)" hint to stderr for
    // human output when a change id was resolved via a prefix.
    if matched_via_prefix && is_change && !json {
        eprintln!(
            "{}",
            t!(
                "sdd.prefix_match_hint",
                input = item,
                resolved = resolved_id
            )
        );
    }
    validate_by_type(
        root,
        resolved_type,
        resolved_id,
        strict,
        json,
        compact_json,
        stage_override,
        &archived_changes,
        has_frozen_archive(root),
        archive_config,
        bdd_config,
        locale,
        check_mode,
        check_deprecated,
        matched_via_prefix,
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
        let Ok(change_dir) = resolve_change_dir(root, id) else {
            continue;
        };
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
    bdd_on: bool,
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
            ChangeStage::Designed => {
                // Designed requires design.md + tasks.md but no attach binding.
                if !change_dir.join("design.md").exists() {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: "design.md".to_string(),
                        message: "Stage forced to 'designed' but design.md is missing".to_string(),
                    });
                }
                if !change_dir.join("tasks.md").exists() {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: "tasks.md".to_string(),
                        message: "Stage forced to 'designed' but tasks.md is missing".to_string(),
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
        }
    }

    // Stage-agnostic: always validate proposal existence and frontmatter
    issues.extend(check_proposal_exists(change_dir));
    let (fm_issues, frontmatter) =
        check_proposal_frontmatter(change_dir, all_change_ids, archived_change_ids, has_frozen);
    issues.extend(fm_issues);

    if bdd_on {
        issues.extend(check_bdd_on_change_gates(change_dir, Some(&frontmatter)));
    }

    // Locked-rule integrity on the strict change path (spec-format r135):
    // only when bound to a non-default branch.
    if let Some(root) = crate::sdd::change::specs_landing::repo_root_from_change_dir(change_dir)
        && frontmatter
            .base_sha
            .as_deref()
            .is_some_and(|b| !b.trim().is_empty())
    {
        let acked = frontmatter.rules_edit_acked;
        let base = frontmatter.base_sha.clone().unwrap_or_default();
        for issue in crate::sdd::change::lock_gate::check(root, base.trim(), acked) {
            issues.push(issue);
        }
    }

    // Non-draft stages: leftover `changes/<id>/specs/` are warned (live tree is SSOT).
    if stage != ChangeStage::Draft && has_spec_files(&change_dir.join("specs")) {
        issues.push(ValidationIssue {
            level: ValidationLevel::Warning,
            path: "specs".to_string(),
            message: "leftover change specs directory; live llmanspec/specs is SSOT — run toon2features and delete leftovers before archive".to_string(),
        });
    }

    // tasks.md without design.md is inconsistent at any stage
    issues.extend(check_design_tasks_constraint(change_dir));

    // Full stage: all artifacts present, validate task completion
    if stage == ChangeStage::Full {
        issues.extend(check_tasks_exists(change_dir));
        issues.extend(check_tasks_completion(change_dir, archive_config));
        issues.extend(check_design_md(change_dir));
        // Specs landing soft gate (WARNING): Full but no live specs diff / skip.
        if let Some(root) = crate::sdd::change::specs_landing::repo_root_from_change_dir(change_dir)
        {
            let landing =
                crate::sdd::change::specs_landing::evaluate_specs_landing(root, change_dir);
            if !landing.ready_to_implement {
                let msg = landing.not_ready_message(
                    change_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("<change>"),
                );
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    path: "proposal.md".to_string(),
                    message: msg,
                });
            }
            if let Some(dirty) =
                crate::sdd::change::specs_landing::warn_dirty_specs_on_default_branch(root)
            {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    path: "llmanspec/specs".to_string(),
                    message: dirty,
                });
            }
        }
    }

    // Stage hint (always Info — stage reflects effective stage)
    issues.extend(check_completeness_stage(
        change_dir,
        strict,
        stage_override,
        bdd_on,
    ));

    issues.extend(dag_issues.to_vec());

    crate::sdd::spec::validation::build_report(issues, strict)
}

/// BDD-on change gates: reject legacy feature_delta; require Git binding for full stage.
fn check_bdd_on_change_gates(
    change_dir: &Path,
    frontmatter: Option<&crate::sdd::spec::validation::ProposalFrontmatter>,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let specs_dir = change_dir.join("specs");
    if specs_dir.is_dir() {
        for delta in walk_legacy_feature_deltas(&specs_dir) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: delta,
                message: "legacy feature_delta is a migration blocker under Git-native BDD-on; edit live llmanspec/specs/**/*.feature on the feature branch instead".to_string(),
            });
        }
    }

    let has_binding = frontmatter
        .map(|fm| {
            fm.branch
                .as_ref()
                .map(|b| !b.trim().is_empty())
                .unwrap_or(false)
                && fm
                    .base_sha
                    .as_ref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false)
        })
        .unwrap_or(false);
    if !has_binding {
        issues.push(ValidationIssue {
            level: ValidationLevel::Info,
            path: "proposal.md".to_string(),
            message: "BDD-on change has no Git binding; run `llman sdd change attach <id>` on a non-default feature branch before checkpoint/archive".to_string(),
        });
    }
    issues
}

fn walk_legacy_feature_deltas(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_legacy_feature_deltas(&path));
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".feature.delta.toon") || name == "feature.delta.toon" {
            out.push(path.display().to_string());
        }
    }
    out
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
    locale: &str,
    check_mode: bool,
    check_deprecated: bool,
    matched_via_prefix: bool,
) -> Result<()> {
    let start = Instant::now();
    let (report, staleness) = match item_type {
        ItemType::Change => {
            validate_sdd_id(id, "change")?;
            let change_dir = resolve_change_dir(root, id)?;
            let change_ids = list_changes(root)?;
            let dag_issues = compute_dag_issues_for_single(
                root,
                id,
                &change_ids,
                archived_change_ids,
                has_frozen,
            );
            let mut report = validate_change_full(
                &change_dir,
                &change_ids,
                archived_change_ids,
                has_frozen,
                strict,
                stage_override,
                &dag_issues,
                archive_config,
                bdd_config.is_some(),
            );
            // Common validate path: fail closed on main-library req_id collisions.
            report
                .issues
                .extend(crate::sdd::spec::req_registry::global_req_id_uniqueness_issues(root));
            report.valid = !report
                .issues
                .iter()
                .any(|issue| issue.level == ValidationLevel::Error);
            (report, StalenessInfo::not_applicable())
        }
        ItemType::Spec => {
            validate_sdd_id(id, "spec")?;
            let specs_root = root.join(LLMANSPEC_DIR_NAME).join("specs");
            match crate::sdd::spec::validation::resolve_spec_file(&specs_root, id)
                .and_then(|p| fs::read_to_string(&p).map_err(|e| anyhow!(e)))
            {
                Ok(content) => {
                    // `content` is the capability's single-track `.feature` text;
                    // staleness scope comes from the `# scope:` header via the
                    // parsed doc's valid_scope (r133).
                    let spec_path = specs_root.join(id);
                    let validation = validate_spec_content(
                        &spec_path.join("spec.feature"),
                        &content,
                        strict,
                        crate::sdd::spec::validation::SpecValidateCtx {
                            project_root: Some(root),
                            bdd_config,
                            locale: Some(locale),
                            check_mode,
                            full_mode_cache: None,
                        },
                    );
                    // Staleness scope: the capability .feature header's
                    // `# scope:` is the single source of truth (r133).
                    let staleness_frontmatter = validation.frontmatter.clone();
                    let staleness =
                        evaluate_staleness(root, id, &spec_path, staleness_frontmatter.as_ref());
                    let mut issues = validation.report.issues.clone();
                    issues.extend(crate::sdd::spec::req_registry::global_req_id_uniqueness_issues_for_capability(
                        root, id,
                    ));
                    issues.extend(apply_strict(staleness.issues, strict));
                    let valid = !issues
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

    let mut report = report;
    // Emit INFO when --check is passed but BDD is not configured.
    if check_deprecated && item_type == ItemType::Spec {
        report.issues.push(ValidationIssue {
            level: ValidationLevel::Info,
            path: "--check".to_string(),
            message: t!("sdd.validate.check_deprecated_no_bdd").to_string(),
        });
    }

    if json {
        let items = vec![ValidationItem {
            id: id.to_string(),
            item_type: item_type.as_str().to_string(),
            valid: report.valid,
            issues: report.issues.clone(),
            duration_ms,
            staleness: staleness.clone(),
            matched_via_prefix,
        }];
        let summary = summary_for_items(&items, &[item_type]);
        let output = serde_json::json!({
            "items": items,
            "summary": summary,
            "version": "1.0"
        });
        print_json(&output, compact_json)?;
    } else {
        print_single_report(item_type, id, &report, &staleness, bdd_config.is_some());
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
    bdd_on: bool,
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
    print_next_steps(item_type, &report.issues, bdd_on);
}

fn print_next_steps(item_type: ItemType, issues: &[ValidationIssue], _bdd_on: bool) {
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

    // BDD-specific hints when feature-as-spec issues are detected
    let has_bdd_issues = issues.iter().any(|i| {
        i.message.contains(".feature") || i.message.contains("gherkin") || i.message.contains("BDD")
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
    locale: &str,
    check_mode: bool,
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

    let global_req_issues = if validate_specs || validate_changes {
        crate::sdd::spec::req_registry::global_req_id_uniqueness_issues(root)
    } else {
        Vec::new()
    };

    for id in changes {
        let start = Instant::now();
        validate_sdd_id(&id, "change")?;
        let change_dir = match resolve_change_dir(root, &id) {
            Ok(p) => p,
            Err(err) => {
                items.push(ValidationItem {
                    id,
                    item_type: "change".to_string(),
                    valid: false,
                    issues: vec![ValidationIssue {
                        level: ValidationLevel::Error,
                        path: "discovery".to_string(),
                        message: err.to_string(),
                    }],
                    duration_ms: start.elapsed().as_millis(),
                    staleness: StalenessInfo::not_applicable(),
                    matched_via_prefix: false,
                });
                continue;
            }
        };
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
            bdd_config.is_some(),
        );
        items.push(ValidationItem {
            id,
            item_type: "change".to_string(),
            valid: report.valid,
            issues: report.issues,
            duration_ms: start.elapsed().as_millis(),
            staleness: StalenessInfo::not_applicable(),
            matched_via_prefix: false,
        });
    }
    // When validating changes without specs, still surface main-library req_id debt once.
    if validate_changes && !validate_specs && !global_req_issues.is_empty() {
        items.push(ValidationItem {
            id: "_global_req_id".to_string(),
            item_type: "spec".to_string(),
            valid: false,
            issues: global_req_issues.clone(),
            duration_ms: 0,
            staleness: StalenessInfo::not_applicable(),
            matched_via_prefix: false,
        });
    }
    let staleness_evaluator = StalenessEvaluator::new(root);
    let mut full_mode_cache = crate::sdd::spec::validation::FullModeCache::new();
    for id in specs {
        let start = Instant::now();
        validate_sdd_id(&id, "spec")?;
        let specs_root = root.join(LLMANSPEC_DIR_NAME).join("specs");
        let loaded =
            crate::sdd::spec::validation::resolve_spec_file(&specs_root, &id).and_then(|p| {
                fs::read_to_string(&p)
                    .map(|c| (p, c))
                    .map_err(|e| anyhow!(e))
            });
        match loaded {
            Ok((spec_path, content)) => {
                let validation = validate_spec_content(
                    &spec_path,
                    &content,
                    strict,
                    crate::sdd::spec::validation::SpecValidateCtx {
                        project_root: Some(root),
                        bdd_config,
                        locale: Some(locale),
                        check_mode,
                        full_mode_cache: if check_mode {
                            Some(&mut full_mode_cache)
                        } else {
                            None
                        },
                    },
                );
                let staleness_frontmatter = validation.frontmatter.clone();
                let staleness = staleness_evaluator.evaluate(
                    &id,
                    spec_path.parent().unwrap_or(&spec_path),
                    staleness_frontmatter.as_ref(),
                    None,
                );
                let mut issues = validation.report.issues;
                issues.extend(
                    global_req_issues
                        .iter()
                        .filter(|issue| issue.message.contains(&id))
                        .cloned(),
                );
                issues.extend(apply_strict(staleness.issues, strict));
                let valid = !issues
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
                    matched_via_prefix: false,
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
                    matched_via_prefix: false,
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

#[derive(Default, Serialize)]
struct SummaryCounts {
    items: usize,
    passed: usize,
    failed: usize,
}

fn non_interactive_hint_message() -> String {
    crate::sdd::shared::interactive::non_interactive_hint_message(
        t!("sdd.validate.non_interactive.line1").to_string(),
        &[
            t!("sdd.validate.non_interactive.line2").to_string(),
            t!("sdd.validate.non_interactive.line3").to_string(),
            t!("sdd.validate.non_interactive.line4").to_string(),
            t!("sdd.validate.non_interactive.line5").to_string(),
        ],
    )
}
