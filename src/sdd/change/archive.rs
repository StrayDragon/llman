use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::ison::{compose_with_frontmatter, split_frontmatter};
use crate::sdd::spec::ison_table::render_ison_fence;
use crate::sdd::spec::ison_v1;
use crate::sdd::spec::staleness::evaluate_staleness_with_override;
use crate::sdd::spec::validation::{
    ValidationIssue, ValidationLevel, validate_spec_content_with_frontmatter,
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ArchiveArgs {
    pub change: Option<String>,
    pub skip_specs: bool,
    pub dry_run: bool,
    pub pretty_ison: bool,
    pub force: bool,
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

    if !args.skip_specs {
        let validate_specs = !args.force;
        let updates = find_spec_updates(&change_dir, root)?;
        if !updates.is_empty() {
            let prepared = prepare_updates(
                &updates,
                change_name,
                root,
                validate_specs,
                args.pretty_ison,
            )?;
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

    if archive_path.exists() {
        return Err(anyhow!(t!(
            "sdd.archive.archive_exists",
            name = archive_name
        )));
    }

    fs::create_dir_all(&archive_dir)?;
    fs::rename(&change_dir, &archive_path)?;
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
        let source = entry.path().join("spec.md");
        if !source.exists() {
            continue;
        }
        let target = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join(&capability)
            .join("spec.md");
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
    pretty_ison: bool,
) -> Result<Vec<(SpecUpdate, String, ApplyCounts)>> {
    let mut prepared = Vec::new();
    for update in updates {
        let built = build_updated_spec(update, change_name, pretty_ison)?;
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
    pretty_ison: bool,
) -> Result<(String, ApplyCounts)> {
    build_updated_spec_table_object(update, change_name, pretty_ison)
}

fn build_updated_spec_table_object(
    update: &SpecUpdate,
    change_name: &str,
    pretty_ison: bool,
) -> Result<(String, ApplyCounts)> {
    let delta_content = fs::read_to_string(&update.source)?;
    let delta = ison_v1::parse_delta_body(
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
        let has_non_add = delta.ops.iter().any(|op| op.op != "add_requirement");
        if has_non_add {
            return Err(anyhow!(t!(
                "sdd.archive.new_spec_only_added",
                spec = update.capability
            )));
        }
    }

    let (frontmatter_yaml, mut spec_doc) = if update.target_exists {
        let target_content = fs::read_to_string(&update.target)?;
        let (frontmatter_yaml, body) = split_frontmatter(&target_content);
        let spec = ison_v1::parse_spec_body(
            &body,
            &format!("spec `{}` during archive merge", update.capability),
        )?;
        (frontmatter_yaml, spec)
    } else {
        let spec = ison_v1::CanonicalSpec {
            meta: ison_v1::SpecMeta {
                kind: ison_v1::SPEC_KIND.to_string(),
                name: update.capability.clone(),
                purpose: format!(
                    "TBD - created by archiving change {change}. Update purpose after archive.",
                    change = change_name
                ),
            },
            requirements: Vec::new(),
            scenarios: Vec::new(),
        };
        (Some(default_frontmatter_yaml(change_name)), spec)
    };

    let mut scenarios_by_req: HashMap<String, Vec<ison_v1::ScenarioRow>> = HashMap::new();
    for row in delta.scenarios {
        scenarios_by_req
            .entry(row.req_id.clone())
            .or_default()
            .push(row);
    }

    let mut add_or_modify_ids = std::collections::HashSet::new();
    for op in &delta.ops {
        if op.op == "add_requirement" || op.op == "modify_requirement" {
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
        scenarios: &mut Vec<ison_v1::ScenarioRow>,
        req_id: &str,
        new_rows: Vec<ison_v1::ScenarioRow>,
    ) {
        let insert_pos = scenarios
            .iter()
            .position(|row| row.req_id == req_id)
            .unwrap_or(scenarios.len());
        scenarios.retain(|row| row.req_id != req_id);
        scenarios.splice(insert_pos..insert_pos, new_rows);
    }

    for op in delta.ops {
        match op.op.as_str() {
            "add_requirement" => {
                counts.added += 1;
                let title = op
                    .title
                    .ok_or_else(|| anyhow!("add_requirement missing title"))?;
                let statement = op
                    .statement
                    .ok_or_else(|| anyhow!("add_requirement missing statement"))?;

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

                spec_doc.requirements.push(ison_v1::RequirementRow {
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
                    .ok_or_else(|| anyhow!("modify_requirement missing title"))?;
                let statement = op
                    .statement
                    .ok_or_else(|| anyhow!("modify_requirement missing statement"))?;

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
                    let missing_name = op.name.as_deref().unwrap_or(&op.req_id);
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

    let payload = ison_v1::dump_spec_payload(&spec_doc, pretty_ison);
    let body = render_ison_fence(&payload);
    let rebuilt = compose_with_frontmatter(frontmatter_yaml.as_deref(), &body);
    Ok((rebuilt, counts))
}

fn default_frontmatter_yaml(change_name: &str) -> String {
    format!(
        "llman_spec_valid_scope:\n  - src/\n  - tests/\nllman_spec_valid_commands:\n  - cargo test\nllman_spec_evidence:\n  - \"Archived from change {change}\"",
        change = change_name
    )
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
        let change_spec = dir.path().join("changes/add-thing/specs/foo/spec.md");
        let delta = r#"```ison
object.delta
kind
llman.sdd.delta

table.ops
op req_id title statement from to name
add_requirement new-capability "New capability" "System MUST support the new capability." ~ ~ ~

table.op_scenarios
req_id id given when then
new-capability new-capability "" "a request arrives" "it succeeds"
```
"#;
        write_file(&change_spec, delta);

        let update = SpecUpdate {
            capability: "foo".to_string(),
            source: change_spec,
            target: dir.path().join("llmanspec/specs/foo/spec.md"),
            target_exists: false,
        };

        let result = build_updated_spec(&update, "add-thing", false).expect("build spec");
        assert!(result.0.contains("object.spec"));
        assert!(result.0.contains("llman.sdd.spec"));
        assert!(result.0.contains("\"New capability\""));
        assert!(result.0.contains("System MUST support the new capability."));
        assert!(result.0.contains("llman_spec_valid_scope"));
        assert!(result.0.contains("llman_spec_valid_commands"));
        assert!(result.0.contains("llman_spec_evidence"));
    }

    #[test]
    fn errors_on_removed_requirement_for_new_spec() {
        let dir = tempdir().expect("tempdir");
        let change_spec = dir.path().join("changes/remove-thing/specs/foo/spec.md");
        let delta = r#"```ison
object.delta
kind
llman.sdd.delta

table.ops
op req_id title statement from to name
remove_requirement old-capability ~ ~ ~ ~ "Old capability"

table.op_scenarios
req_id id given when then
```
"#;
        write_file(&change_spec, delta);

        let update = SpecUpdate {
            capability: "foo".to_string(),
            source: change_spec,
            target: dir.path().join("llmanspec/specs/foo/spec.md"),
            target_exists: false,
        };

        let result = build_updated_spec(&update, "remove-thing", false);
        assert!(result.is_err());
    }

    #[test]
    fn errors_on_missing_modified_requirement() {
        let dir = tempdir().expect("tempdir");
        let change_spec = dir.path().join("changes/update-thing/specs/foo/spec.md");
        let delta = r#"```ison
object.delta
kind
llman.sdd.delta

table.ops
op req_id title statement from to name
modify_requirement beta "Beta" "System MUST update beta." ~ ~ ~

table.op_scenarios
req_id id given when then
beta beta "" "beta changes" "it is updated"
```
"#;
        write_file(&change_spec, delta);

        let existing_spec = r#"---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - tests
---

```ison
object.spec
kind name purpose
llman.sdd.spec foo "Test spec."

table.requirements
req_id title statement
alpha Alpha "System MUST support alpha."

table.scenarios
req_id id given when then
alpha alpha "" "alpha is used" "it works"
```
"#;
        let target = dir.path().join("llmanspec/specs/foo/spec.md");
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
            pretty_ison: false,
            force: false,
        };
        let result = run_with_root(dir.path(), args);
        assert!(result.is_err());
    }
}
