use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::shared::interactive::is_interactive;
use crate::sdd::shared::tasks;
use crate::sdd::spec::backend::{BACKEND, SpecBackend};
use crate::sdd::spec::staleness::evaluate_staleness_with_override;
use crate::sdd::spec::validation::{
    ValidationIssue, ValidationLevel, validate_spec_content_with_frontmatter,
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use inquire::Text;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ArchiveArgs {
    pub change: Option<String>,
    pub skip_specs: bool,
    pub dry_run: bool,
    pub force: bool,
    pub no_interactive: bool,
}

#[derive(Default)]
struct ApplyCounts {
    added: usize,
    modified: usize,
    removed: usize,
    renamed: usize,
}

struct SpecUpdate {
    capability: String,
    source: PathBuf,
    target: PathBuf,
    target_exists: bool,
}

pub fn run(args: ArchiveArgs) -> Result<()> {
    run_with_root(Path::new("."), args)
}

fn run_with_root(root: &Path, args: ArchiveArgs) -> Result<()> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    let archive_config = config.archive_config();

    let change_name = args
        .change
        .as_ref()
        .ok_or_else(|| anyhow!(t!("sdd.archive.change_required")))?;
    validate_sdd_id(change_name, "change")?;
    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");
    let change_dir = changes_dir.join(change_name);

    if !change_dir.exists() {
        return Err(anyhow!(t!(
            "sdd.archive.change_not_found",
            id = change_name
        )));
    }

    if !args.force {
        let tasks_path = change_dir.join("tasks.md");
        if let Some(report) = tasks::parse_tasks_file(&tasks_path)? {
            if report.pending > 0 {
                eprintln!(
                    "{}",
                    t!("sdd.archive.task_gate_blocked", pending = report.pending)
                );
                for item in &report.items {
                    if matches!(item.status, tasks::TaskStatus::Pending) {
                        eprintln!("{}", t!("sdd.archive.task_gate_item", task = item.text));
                    }
                }
                eprintln!("{}", t!("sdd.archive.task_gate_options"));
                return Err(anyhow!("archive blocked by unchecked tasks"));
            }

            if let Some(min_ratio) = archive_config.min_completion_ratio() {
                let actual = report.completion_ratio();
                if actual < min_ratio {
                    let ratio_pct = (actual * 100.0) as u32;
                    let min_pct = (min_ratio * 100.0) as u32;
                    return Err(anyhow!(
                        "{}",
                        t!(
                            "sdd.archive.task_completion_low",
                            ratio = ratio_pct,
                            min = min_pct
                        )
                    ));
                }
            }
        }
    }

    if !args.skip_specs {
        let validate_specs = !args.force;
        let interactive = is_interactive(args.no_interactive);
        let updates = find_spec_updates(&change_dir, root)?;
        if !updates.is_empty() {
            let prepared =
                prepare_updates(&updates, change_name, root, validate_specs, interactive)?;
            if args.dry_run {
                print_dry_run_specs(&prepared);
            } else {
                write_updates(&prepared)?;
            }
        }
    }

    let archive_dir = changes_dir.join("archive");
    let archive_name = format!("{}-{}", archive_date(), change_name);
    let archive_path = archive_dir.join(&archive_name);

    if args.dry_run {
        print_archive_move(&change_dir, &archive_path);
        return Ok(());
    }

    fs::create_dir_all(&archive_dir)?;
    match fs::rename(&change_dir, &archive_path) {
        Ok(()) => {}
        Err(e)
            if e.kind() == ErrorKind::AlreadyExists
                || e.kind() == ErrorKind::DirectoryNotEmpty
                || archive_path.exists() =>
        {
            return Err(anyhow!(t!(
                "sdd.archive.archive_exists",
                name = archive_name
            )));
        }
        Err(e) => return Err(e.into()),
    }
    println!(
        "{}",
        t!(
            "sdd.archive.archived",
            change = change_name,
            archive = archive_name
        )
    );

    Ok(())
}

fn find_spec_updates(change_dir: &Path, root: &Path) -> Result<Vec<SpecUpdate>> {
    let mut updates = Vec::new();
    let change_specs_dir = change_dir.join("specs");
    if !change_specs_dir.exists() {
        return Ok(updates);
    }

    for entry in fs::read_dir(change_specs_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let capability = entry.file_name().to_string_lossy().to_string();
        let source = entry.path().join(SPEC_FILE);
        if !source.exists() {
            continue;
        }
        let target = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join(&capability)
            .join(SPEC_FILE);
        let target_exists = target.exists();
        updates.push(SpecUpdate {
            capability,
            source,
            target,
            target_exists,
        });
    }

    Ok(updates)
}

fn prepare_updates(
    updates: &[SpecUpdate],
    change_name: &str,
    root: &Path,
    validate_specs: bool,
    interactive: bool,
) -> Result<Vec<(SpecUpdate, String, ApplyCounts)>> {
    let mut prepared = Vec::new();
    for update in updates {
        let built = build_updated_spec(update, change_name, interactive)?;
        if validate_specs {
            validate_rebuilt_spec(update, &built.0, root)?;
        }
        prepared.push((clone_update(update), built.0, built.1));
    }
    Ok(prepared)
}

fn validate_rebuilt_spec(update: &SpecUpdate, content: &str, root: &Path) -> Result<()> {
    let validation = validate_spec_content_with_frontmatter(&update.target, content, true);
    let mut issues = validation.report.issues;

    if let Some(frontmatter) = validation.frontmatter.as_ref() {
        let staleness = evaluate_staleness_with_override(
            root,
            &update.capability,
            &update.target,
            Some(frontmatter),
            Some(true),
        );
        issues.extend(apply_strict_levels(staleness.issues));
    }

    if issues
        .iter()
        .any(|issue| issue.level == ValidationLevel::Error)
    {
        let details = format_issues(&issues);
        return Err(anyhow!(t!(
            "sdd.archive.rebuilt_invalid",
            spec = update.capability,
            errors = details
        )));
    }

    Ok(())
}

fn apply_strict_levels(mut issues: Vec<ValidationIssue>) -> Vec<ValidationIssue> {
    for issue in &mut issues {
        if issue.level == ValidationLevel::Warning {
            issue.level = ValidationLevel::Error;
        }
    }
    issues
}

fn format_issues(issues: &[ValidationIssue]) -> String {
    issues
        .iter()
        .map(|issue| format!("{}: {}", issue.path, issue.message))
        .collect::<Vec<_>>()
        .join("; ")
}

fn build_updated_spec(
    update: &SpecUpdate,
    change_name: &str,
    interactive: bool,
) -> Result<(String, ApplyCounts)> {
    let backend = &BACKEND;
    let delta_content = fs::read_to_string(&update.source)?;
    let delta = backend.parse_delta_spec(
        &delta_content,
        &format!("delta spec `{}` during archive merge", update.capability),
    )?;

    let delta_count = delta.ops.len();
    if delta_count == 0 {
        return Err(anyhow!(t!(
            "sdd.archive.no_deltas",
            spec = update.capability
        )));
    }

    if !update.target_exists {
        let has_non_add = delta
            .ops
            .iter()
            .any(|op| !op.op.trim().eq_ignore_ascii_case("add_requirement"));
        if has_non_add {
            return Err(anyhow!(t!(
                "sdd.archive.new_spec_only_added",
                spec = update.capability
            )));
        }
    }

    let mut spec_doc = if update.target_exists {
        let target_content = fs::read_to_string(&update.target)?;
        let spec = backend.parse_main_spec(
            &target_content,
            &format!("spec `{}` during archive merge", update.capability),
        )?;
        spec.clone()
    } else {
        let purpose = if interactive {
            let prompt = format!("Purpose for new spec '{}':", update.capability);
            match Text::new(&prompt).prompt() {
                Ok(input) if !input.trim().is_empty() => input.trim().to_string(),
                _ => format!(
                    "TBD - created by archiving change {change}. Update purpose after archive.",
                    change = change_name
                ),
            }
        } else {
            format!(
                "TBD - created by archiving change {change}. Update purpose after archive.",
                change = change_name
            )
        };
        crate::sdd::spec::ir::MainSpecDoc {
            kind: "llman.sdd.spec".to_string(),
            name: update.capability.clone(),
            purpose,
            valid_scope: vec!["src/".to_string(), "tests/".to_string()],
            requirements: Vec::new(),
            scenarios: Vec::new(),
            feature_refs: None,
        }
    };

    // Ensure canonical metadata.
    spec_doc.kind = "llman.sdd.spec".to_string();
    spec_doc.name = update.capability.clone();

    let mut scenarios_by_req: HashMap<String, Vec<crate::sdd::spec::ir::ScenarioEntry>> =
        HashMap::new();
    for row in delta.op_scenarios {
        scenarios_by_req
            .entry(row.req_id.clone())
            .or_default()
            .push(row);
    }

    let mut add_or_modify_ids = std::collections::HashSet::new();
    for op in &delta.ops {
        let kind = op.op.trim().to_ascii_lowercase();
        if kind == "add_requirement" || kind == "modify_requirement" {
            add_or_modify_ids.insert(op.req_id.clone());
        }
    }
    for req_id in scenarios_by_req.keys() {
        if !add_or_modify_ids.contains(req_id) {
            return Err(anyhow!(
                "delta spec `{}`: op scenarios must reference add/modify ops only; found scenarios for `{}`",
                update.capability,
                req_id
            ));
        }
    }

    let mut counts = ApplyCounts::default();

    fn replace_scenarios(
        scenarios: &mut Vec<crate::sdd::spec::ir::ScenarioEntry>,
        req_id: &str,
        new_rows: Vec<crate::sdd::spec::ir::ScenarioEntry>,
    ) {
        let insert_pos = scenarios
            .iter()
            .position(|row| row.req_id == req_id)
            .unwrap_or(scenarios.len());
        scenarios.retain(|row| row.req_id != req_id);
        scenarios.splice(insert_pos..insert_pos, new_rows);
    }

    for op in delta.ops {
        let op_kind = op.op.trim().to_ascii_lowercase();
        match op_kind.as_str() {
            "add_requirement" => {
                counts.added += 1;
                let title = op
                    .title
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("add_requirement missing title"))?
                    .to_string();
                let statement = op
                    .statement
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("add_requirement missing statement"))?
                    .to_string();

                if spec_doc
                    .requirements
                    .iter()
                    .any(|req| req.req_id == op.req_id)
                {
                    return Err(anyhow!(t!(
                        "sdd.archive.add_exists",
                        spec = update.capability,
                        name = op.req_id
                    )));
                }

                spec_doc
                    .requirements
                    .push(crate::sdd::spec::ir::RequirementEntry {
                        req_id: op.req_id.clone(),
                        title,
                        statement,
                    });

                let scenarios = scenarios_by_req.remove(&op.req_id).unwrap_or_default();
                if scenarios.is_empty() {
                    return Err(anyhow!(
                        "delta spec `{}`: add_requirement `{}` must include at least one scenario row",
                        update.capability,
                        op.req_id
                    ));
                }
                replace_scenarios(&mut spec_doc.scenarios, &op.req_id, scenarios);
            }
            "modify_requirement" => {
                counts.modified += 1;
                let title = op
                    .title
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("modify_requirement missing title"))?
                    .to_string();
                let statement = op
                    .statement
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow!("modify_requirement missing statement"))?
                    .to_string();

                let Some(req) = spec_doc
                    .requirements
                    .iter_mut()
                    .find(|req| req.req_id == op.req_id)
                else {
                    return Err(anyhow!(t!(
                        "sdd.archive.modify_missing",
                        spec = update.capability,
                        name = op.req_id
                    )));
                };
                req.title = title;
                req.statement = statement;

                let scenarios = scenarios_by_req.remove(&op.req_id).unwrap_or_default();
                if scenarios.is_empty() {
                    return Err(anyhow!(
                        "delta spec `{}`: modify_requirement `{}` must include at least one scenario row",
                        update.capability,
                        op.req_id
                    ));
                }
                replace_scenarios(&mut spec_doc.scenarios, &op.req_id, scenarios);
            }
            "remove_requirement" => {
                counts.removed += 1;
                let key = op.req_id.clone();
                if !spec_doc.requirements.iter().any(|req| req.req_id == key) {
                    let missing_name = op
                        .name
                        .as_deref()
                        .map(|v| v.trim())
                        .filter(|v| !v.is_empty())
                        .unwrap_or_else(|| op.req_id.trim());
                    return Err(anyhow!(t!(
                        "sdd.archive.remove_missing",
                        spec = update.capability,
                        name = missing_name
                    )));
                }
                spec_doc.requirements.retain(|req| req.req_id != key);
                spec_doc.scenarios.retain(|row| row.req_id != key);
            }
            "rename_requirement" => {
                counts.renamed += 1;
                let from = op
                    .from
                    .ok_or_else(|| anyhow!("rename_requirement missing from"))?;
                let to = op
                    .to
                    .ok_or_else(|| anyhow!("rename_requirement missing to"))?;
                if to.trim().is_empty() {
                    return Err(anyhow!("rename_requirement missing to"));
                }

                if spec_doc
                    .requirements
                    .iter()
                    .any(|r| r.title.trim() == to.trim())
                {
                    return Err(anyhow!(t!(
                        "sdd.archive.rename_exists",
                        spec = update.capability,
                        name = to
                    )));
                }

                let current_title =
                    match spec_doc.requirements.iter().find(|r| r.req_id == op.req_id) {
                        Some(req) => req.title.clone(),
                        None => {
                            return Err(anyhow!(t!(
                                "sdd.archive.rename_missing",
                                spec = update.capability,
                                name = op.req_id
                            )));
                        }
                    };

                if !from.trim().is_empty() && current_title.trim() != from.trim() {
                    return Err(anyhow!(
                        "Rename source mismatch for spec `{}` requirement `{}`: expected `{}`, found `{}`",
                        update.capability,
                        op.req_id,
                        from,
                        current_title
                    ));
                }

                let Some(req) = spec_doc
                    .requirements
                    .iter_mut()
                    .find(|req| req.req_id == op.req_id)
                else {
                    return Err(anyhow!(t!(
                        "sdd.archive.rename_missing",
                        spec = update.capability,
                        name = op.req_id
                    )));
                };
                req.title = to.trim().to_string();
            }
            other => {
                return Err(anyhow!(
                    "delta spec `{}`: unsupported op `{}`",
                    update.capability,
                    other
                ));
            }
        }
    }

    if !scenarios_by_req.is_empty() {
        let keys = scenarios_by_req
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        return Err(anyhow!(
            "delta spec `{}`: found op scenarios without matching add/modify ops: {}",
            update.capability,
            keys
        ));
    }

    let rebuilt = backend.dump_main_spec(&spec_doc)?;

    Ok((rebuilt, counts))
}

fn write_updates(prepared: &[(SpecUpdate, String, ApplyCounts)]) -> Result<()> {
    let mut totals = ApplyCounts::default();
    for (update, rebuilt, counts) in prepared {
        let target_dir = update
            .target
            .parent()
            .ok_or_else(|| anyhow!(t!("sdd.archive.invalid_target")))?;
        fs::create_dir_all(target_dir)?;
        atomic_write_with_mode(&update.target, rebuilt.as_bytes(), None)?;
        print_apply_counts(update, counts, false);
        totals.added += counts.added;
        totals.modified += counts.modified;
        totals.removed += counts.removed;
        totals.renamed += counts.renamed;
    }
    println!(
        "{}",
        t!(
            "sdd.archive.totals",
            added = totals.added,
            modified = totals.modified,
            removed = totals.removed,
            renamed = totals.renamed
        )
    );
    Ok(())
}

fn print_dry_run_specs(prepared: &[(SpecUpdate, String, ApplyCounts)]) {
    for (update, _rebuilt, counts) in prepared {
        print_apply_counts(update, counts, true);
    }
}

fn print_apply_counts(update: &SpecUpdate, counts: &ApplyCounts, dry_run: bool) {
    let label = if dry_run {
        t!(
            "sdd.archive.dry_run_apply",
            path = display_llmanspec_path(&update.target)
        )
    } else {
        t!(
            "sdd.archive.apply",
            path = display_llmanspec_path(&update.target)
        )
    };
    println!("{label}");
    if counts.added > 0 {
        println!("{}", t!("sdd.archive.count_added", count = counts.added));
    }
    if counts.modified > 0 {
        println!(
            "{}",
            t!("sdd.archive.count_modified", count = counts.modified)
        );
    }
    if counts.removed > 0 {
        println!(
            "{}",
            t!("sdd.archive.count_removed", count = counts.removed)
        );
    }
    if counts.renamed > 0 {
        println!(
            "{}",
            t!("sdd.archive.count_renamed", count = counts.renamed)
        );
    }
}

fn print_archive_move(from: &Path, to: &Path) {
    println!(
        "{}",
        t!(
            "sdd.archive.dry_run_move",
            from = display_llmanspec_path(from),
            to = display_llmanspec_path(to)
        )
    );
}

fn display_llmanspec_path(path: &Path) -> String {
    let display = path.display().to_string();
    if let Some(idx) = display.find(LLMANSPEC_DIR_NAME) {
        return display[idx..].to_string();
    }
    display
}

fn archive_date() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn clone_update(update: &SpecUpdate) -> SpecUpdate {
    SpecUpdate {
        capability: update.capability.clone(),
        source: update.source.clone(),
        target: update.target.clone(),
        target_exists: update.target_exists,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create dir");
        }
        fs::write(path, content).expect("write file");
    }

    #[test]
    fn builds_new_spec_with_added_requirement() {
        let dir = tempdir().expect("tempdir");
        let change_spec = dir.path().join("changes/add-thing/specs/foo/spec.toon");
        let delta = "kind: llman.sdd.delta\nops[1]{op,req_id,title,statement,from,to,name}:\n  add_requirement,new-capability,\"New capability\",\"System MUST support the new capability.\",null,null,null\nop_scenarios[1]{req_id,id,given,when,then}:\n  new-capability,new-capability,\"\",a request arrives,it succeeds\n";
        write_file(&change_spec, delta);

        let update = SpecUpdate {
            capability: "foo".to_string(),
            source: change_spec,
            target: dir.path().join("llmanspec/specs/foo/spec.toon"),
            target_exists: false,
        };

        let result = build_updated_spec(&update, "add-thing", false).expect("build spec");
        assert!(result.0.contains("llman.sdd.spec"));
        assert!(result.0.contains("New capability"));
        assert!(result.0.contains("System MUST support the new capability."));
        assert!(result.0.contains("valid_scope"));
    }

    #[test]
    fn errors_on_removed_requirement_for_new_spec() {
        let dir = tempdir().expect("tempdir");
        let change_spec = dir.path().join("changes/remove-thing/specs/foo/spec.toon");
        let delta = "kind: llman.sdd.delta\nops[1]{op,req_id,title,statement,from,to,name}:\n  remove_requirement,old-capability,null,null,null,null,\"Old capability\"\nop_scenarios[0]{req_id,id,given,when,then}:\n";
        write_file(&change_spec, delta);

        let update = SpecUpdate {
            capability: "foo".to_string(),
            source: change_spec,
            target: dir.path().join("llmanspec/specs/foo/spec.toon"),
            target_exists: false,
        };

        let result = build_updated_spec(&update, "remove-thing", false);
        assert!(result.is_err());
    }

    #[test]
    fn errors_on_missing_modified_requirement() {
        let dir = tempdir().expect("tempdir");
        let change_spec = dir.path().join("changes/update-thing/specs/foo/spec.toon");
        let delta = "kind: llman.sdd.delta\nops[1]{op,req_id,title,statement,from,to,name}:\n  modify_requirement,beta,\"Beta\",\"System MUST update beta.\",null,null,null\nop_scenarios[1]{req_id,id,given,when,then}:\n  beta,beta,\"\",beta changes,it is updated\n";
        write_file(&change_spec, delta);

        let existing_spec = "kind: llman.sdd.spec\nname: foo\npurpose: \"Test spec.\"\nvalid_scope[1]: src\nrequirements[1]{req_id,title,statement}:\n  alpha,\"Alpha\",\"System MUST support alpha.\"\nscenarios[1]{req_id,id,given,when,then}:\n  alpha,alpha,\"\",alpha is used,it works\n";
        let target = dir.path().join("llmanspec/specs/foo/spec.toon");
        write_file(&target, existing_spec);

        let update = SpecUpdate {
            capability: "foo".to_string(),
            source: change_spec,
            target,
            target_exists: true,
        };

        let result = build_updated_spec(&update, "update-thing", false);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_path_traversal_change_id() {
        let dir = tempdir().expect("tempdir");
        let args = ArchiveArgs {
            change: Some("../oops".to_string()),
            skip_specs: true,
            dry_run: true,
            force: false,
            no_interactive: false,
        };
        let result = run_with_root(dir.path(), args);
        assert!(result.is_err());
    }

    #[test]
    fn archive_blocked_by_pending_tasks() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let config_path = root.join("llmanspec/config.yaml");
        write_file(&config_path, "schema: spec-driven\nlocale: en\n");
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(
            &change_dir.join("proposal.md"),
            "## Why\nTest change for archive gate",
        );
        write_file(
            &change_dir.join("tasks.md"),
            "- [x] Done task\n- [ ] Pending task\n",
        );
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unchecked tasks"));
    }

    #[test]
    fn archive_allowed_when_force_with_pending() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let config_path = root.join("llmanspec/config.yaml");
        write_file(&config_path, "schema: spec-driven\nlocale: en\n");
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(
            &change_dir.join("proposal.md"),
            "## Why\nTest change for archive gate",
        );
        write_file(&change_dir.join("tasks.md"), "- [x] Done\n- [ ] Pending\n");
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: true,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_ok());
    }

    #[test]
    fn archive_passes_with_all_completed() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let config_path = root.join("llmanspec/config.yaml");
        write_file(&config_path, "schema: spec-driven\nlocale: en\n");
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(&change_dir.join("proposal.md"), "## Why\nAll done");
        write_file(&change_dir.join("tasks.md"), "- [x] Done1\n- [x] Done2\n");
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_ok());
    }

    #[test]
    fn archive_blocked_by_cancelled_now_pending() {
        // Cancelled tasks are now Pending, so they block archive.
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let config_path = root.join("llmanspec/config.yaml");
        write_file(&config_path, "schema: spec-driven\nlocale: en\n");
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(&change_dir.join("proposal.md"), "## Why\nTest");
        write_file(
            &change_dir.join("tasks.md"),
            "- [x] Done\n- [ ] Not needed (cancelled — done)\n",
        );
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unchecked"));
    }

    #[test]
    fn archive_blocked_by_completion_ratio() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let config_path = root.join("llmanspec/config.yaml");
        write_file(
            &config_path,
            "schema: spec-driven\nlocale: en\narchive:\n  min_completion_ratio: 0.8\n",
        );
        let change_dir = root.join("llmanspec/changes/test-change");
        write_file(&change_dir.join("proposal.md"), "## Why\nTest");
        write_file(
            &change_dir.join("tasks.md"),
            // All cancelled are now pending, so pending > 0 blocks first
            "- [x] Done\n- [ ] Not needed (cancelled — x)\n- [ ] Also cancelled (cancelled — y)\n",
        );
        let args = ArchiveArgs {
            change: Some("test-change".to_string()),
            skip_specs: true,
            dry_run: false,
            force: false,
            no_interactive: true,
        };
        let result = run_with_root(root, args);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unchecked")
                || msg.contains("completion ratio")
                || msg.contains("below minimum"),
            "got: {msg}"
        );
    }
}
