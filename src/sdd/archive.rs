use crate::sdd::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::delta::{RequirementBlock, normalize_requirement_name, parse_delta_spec};
use crate::sdd::staleness::evaluate_staleness_with_override;
use crate::sdd::validation::{
    ValidationIssue, ValidationLevel, validate_spec_content_with_frontmatter,
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use regex::Regex;
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

struct RequirementsSection {
    before: String,
    header_line: String,
    preamble: String,
    body_blocks: Vec<RequirementBlock>,
    after: String,
}

pub fn run(args: ArchiveArgs) -> Result<()> {
    let change_name = args
        .change
        .as_ref()
        .ok_or_else(|| anyhow!(t!("sdd.archive.change_required")))?;
    let root = Path::new(".");
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

    let target_content = if update.target_exists {
        fs::read_to_string(&update.target)?
    } else {
        build_spec_skeleton(&update.capability, change_name)
    };

    let section = extract_requirements_section(&target_content)?;
    let mut order: Vec<String> = Vec::new();
    let mut map: HashMap<String, RequirementBlock> = HashMap::new();
    for block in &section.body_blocks {
        let key = normalize_requirement_name(&block.name);
        order.push(key.clone());
        map.insert(key, block.clone());
    }

    let counts = ApplyCounts {
        added: plan.added.len(),
        modified: plan.modified.len(),
        removed: plan.removed.len(),
        renamed: plan.renamed.len(),
    };

    for rename in &plan.renamed {
        let from = normalize_requirement_name(&rename.from);
        let to = normalize_requirement_name(&rename.to);
        if !map.contains_key(&from) {
            return Err(anyhow!(t!(
                "sdd.archive.rename_missing",
                spec = update.capability,
                name = rename.from
            )));
        }
        if map.contains_key(&to) {
            return Err(anyhow!(t!(
                "sdd.archive.rename_exists",
                spec = update.capability,
                name = rename.to
            )));
        }
        let mut block = map.remove(&from).expect("rename block");
        let new_header = format!("### Requirement: {}", rename.to);
        let mut lines: Vec<&str> = block.raw.lines().collect();
        if !lines.is_empty() {
            lines[0] = &new_header;
        }
        block.raw = lines.join("\n");
        block.name = rename.to.clone();
        block.header_line = new_header;
        map.insert(to.clone(), block);
        order.retain(|key| key != &from);
        order.push(to);
    }

    for name in &plan.removed {
        let key = normalize_requirement_name(name);
        if !map.contains_key(&key) {
            return Err(anyhow!(t!(
                "sdd.archive.remove_missing",
                spec = update.capability,
                name = name
            )));
        }
        map.remove(&key);
        order.retain(|k| k != &key);
    }

    for block in &plan.modified {
        let key = normalize_requirement_name(&block.name);
        if !map.contains_key(&key) {
            return Err(anyhow!(t!(
                "sdd.archive.modify_missing",
                spec = update.capability,
                name = block.name
            )));
        }
        if !header_matches(block, &key) {
            return Err(anyhow!(t!(
                "sdd.archive.modify_header_mismatch",
                spec = update.capability,
                name = block.name
            )));
        }
        map.insert(key, block.clone());
    }

    for block in &plan.added {
        let key = normalize_requirement_name(&block.name);
        if map.contains_key(&key) {
            return Err(anyhow!(t!(
                "sdd.archive.add_exists",
                spec = update.capability,
                name = block.name
            )));
        }
        order.push(key.clone());
        map.insert(key, block.clone());
    }

    let mut ordered_blocks = Vec::new();
    for key in &order {
        if let Some(block) = map.get(key) {
            ordered_blocks.push(block.clone());
        }
    }

    let rebuilt = rebuild_spec(&section, &ordered_blocks);
    Ok((rebuilt, counts))
}

fn extract_requirements_section(content: &str) -> Result<RequirementsSection> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let header_re = Regex::new(r"^##\s+Requirements\s*$").expect("regex");
    let mut header_index = None;

    for (idx, line) in lines.iter().enumerate() {
        if header_re.is_match(line.trim()) {
            header_index = Some(idx);
            break;
        }
    }

    let header_index =
        header_index.ok_or_else(|| anyhow!(t!("sdd.archive.requirements_missing")))?;
    let header_line = lines[header_index].to_string();
    let before = lines[..header_index].join("\n");

    let mut end_index = lines.len();
    for (idx, line) in lines.iter().enumerate().skip(header_index + 1) {
        if line.trim_start().starts_with("## ") {
            end_index = idx;
            break;
        }
    }

    let section_lines = &lines[header_index + 1..end_index];
    let (preamble, body_blocks) = parse_requirement_blocks(section_lines);
    let after = lines[end_index..].join("\n");

    Ok(RequirementsSection {
        before,
        header_line,
        preamble,
        body_blocks,
        after,
    })
}

fn parse_requirement_blocks(lines: &[&str]) -> (String, Vec<RequirementBlock>) {
    let header_re = Regex::new(r"^###\s*Requirement:\s*(.+)\s*$").expect("regex");
    let mut i = 0;
    let mut preamble_lines = Vec::new();

    while i < lines.len() && !header_re.is_match(lines[i]) {
        preamble_lines.push(lines[i]);
        i += 1;
    }

    let mut blocks = Vec::new();
    while i < lines.len() {
        if !header_re.is_match(lines[i]) {
            i += 1;
            continue;
        }
        let header_line = lines[i].to_string();
        let name = header_re
            .captures(lines[i])
            .and_then(|caps| caps.get(1))
            .map(|m| normalize_requirement_name(m.as_str()))
            .unwrap_or_default();
        i += 1;
        let mut buffer = vec![header_line.clone()];
        while i < lines.len() && !header_re.is_match(lines[i]) {
            buffer.push(lines[i].to_string());
            i += 1;
        }
        blocks.push(RequirementBlock {
            header_line,
            name,
            raw: buffer.join("\n").trim_end().to_string(),
        });
    }

    (preamble_lines.join("\n").trim_end().to_string(), blocks)
}

fn rebuild_spec(section: &RequirementsSection, blocks: &[RequirementBlock]) -> String {
    let mut body_parts = Vec::new();
    if !section.preamble.trim().is_empty() {
        body_parts.push(section.preamble.trim_end().to_string());
    }
    for block in blocks {
        body_parts.push(block.raw.trim_end().to_string());
    }
    let body = body_parts.join("\n\n").trim_end().to_string();

    let mut parts = Vec::new();
    if !section.before.trim().is_empty() {
        parts.push(section.before.trim_end().to_string());
    }
    parts.push(section.header_line.clone());
    if !body.is_empty() {
        parts.push(body);
    }
    if !section.after.trim().is_empty() {
        parts.push(section.after.trim_start().to_string());
    }
    parts.join("\n")
}

fn header_matches(block: &RequirementBlock, key: &str) -> bool {
    let header_re = Regex::new(r"^###\s*Requirement:\s*(.+)\s*$").expect("regex");
    let header = block.raw.lines().next().unwrap_or("").trim();
    let name = header_re
        .captures(header)
        .and_then(|caps| caps.get(1))
        .map(|m| normalize_requirement_name(m.as_str()))
        .unwrap_or_default();
    name == key
}

fn build_spec_skeleton(spec_name: &str, change_name: &str) -> String {
    format!(
        "---\nllman_spec_valid_scope:\n  - src/\n  - tests/\nllman_spec_valid_commands:\n  - cargo test\nllman_spec_evidence:\n  - \"Archived from change {change}\"\n---\n\n# {spec} Specification\n\n## Purpose\nTBD - created by archiving change {change}. Update Purpose after archive.\n\n## Requirements\n",
        spec = spec_name,
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
        let delta = r#"## ADDED Requirements
### Requirement: New capability
System MUST support the new capability.

#### Scenario: New capability
- **WHEN** a request arrives
- **THEN** it succeeds
"#;
        write_file(&change_spec, delta);

        let update = SpecUpdate {
            capability: "foo".to_string(),
            source: change_spec,
            target: dir.path().join("llmanspec/specs/foo/spec.md"),
            target_exists: false,
        };

        let result = build_updated_spec(&update, "add-thing").expect("build spec");
        assert!(result.0.contains("### Requirement: New capability"));
        assert!(result.0.contains("System MUST support the new capability."));
        assert!(result.0.contains("llman_spec_valid_scope"));
        assert!(result.0.contains("llman_spec_valid_commands"));
        assert!(result.0.contains("llman_spec_evidence"));
    }

    #[test]
    fn errors_on_removed_requirement_for_new_spec() {
        let dir = tempdir().expect("tempdir");
        let change_spec = dir.path().join("changes/remove-thing/specs/foo/spec.md");
        let delta = r#"## REMOVED Requirements
### Requirement: Old capability
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
        let delta = r#"## MODIFIED Requirements
### Requirement: Beta
System MUST update beta.

#### Scenario: Beta
- **WHEN** beta changes
- **THEN** it is updated
"#;
        write_file(&change_spec, delta);

        let existing_spec = r#"# Foo Specification

## Purpose
Test spec.

## Requirements
### Requirement: Alpha
System MUST support alpha.

#### Scenario: Alpha
- **WHEN** alpha is used
- **THEN** it works
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
}
