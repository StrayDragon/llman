use crate::sdd::change::delta::{RequirementBlock, normalize_requirement_name, parse_delta_spec};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::ison::{
    compose_with_frontmatter, parse_ison_document, render_ison_code_block, split_frontmatter,
};
use crate::sdd::spec::staleness::evaluate_staleness_with_override;
use crate::sdd::spec::validation::{
    ValidationIssue, ValidationLevel, validate_spec_content_with_frontmatter,
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ArchiveArgs {
    pub change: Option<String>,
    pub skip_specs: bool,
    pub dry_run: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchiveSpecDocument {
    version: String,
    kind: String,
    name: String,
    purpose: String,
    requirements: Vec<ArchiveRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchiveRequirement {
    req_id: String,
    title: String,
    statement: String,
    scenarios: Vec<ArchiveScenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchiveScenario {
    id: String,
    text: String,
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
            let prepared = prepare_updates(&updates, change_name, root, validate_specs)?;
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
) -> Result<Vec<(SpecUpdate, String, ApplyCounts)>> {
    let mut prepared = Vec::new();
    for update in updates {
        let built = build_updated_spec(update, change_name)?;
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

fn build_updated_spec(update: &SpecUpdate, change_name: &str) -> Result<(String, ApplyCounts)> {
    let change_content = fs::read_to_string(&update.source)?;
    let plan = parse_delta_spec(&change_content)?;

    let delta_count =
        plan.added.len() + plan.modified.len() + plan.removed.len() + plan.renamed.len();
    if delta_count == 0 {
        return Err(anyhow!(t!(
            "sdd.archive.no_deltas",
            spec = update.capability
        )));
    }

    if !update.target_exists
        && (!plan.modified.is_empty() || !plan.removed.is_empty() || !plan.renamed.is_empty())
    {
        return Err(anyhow!(t!(
            "sdd.archive.new_spec_only_added",
            spec = update.capability
        )));
    }

    let (frontmatter_yaml, target_doc) = if update.target_exists {
        let target_content = fs::read_to_string(&update.target)?;
        let (frontmatter_yaml, body) = split_frontmatter(&target_content);
        let doc: ArchiveSpecDocument = parse_ison_document(
            &body,
            &format!("spec `{}` during archive merge", update.capability),
        )?;
        (frontmatter_yaml, doc)
    } else {
        (
            Some(default_frontmatter_yaml(change_name)),
            build_spec_skeleton(&update.capability, change_name),
        )
    };

    let mut order: Vec<String> = Vec::new();
    let mut map: HashMap<String, ArchiveRequirement> = HashMap::new();
    for requirement in &target_doc.requirements {
        let key = normalize_requirement_name(&requirement.req_id);
        order.push(key.clone());
        map.insert(key, requirement.clone());
    }

    let counts = ApplyCounts {
        added: plan.added.len(),
        modified: plan.modified.len(),
        removed: plan.removed.len(),
        renamed: plan.renamed.len(),
    };

    for rename in &plan.renamed {
        let req_id = normalize_requirement_name(&rename.req_id);
        if !map.contains_key(&req_id) {
            return Err(anyhow!(t!(
                "sdd.archive.rename_missing",
                spec = update.capability,
                name = rename.req_id
            )));
        }
        if map.values().any(|req| req.title.trim() == rename.to.trim()) {
            return Err(anyhow!(t!(
                "sdd.archive.rename_exists",
                spec = update.capability,
                name = rename.to
            )));
        }
        let requirement = map.get_mut(&req_id).expect("rename requirement");
        if !rename.from.trim().is_empty() && requirement.title.trim() != rename.from.trim() {
            return Err(anyhow!(
                "Rename source mismatch for spec `{}` requirement `{}`: expected `{}`, found `{}`",
                update.capability,
                rename.req_id,
                rename.from,
                requirement.title
            ));
        }
        requirement.title = rename.to.trim().to_string();
    }

    for removed in &plan.removed {
        let key = normalize_requirement_name(&removed.req_id);
        if !map.contains_key(&key) {
            let missing_name = removed.name.as_deref().unwrap_or(&removed.req_id);
            return Err(anyhow!(t!(
                "sdd.archive.remove_missing",
                spec = update.capability,
                name = missing_name
            )));
        }
        map.remove(&key);
        order.retain(|k| k != &key);
    }

    for block in &plan.modified {
        let key = normalize_requirement_name(&block.req_id);
        if !map.contains_key(&key) {
            return Err(anyhow!(t!(
                "sdd.archive.modify_missing",
                spec = update.capability,
                name = block.req_id
            )));
        }
        map.insert(key, archive_requirement_from_block(block));
    }

    for block in &plan.added {
        let key = normalize_requirement_name(&block.req_id);
        if map.contains_key(&key) {
            return Err(anyhow!(t!(
                "sdd.archive.add_exists",
                spec = update.capability,
                name = block.req_id
            )));
        }
        order.push(key.clone());
        map.insert(key, archive_requirement_from_block(block));
    }

    let mut requirements = Vec::new();
    for key in &order {
        if let Some(requirement) = map.get(key) {
            requirements.push(requirement.clone());
        }
    }

    let mut rebuilt_doc = target_doc;
    rebuilt_doc.requirements = requirements;
    let body = render_ison_code_block(&rebuilt_doc)?;
    let rebuilt = compose_with_frontmatter(frontmatter_yaml.as_deref(), &body);
    Ok((rebuilt, counts))
}

fn archive_requirement_from_block(block: &RequirementBlock) -> ArchiveRequirement {
    ArchiveRequirement {
        req_id: block.req_id.clone(),
        title: block.name.clone(),
        statement: block.statement.clone(),
        scenarios: block
            .scenarios
            .iter()
            .map(|scenario| ArchiveScenario {
                id: scenario.scenario_id.clone(),
                text: scenario.text.clone(),
            })
            .collect(),
    }
}

fn build_spec_skeleton(spec_name: &str, change_name: &str) -> ArchiveSpecDocument {
    ArchiveSpecDocument {
        version: "1.0.0".to_string(),
        kind: "llman.sdd.spec".to_string(),
        name: spec_name.to_string(),
        purpose: format!(
            "TBD - created by archiving change {change}. Update purpose after archive.",
            change = change_name
        ),
        requirements: Vec::new(),
    }
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
        fs::write(&update.target, rebuilt)?;
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
{
  "version": "1.0.0",
  "kind": "llman.sdd.delta",
  "ops": [
    {
      "op": "add_requirement",
      "req_id": "new-capability",
      "title": "New capability",
      "statement": "System MUST support the new capability.",
      "scenarios": [
        {
          "id": "new-capability",
          "text": "- **WHEN** a request arrives\n- **THEN** it succeeds"
        }
      ]
    }
  ]
}
```
"#;
        write_file(&change_spec, delta);

        let update = SpecUpdate {
            capability: "foo".to_string(),
            source: change_spec,
            target: dir.path().join("llmanspec/specs/foo/spec.md"),
            target_exists: false,
        };

        let result = build_updated_spec(&update, "add-thing").expect("build spec");
        assert!(result.0.contains("\"kind\": \"llman.sdd.spec\""));
        assert!(result.0.contains("\"title\": \"New capability\""));
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
{
  "version": "1.0.0",
  "kind": "llman.sdd.delta",
  "ops": [
    {
      "op": "remove_requirement",
      "req_id": "old-capability",
      "name": "Old capability"
    }
  ]
}
```
"#;
        write_file(&change_spec, delta);

        let update = SpecUpdate {
            capability: "foo".to_string(),
            source: change_spec,
            target: dir.path().join("llmanspec/specs/foo/spec.md"),
            target_exists: false,
        };

        let result = build_updated_spec(&update, "remove-thing");
        assert!(result.is_err());
    }

    #[test]
    fn errors_on_missing_modified_requirement() {
        let dir = tempdir().expect("tempdir");
        let change_spec = dir.path().join("changes/update-thing/specs/foo/spec.md");
        let delta = r#"```ison
{
  "version": "1.0.0",
  "kind": "llman.sdd.delta",
  "ops": [
    {
      "op": "modify_requirement",
      "req_id": "beta",
      "title": "Beta",
      "statement": "System MUST update beta.",
      "scenarios": [
        {
          "id": "beta",
          "text": "- **WHEN** beta changes\n- **THEN** it is updated"
        }
      ]
    }
  ]
}
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
{
  "version": "1.0.0",
  "kind": "llman.sdd.spec",
  "name": "foo",
  "purpose": "Test spec.",
  "requirements": [
    {
      "req_id": "alpha",
      "title": "Alpha",
      "statement": "System MUST support alpha.",
      "scenarios": [
        {
          "id": "alpha",
          "text": "- **WHEN** alpha is used\n- **THEN** it works"
        }
      ]
    }
  ]
}
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

        let result = build_updated_spec(&update, "update-thing");
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
        };
        let result = run_with_root(dir.path(), args);
        assert!(result.is_err());
    }
}
