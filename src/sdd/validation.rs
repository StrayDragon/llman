use crate::sdd::delta::{
    DeltaPlan, RequirementBlock, normalize_requirement_name, parse_delta_spec,
};
use crate::sdd::parser::{Requirement, parse_spec};
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ValidationLevel {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub level: ValidationLevel,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationSummary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub summary: ValidationSummary,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SpecFrontmatter {
    pub valid_scope: Vec<String>,
    pub valid_commands: Vec<String>,
    pub evidence: Vec<String>,
}

pub struct SpecValidation {
    pub report: ValidationReport,
    pub frontmatter: Option<SpecFrontmatter>,
}

pub fn validate_spec_content_with_frontmatter(
    path: &Path,
    content: &str,
    strict: bool,
) -> SpecValidation {
    let spec_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("spec")
        .to_string();

    let parsed_frontmatter = parse_spec_frontmatter(path, content);
    let mut issues = parsed_frontmatter.issues;
    let body = parsed_frontmatter.body.clone();

    match parse_spec(&body, &spec_name) {
        Ok(spec) => {
            issues.extend(validate_requirements(&spec.requirements, &spec_name));
            SpecValidation {
                report: build_report(issues, strict),
                frontmatter: parsed_frontmatter.frontmatter,
            }
        }
        Err(err) => {
            let missing = missing_sections(&body);
            let message = if missing.is_empty() {
                err.to_string()
            } else {
                let sections = missing.join(", ");
                format!(
                    "{}\n{}",
                    t!("sdd.validate.sections_missing", sections = sections),
                    t!("sdd.validate.sections_example")
                )
            };
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: "file".to_string(),
                message,
            });
            SpecValidation {
                report: build_report(issues, strict),
                frontmatter: parsed_frontmatter.frontmatter,
            }
        }
    }
}

struct FrontmatterParse {
    frontmatter: Option<SpecFrontmatter>,
    body: String,
    issues: Vec<ValidationIssue>,
}

fn parse_spec_frontmatter(_path: &Path, content: &str) -> FrontmatterParse {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let mut issues = Vec::new();

    if !normalized.starts_with("---\n") {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: "frontmatter".to_string(),
            message: t!("sdd.validate.frontmatter_missing").to_string(),
        });
        return FrontmatterParse {
            frontmatter: None,
            body: normalized,
            issues,
        };
    }

    let mut lines = normalized.lines();
    let mut yaml_lines = Vec::new();
    let mut reached_end = false;

    lines.next();
    for line in lines.by_ref() {
        if line.trim() == "---" {
            reached_end = true;
            break;
        }
        yaml_lines.push(line);
    }

    if !reached_end {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: "frontmatter".to_string(),
            message: t!("sdd.validate.frontmatter_unterminated").to_string(),
        });
        return FrontmatterParse {
            frontmatter: None,
            body: normalized,
            issues,
        };
    }

    let yaml = yaml_lines.join("\n");
    let body = lines.collect::<Vec<_>>().join("\n");

    let parsed: serde_yaml::Value = match serde_yaml::from_str(&yaml) {
        Ok(value) => value,
        Err(err) => {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: "frontmatter".to_string(),
                message: t!("sdd.validate.frontmatter_parse_error", error = err).to_string(),
            });
            return FrontmatterParse {
                frontmatter: None,
                body,
                issues,
            };
        }
    };

    let scope = parse_frontmatter_list(&parsed, "llman_spec_valid_scope", &mut issues);
    let commands = parse_frontmatter_list(&parsed, "llman_spec_valid_commands", &mut issues);
    let evidence = parse_frontmatter_list(&parsed, "llman_spec_evidence", &mut issues);

    let frontmatter = if issues.is_empty() {
        Some(SpecFrontmatter {
            valid_scope: scope,
            valid_commands: commands,
            evidence,
        })
    } else {
        None
    };

    FrontmatterParse {
        frontmatter,
        body,
        issues,
    }
}

fn parse_frontmatter_list(
    doc: &serde_yaml::Value,
    key: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<String> {
    let value = match doc.get(key) {
        Some(value) => value,
        None => {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("frontmatter.{}", key),
                message: t!("sdd.validate.frontmatter_key_missing", key = key).to_string(),
            });
            return Vec::new();
        }
    };

    let mut items = Vec::new();
    match value {
        serde_yaml::Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    path: format!("frontmatter.{}", key),
                    message: t!("sdd.validate.frontmatter_value_empty", key = key).to_string(),
                });
            } else {
                items.extend(split_csv(trimmed));
            }
        }
        serde_yaml::Value::Sequence(values) => {
            if values.is_empty() {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    path: format!("frontmatter.{}", key),
                    message: t!("sdd.validate.frontmatter_value_empty", key = key).to_string(),
                });
            }
            for value in values {
                match value {
                    serde_yaml::Value::String(value) => {
                        let trimmed = value.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        items.extend(split_csv(trimmed));
                    }
                    _ => {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Error,
                            path: format!("frontmatter.{}", key),
                            message: t!("sdd.validate.frontmatter_value_invalid", key = key)
                                .to_string(),
                        });
                    }
                }
            }
        }
        _ => {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("frontmatter.{}", key),
                message: t!("sdd.validate.frontmatter_value_invalid", key = key).to_string(),
            });
        }
    }

    items
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim().trim_matches('"').trim_matches('\''))
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_missing_is_error() {
        let content = "## Purpose\nTest\n\n## Requirements\n";
        let result = parse_spec_frontmatter(Path::new("spec.md"), content);
        assert!(!result.issues.is_empty());
        assert!(result.frontmatter.is_none());
    }

    #[test]
    fn frontmatter_parses_string_and_list() {
        let content = r#"---
llman_spec_valid_scope: "src, tests"
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence: "CI #123"
---

## Purpose
Test

## Requirements
"#;
        let result = parse_spec_frontmatter(Path::new("spec.md"), content);
        assert!(result.issues.is_empty());
        let frontmatter = result.frontmatter.expect("frontmatter");
        assert_eq!(
            frontmatter.valid_scope,
            vec!["src".to_string(), "tests".to_string()]
        );
        assert_eq!(frontmatter.valid_commands, vec!["cargo test".to_string()]);
        assert_eq!(frontmatter.evidence, vec!["CI #123".to_string()]);
    }
}

pub fn validate_change_delta_specs(change_dir: &Path, strict: bool) -> ValidationReport {
    let mut issues = Vec::new();
    let specs_dir = change_dir.join("specs");
    let mut total_deltas = 0usize;
    if !specs_dir.exists() {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: "specs".to_string(),
            message: delta_missing_message(),
        });
        return build_report(issues, strict);
    }

    let entries = match fs::read_dir(&specs_dir) {
        Ok(entries) => entries,
        Err(err) => {
            return report_with_issue(
                ValidationIssue {
                    level: ValidationLevel::Error,
                    path: "specs".to_string(),
                    message: err.to_string(),
                },
                strict,
            );
        }
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let spec_name = entry.file_name().to_string_lossy().to_string();
        let spec_file = entry.path().join("spec.md");
        if !spec_file.exists() {
            continue;
        }
        let content = match fs::read_to_string(&spec_file) {
            Ok(content) => content,
            Err(err) => {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    path: format!("{}/spec.md", spec_name),
                    message: err.to_string(),
                });
                continue;
            }
        };
        let plan = match parse_delta_spec(&content) {
            Ok(plan) => plan,
            Err(err) => {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    path: format!("{}/spec.md", spec_name),
                    message: err.to_string(),
                });
                continue;
            }
        };

        total_deltas +=
            plan.added.len() + plan.modified.len() + plan.removed.len() + plan.renamed.len();
        issues.extend(validate_delta_plan(&spec_name, &plan));
    }

    if total_deltas == 0 {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: "specs".to_string(),
            message: delta_missing_message(),
        });
    }

    build_report(issues, strict)
}

fn validate_requirements(requirements: &[Requirement], spec_name: &str) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for (idx, req) in requirements.iter().enumerate() {
        if !contains_shall_or_must(&req.text) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{}/requirements[{}]", spec_name, idx),
                message: format!("Requirement must contain SHALL or MUST: {}", req.text),
            });
        }
        if req.scenarios.is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{}/requirements[{}]", spec_name, idx),
                message: scenario_missing_message(),
            });
        }
    }
    issues
}

fn validate_delta_plan(spec_name: &str, plan: &DeltaPlan) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let entry_path = format!("{}/spec.md", spec_name);

    let mut added_names = Vec::new();
    let mut modified_names = Vec::new();
    let mut removed_names = Vec::new();
    let mut renamed_from = Vec::new();
    let mut renamed_to = Vec::new();

    for block in &plan.added {
        issues.extend(validate_requirement_block(&entry_path, "ADDED", block));
        added_names.push(normalize_requirement_name(&block.name));
    }
    for block in &plan.modified {
        issues.extend(validate_requirement_block(&entry_path, "MODIFIED", block));
        modified_names.push(normalize_requirement_name(&block.name));
    }
    for name in &plan.removed {
        removed_names.push(normalize_requirement_name(name));
    }
    for pair in &plan.renamed {
        renamed_from.push(normalize_requirement_name(&pair.from));
        renamed_to.push(normalize_requirement_name(&pair.to));
    }

    issues.extend(check_duplicates(&entry_path, "ADDED", &added_names));
    issues.extend(check_duplicates(&entry_path, "MODIFIED", &modified_names));
    issues.extend(check_duplicates(&entry_path, "REMOVED", &removed_names));
    issues.extend(check_duplicates(&entry_path, "RENAMED FROM", &renamed_from));
    issues.extend(check_duplicates(&entry_path, "RENAMED TO", &renamed_to));

    for name in &modified_names {
        if removed_names.contains(name) || added_names.contains(name) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: format!("Requirement present in multiple sections: {}", name),
            });
        }
    }
    for name in &added_names {
        if removed_names.contains(name) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: format!("Requirement present in multiple sections: {}", name),
            });
        }
    }

    if plan.section_presence.renamed && plan.renamed.is_empty() {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: entry_path.clone(),
            message: "RENAMED section must include FROM/TO pairs".to_string(),
        });
    }

    issues
}

fn validate_requirement_block(
    path: &str,
    section: &str,
    block: &RequirementBlock,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if let Some(text) = extract_requirement_text(&block.raw) {
        if !contains_shall_or_must(&text) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: path.to_string(),
                message: format!("{} \"{}\" must contain SHALL or MUST", section, block.name),
            });
        }
    } else {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: path.to_string(),
            message: format!("{} \"{}\" is missing requirement text", section, block.name),
        });
    }

    let scenario_count = count_scenarios(&block.raw);
    if scenario_count < 1 {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: path.to_string(),
            message: format!(
                "{} \"{}\": {}",
                section,
                block.name,
                scenario_missing_message()
            ),
        });
    }

    issues
}

fn extract_requirement_text(raw: &str) -> Option<String> {
    let mut lines = raw.lines();
    lines.next();
    let mut text_lines = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("#### ") {
            break;
        }
        text_lines.push(line);
    }
    let text = text_lines.join("\n").trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

fn count_scenarios(raw: &str) -> usize {
    let re = Regex::new(r"^####\s+Scenario:").expect("regex");
    raw.lines()
        .filter(|line| re.is_match(line.trim_start()))
        .count()
}

fn contains_shall_or_must(text: &str) -> bool {
    text.contains("SHALL") || text.contains("MUST")
}

fn missing_sections(content: &str) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !has_section(content, "Purpose") {
        missing.push("## Purpose");
    }
    if !has_section(content, "Requirements") {
        missing.push("## Requirements");
    }
    missing
}

fn has_section(content: &str, name: &str) -> bool {
    let prefix = format!("## {}", name);
    content
        .lines()
        .any(|line| line.trim_start().starts_with(&prefix))
}

fn scenario_missing_message() -> String {
    format!(
        "{}\n{}",
        t!("sdd.validate.scenario_missing"),
        t!("sdd.validate.scenario_example")
    )
}

fn delta_missing_message() -> String {
    format!(
        "{}\n{}",
        t!("sdd.validate.delta_missing"),
        t!("sdd.validate.delta_example")
    )
}

fn check_duplicates(path: &str, label: &str, names: &[String]) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for name in names {
        if !seen.insert(name) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: path.to_string(),
                message: format!("Duplicate requirement in {}: {}", label, name),
            });
        }
    }
    issues
}

fn build_report(issues: Vec<ValidationIssue>, strict: bool) -> ValidationReport {
    let mut errors = 0;
    let mut warnings = 0;
    let mut info = 0;
    let mut normalized = Vec::new();

    for issue in issues {
        let level = match issue.level {
            ValidationLevel::Warning if strict => ValidationLevel::Error,
            level => level,
        };
        match level {
            ValidationLevel::Error => errors += 1,
            ValidationLevel::Warning => warnings += 1,
            ValidationLevel::Info => info += 1,
        }
        normalized.push(ValidationIssue {
            level,
            path: issue.path,
            message: issue.message,
        });
    }

    ValidationReport {
        valid: errors == 0,
        issues: normalized,
        summary: ValidationSummary {
            errors,
            warnings,
            info,
        },
    }
}

fn report_with_issue(issue: ValidationIssue, strict: bool) -> ValidationReport {
    build_report(vec![issue], strict)
}
