use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::spec::ison::{compose_with_frontmatter, render_ison_code_block, split_frontmatter};
use anyhow::{Result, anyhow};
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MigrateArgs {
    pub to_ison: bool,
    pub dry_run: bool,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct ScenarioBlock {
    name: String,
    text: String,
}

#[derive(Debug, Clone)]
struct RequirementBlock {
    title: String,
    statement: String,
    scenarios: Vec<ScenarioBlock>,
}

#[derive(Debug, Clone)]
struct RenamePair {
    from: String,
    to: String,
}

pub fn run(args: MigrateArgs) -> Result<()> {
    if !args.to_ison {
        return Err(anyhow!("`--to-ison` is required"));
    }
    let root = args.path.unwrap_or_else(|| PathBuf::from("."));
    run_with_root(&root, args.dry_run)
}

fn run_with_root(root: &Path, dry_run: bool) -> Result<()> {
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec.exists() {
        return Err(anyhow!(
            "llmanspec directory not found at {}",
            llmanspec.display()
        ));
    }

    let main_specs = collect_main_specs(&llmanspec)?;
    let delta_specs = collect_delta_specs(&llmanspec)?;

    let mut migrated = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for path in main_specs {
        let content = fs::read_to_string(&path)?;
        match migrate_main_spec(&path, &content) {
            Ok(Some(updated)) => {
                migrated.push(path.clone());
                if !dry_run {
                    fs::write(&path, updated)?;
                }
            }
            Ok(None) => skipped.push(path),
            Err(err) => failed.push((path, err.to_string())),
        }
    }

    for path in delta_specs {
        let content = fs::read_to_string(&path)?;
        match migrate_delta_spec(&path, &content) {
            Ok(Some(updated)) => {
                migrated.push(path.clone());
                if !dry_run {
                    fs::write(&path, updated)?;
                }
            }
            Ok(None) => skipped.push(path),
            Err(err) => failed.push((path, err.to_string())),
        }
    }

    let mode = if dry_run { "dry-run" } else { "applied" };
    println!(
        "ISON migration ({mode}): migrated={}, skipped={}, failed={}",
        migrated.len(),
        skipped.len(),
        failed.len()
    );
    for path in &migrated {
        println!("  + {}", display_rel(path));
    }
    for path in &skipped {
        println!("  = {}", display_rel(path));
    }

    if !failed.is_empty() {
        let mut details = String::new();
        for (path, err) in failed {
            details.push_str(&format!("\n- {}: {}", display_rel(&path), err));
        }
        return Err(anyhow!("Migration failed for some files:{details}"));
    }

    Ok(())
}

fn display_rel(path: &Path) -> String {
    let display = path.display().to_string();
    if let Some(idx) = display.find(LLMANSPEC_DIR_NAME) {
        return display[idx..].to_string();
    }
    display
}

fn collect_main_specs(llmanspec: &Path) -> Result<Vec<PathBuf>> {
    let specs_dir = llmanspec.join("specs");
    if !specs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(specs_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let spec = entry.path().join("spec.md");
        if spec.exists() {
            files.push(spec);
        }
    }
    files.sort();
    Ok(files)
}

fn collect_delta_specs(llmanspec: &Path) -> Result<Vec<PathBuf>> {
    let changes_dir = llmanspec.join("changes");
    if !changes_dir.exists() {
        return Ok(Vec::new());
    }

    let mut stack = vec![changes_dir];
    let mut files = Vec::new();

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = entry.file_type()?;
            if meta.is_dir() {
                stack.push(path);
                continue;
            }
            if !meta.is_file() || entry.file_name() != "spec.md" {
                continue;
            }
            if is_change_delta_path(&path) {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn is_change_delta_path(path: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    let Some(specs_dir) = parent.parent() else {
        return false;
    };
    specs_dir.file_name().is_some_and(|name| name == "specs")
}

fn migrate_main_spec(path: &Path, content: &str) -> Result<Option<String>> {
    let (frontmatter, body) = split_frontmatter(content);
    if body.contains("```ison") {
        return Ok(None);
    }

    let purpose = extract_section(&body, "Purpose").ok_or_else(|| {
        anyhow!(
            "main spec `{}` missing `## Purpose` section; cannot migrate",
            display_rel(path)
        )
    })?;
    let requirements_body = extract_section(&body, "Requirements").ok_or_else(|| {
        anyhow!(
            "main spec `{}` missing `## Requirements` section; cannot migrate",
            display_rel(path)
        )
    })?;

    let requirements = parse_requirement_blocks(&requirements_body);
    let mut req_slug_counter = HashMap::new();

    let requirement_values = requirements
        .into_iter()
        .map(|requirement| {
            let req_id = unique_slug(&requirement.title, &mut req_slug_counter, "requirement");
            let mut scenario_slug_counter = HashMap::new();
            let scenarios = requirement
                .scenarios
                .into_iter()
                .map(|scenario| {
                    let id = unique_slug(&scenario.name, &mut scenario_slug_counter, "scenario");
                    json!({
                        "id": id,
                        "text": scenario.text,
                    })
                })
                .collect::<Vec<_>>();

            json!({
                "req_id": req_id,
                "title": requirement.title,
                "statement": requirement.statement,
                "scenarios": scenarios,
            })
        })
        .collect::<Vec<_>>();

    let name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("spec")
        .to_string();

    let payload = json!({
        "version": "1.0.0",
        "kind": "llman.sdd.spec",
        "name": name,
        "purpose": purpose,
        "requirements": requirement_values,
    });

    let rendered = render_ison_code_block(&payload)?;
    Ok(Some(compose_with_frontmatter(
        frontmatter.as_deref(),
        &rendered,
    )))
}

fn migrate_delta_spec(path: &Path, content: &str) -> Result<Option<String>> {
    let (frontmatter, body) = split_frontmatter(content);
    if body.contains("```ison") {
        return Ok(None);
    }

    let added = extract_section(&body, "ADDED Requirements").unwrap_or_default();
    let modified = extract_section(&body, "MODIFIED Requirements").unwrap_or_default();
    let removed = extract_section(&body, "REMOVED Requirements").unwrap_or_default();
    let renamed = extract_section(&body, "RENAMED Requirements").unwrap_or_default();

    if added.trim().is_empty()
        && modified.trim().is_empty()
        && removed.trim().is_empty()
        && renamed.trim().is_empty()
    {
        return Err(anyhow!(
            "delta spec `{}` has no ADDED/MODIFIED/REMOVED/RENAMED sections",
            display_rel(path)
        ));
    }

    let mut ops = Vec::new();
    let mut req_slug_counter = HashMap::new();

    for requirement in parse_requirement_blocks(&added) {
        let req_id = unique_slug(&requirement.title, &mut req_slug_counter, "requirement");
        ops.push(requirement_to_op("add_requirement", req_id, requirement));
    }

    for requirement in parse_requirement_blocks(&modified) {
        let req_id = unique_slug(&requirement.title, &mut req_slug_counter, "requirement");
        ops.push(requirement_to_op("modify_requirement", req_id, requirement));
    }

    for title in parse_removed_titles(&removed) {
        let req_id = unique_slug(&title, &mut req_slug_counter, "requirement");
        ops.push(json!({
            "op": "remove_requirement",
            "req_id": req_id,
            "name": title,
        }));
    }

    for pair in parse_renamed_pairs(&renamed) {
        let req_id = unique_slug(&pair.from, &mut req_slug_counter, "requirement");
        ops.push(json!({
            "op": "rename_requirement",
            "req_id": req_id,
            "from": pair.from,
            "to": pair.to,
        }));
    }

    let payload = json!({
        "version": "1.0.0",
        "kind": "llman.sdd.delta",
        "ops": ops,
    });

    let rendered = render_ison_code_block(&payload)?;
    Ok(Some(compose_with_frontmatter(
        frontmatter.as_deref(),
        &rendered,
    )))
}

fn requirement_to_op(op: &str, req_id: String, requirement: RequirementBlock) -> serde_json::Value {
    let mut scenario_slug_counter = HashMap::new();
    let scenarios = requirement
        .scenarios
        .into_iter()
        .map(|scenario| {
            let id = unique_slug(&scenario.name, &mut scenario_slug_counter, "scenario");
            json!({
                "id": id,
                "text": scenario.text,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "op": op,
        "req_id": req_id,
        "title": requirement.title,
        "statement": requirement.statement,
        "scenarios": scenarios,
    })
}

fn parse_requirement_blocks(section_body: &str) -> Vec<RequirementBlock> {
    if section_body.trim().is_empty() {
        return Vec::new();
    }

    let req_re = Regex::new(r"^###\s*Requirement:\s*(.+)\s*$").expect("regex");
    let scenario_re = Regex::new(r"^####\s*Scenario:\s*(.+)\s*$").expect("regex");
    let lines: Vec<&str> = section_body.lines().collect();
    let mut i = 0;
    let mut blocks = Vec::new();

    while i < lines.len() {
        let req_caps = match req_re.captures(lines[i].trim()) {
            Some(caps) => caps,
            None => {
                i += 1;
                continue;
            }
        };
        let title = req_caps
            .get(1)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "Requirement".to_string());
        i += 1;

        let mut statement_lines = Vec::new();
        let mut scenarios = Vec::new();
        let mut current_scenario_name: Option<String> = None;
        let mut current_scenario_lines = Vec::new();

        while i < lines.len() && !req_re.is_match(lines[i].trim()) {
            if let Some(caps) = scenario_re.captures(lines[i].trim()) {
                if let Some(name) = current_scenario_name.take() {
                    let text = current_scenario_lines.join("\n").trim().to_string();
                    if !text.is_empty() {
                        scenarios.push(ScenarioBlock { name, text });
                    }
                    current_scenario_lines.clear();
                }
                current_scenario_name = Some(
                    caps.get(1)
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_else(|| "scenario".to_string()),
                );
                i += 1;
                continue;
            }

            if current_scenario_name.is_some() {
                current_scenario_lines.push(lines[i].to_string());
            } else {
                statement_lines.push(lines[i].to_string());
            }
            i += 1;
        }

        if let Some(name) = current_scenario_name.take() {
            let text = current_scenario_lines.join("\n").trim().to_string();
            if !text.is_empty() {
                scenarios.push(ScenarioBlock { name, text });
            }
        }

        let statement = statement_lines.join("\n").trim().to_string();
        blocks.push(RequirementBlock {
            title,
            statement,
            scenarios,
        });
    }

    blocks
}

fn parse_removed_titles(section_body: &str) -> Vec<String> {
    let header_re = Regex::new(r"^###\s*Requirement:\s*(.+?)\s*$").expect("regex");
    let bullet_re = Regex::new(r"^\s*-\s*`?###\s*Requirement:\s*(.+?)`?\s*$").expect("regex");
    let mut out = Vec::new();

    for line in section_body.lines() {
        let trimmed = line.trim();
        if let Some(caps) = header_re.captures(trimmed) {
            if let Some(m) = caps.get(1) {
                let title = m.as_str().trim().to_string();
                if !title.is_empty() {
                    out.push(title);
                }
            }
            continue;
        }
        if let Some(caps) = bullet_re.captures(trimmed)
            && let Some(m) = caps.get(1)
        {
            let title = m.as_str().trim().to_string();
            if !title.is_empty() {
                out.push(title);
            }
        }
    }

    out
}

fn parse_renamed_pairs(section_body: &str) -> Vec<RenamePair> {
    let from_re =
        Regex::new(r"^\s*-?\s*FROM:\s*`?###\s*Requirement:\s*(.+?)`?\s*$").expect("regex");
    let to_re = Regex::new(r"^\s*-?\s*TO:\s*`?###\s*Requirement:\s*(.+?)`?\s*$").expect("regex");

    let mut out = Vec::new();
    let mut current_from: Option<String> = None;
    for line in section_body.lines() {
        let trimmed = line.trim();
        if let Some(caps) = from_re.captures(trimmed) {
            current_from = caps.get(1).map(|m| m.as_str().trim().to_string());
            continue;
        }
        if let Some(caps) = to_re.captures(trimmed)
            && let Some(from) = current_from.take()
            && let Some(to) = caps.get(1).map(|m| m.as_str().trim().to_string())
            && !from.is_empty()
            && !to.is_empty()
        {
            out.push(RenamePair { from, to });
        }
    }

    out
}

fn unique_slug(raw: &str, seen: &mut HashMap<String, usize>, fallback: &str) -> String {
    let base = slugify(raw).unwrap_or_else(|| fallback.to_string());
    let counter = seen.entry(base.clone()).or_insert(0);
    *counter += 1;
    if *counter == 1 {
        return base;
    }
    format!("{}-{}", base, counter)
}

fn slugify(raw: &str) -> Option<String> {
    let mut out = String::new();
    let mut prev_dash = false;

    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() { None } else { Some(out) }
}

fn extract_section(content: &str, title: &str) -> Option<String> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let header_re = Regex::new(r"^##\s+(.+)$").expect("regex");

    let mut start = None;
    for (idx, line) in lines.iter().enumerate() {
        if let Some(caps) = header_re.captures(line.trim()) {
            let name = caps.get(1)?.as_str().trim();
            if name.eq_ignore_ascii_case(title) {
                start = Some(idx + 1);
                break;
            }
        }
    }

    let start = start?;
    let mut out = Vec::new();
    for line in lines.iter().skip(start) {
        if header_re.is_match(line.trim()) {
            break;
        }
        out.push(*line);
    }

    Some(out.join("\n").trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrate_main_spec_from_markdown_headings() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("llmanspec/specs/sample/spec.md");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }

        let source = r#"---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - tests
---

# Sample Specification

## Purpose
Describe behavior.

## Requirements
### Requirement: Existing behavior
System MUST preserve existing behavior.

#### Scenario: baseline
- **WHEN** run
- **THEN** ok
"#;
        fs::write(&path, source).expect("write");

        run_with_root(dir.path(), false).expect("migrate");

        let output = fs::read_to_string(&path).expect("read");
        assert!(output.contains("```ison"));
        assert!(output.contains("\"kind\": \"llman.sdd.spec\""));
        assert!(output.contains("\"req_id\": \"existing-behavior\""));
    }

    #[test]
    fn migrate_delta_spec_from_markdown_headings() {
        let dir = tempdir().expect("tempdir");
        let path = dir
            .path()
            .join("llmanspec/changes/add-sample/specs/sample/spec.md");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }

        let source = r#"## ADDED Requirements
### Requirement: Added behavior
System MUST support added behavior.

#### Scenario: added
- **WHEN** new action
- **THEN** done
"#;
        fs::write(&path, source).expect("write");

        run_with_root(dir.path(), false).expect("migrate");

        let output = fs::read_to_string(&path).expect("read");
        assert!(output.contains("```ison"));
        assert!(output.contains("\"kind\": \"llman.sdd.delta\""));
        assert!(output.contains("\"op\": \"add_requirement\""));
    }

    #[test]
    fn dry_run_does_not_write() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("llmanspec/specs/sample/spec.md");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        let source = "## Purpose\nX\n\n## Requirements\n";
        fs::write(&path, source).expect("write");

        run_with_root(dir.path(), true).expect("dry run");

        let output = fs::read_to_string(&path).expect("read");
        assert_eq!(output, source);
    }
}
