use crate::sdd::project::config::SpecStyle;
use crate::sdd::spec::backend::backend_for_style;
use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
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
    style: SpecStyle,
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

    let context = format!("spec `{}`", spec_name);
    let backend = backend_for_style(style);
    match backend.parse_main_spec(&body, &context) {
        Ok(doc) => {
            if doc.name.trim() != spec_name {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    path: format!("{}/meta.name", spec_name),
                    message: format!(
                        "Spec feature id must match spec directory name: `{}` != `{}`",
                        doc.name.trim(),
                        spec_name
                    ),
                });
            }

            issues.extend(validate_main_spec_doc(&doc, &spec_name));
            SpecValidation {
                report: build_report(issues, strict),
                frontmatter: parsed_frontmatter.frontmatter,
            }
        }
        Err(err) => {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: "file".to_string(),
                message: err.to_string(),
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

#[allow(clippy::items_after_test_module)]
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

pub fn validate_change_delta_specs(
    change_dir: &Path,
    style: SpecStyle,
    strict: bool,
) -> ValidationReport {
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
        let backend = backend_for_style(style);
        let doc = match backend.parse_delta_spec(&content, &format!("delta spec `{}`", spec_name)) {
            Ok(doc) => doc,
            Err(err) => {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    path: format!("{}/spec.md", spec_name),
                    message: err.to_string(),
                });
                continue;
            }
        };

        total_deltas += doc.ops.len();
        issues.extend(validate_delta_doc(&spec_name, &doc));
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

fn validate_main_spec_doc(doc: &MainSpecDoc, spec_name: &str) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    let mut req_id_seen = std::collections::HashSet::new();
    for (idx, req) in doc.requirements.iter().enumerate() {
        if !req_id_seen.insert(req.req_id.trim()) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{}/requirements[{}]", spec_name, idx),
                message: format!("Duplicate requirement req_id: {}", req.req_id),
            });
        }

        if !contains_shall_or_must(req.statement.trim()) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{}/requirements[{}]", spec_name, idx),
                message: format!(
                    "Requirement must contain SHALL or MUST: {}",
                    req.statement.trim()
                ),
            });
        }
    }

    let mut scenario_key_seen = std::collections::HashSet::new();
    let mut scenarios_by_req: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();

    for scenario in &doc.scenarios {
        let req_id = scenario.req_id.trim();
        if !req_id_seen.contains(req_id) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{spec_name}/scenarios"),
                message: format!(
                    "Scenario references unknown requirement `req_id` `{}`",
                    scenario.req_id
                ),
            });
        }

        let scenario_id = scenario.id.trim();
        let key = format!("{}::{}", req_id, scenario_id);
        if !scenario_key_seen.insert(key) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{spec_name}/scenarios"),
                message: format!(
                    "Duplicate scenario `(req_id, id)` = (`{}`, `{}`)",
                    scenario.req_id, scenario.id
                ),
            });
        }

        if scenario.when_.trim().is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{spec_name}/scenarios"),
                message: "Scenario `when` must not be empty".to_string(),
            });
        }
        if scenario.then_.trim().is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{spec_name}/scenarios"),
                message: "Scenario `then` must not be empty".to_string(),
            });
        }

        *scenarios_by_req.entry(req_id).or_insert(0) += 1;
    }

    for (idx, req) in doc.requirements.iter().enumerate() {
        let count = scenarios_by_req
            .get(req.req_id.trim())
            .copied()
            .unwrap_or(0);
        if count == 0 {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{}/requirements[{}]", spec_name, idx),
                message: scenario_missing_message(),
            });
        }
    }

    issues
}

fn contains_shall_or_must(text: &str) -> bool {
    text.contains("SHALL") || text.contains("MUST")
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

fn validate_delta_doc(spec_name: &str, doc: &DeltaSpecDoc) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let entry_path = format!("{}/spec.md", spec_name);

    if doc.kind.trim() != "llman.sdd.delta" {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: entry_path.clone(),
            message: format!(
                "Delta spec kind must be `llman.sdd.delta`, got `{}`",
                doc.kind.trim()
            ),
        });
    }

    let mut op_by_req: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for op in &doc.ops {
        let req_id = op.req_id.trim();
        if req_id.is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: "Delta op `req_id` must not be empty".to_string(),
            });
            continue;
        }
        if op_by_req.insert(req_id, op.op.trim()).is_some() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: format!("Duplicate op for req_id: {}", op.req_id),
            });
        }

        let op_kind = op.op.trim().to_ascii_lowercase();
        match op_kind.as_str() {
            "add_requirement" | "modify_requirement" => {
                if op.title.as_deref().unwrap_or("").trim().is_empty() {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: entry_path.clone(),
                        message: format!(
                            "{} op for req_id `{}` is missing `title`",
                            op_kind, op.req_id
                        ),
                    });
                }
                let statement = op.statement.as_deref().unwrap_or("").trim();
                if statement.is_empty() {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: entry_path.clone(),
                        message: format!(
                            "{} op for req_id `{}` is missing `statement`",
                            op_kind, op.req_id
                        ),
                    });
                } else if !contains_shall_or_must(statement) {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: entry_path.clone(),
                        message: format!(
                            "{} op for req_id `{}`: statement must contain SHALL or MUST",
                            op_kind, op.req_id
                        ),
                    });
                }
                if op.from.is_some() || op.to.is_some() || op.name.is_some() {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: entry_path.clone(),
                        message: format!(
                            "{} op for req_id `{}` must not set from/to/name",
                            op_kind, op.req_id
                        ),
                    });
                }
            }
            "remove_requirement" => {
                if op.title.is_some()
                    || op.statement.is_some()
                    || op.from.is_some()
                    || op.to.is_some()
                {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: entry_path.clone(),
                        message: format!(
                            "remove_requirement op for req_id `{}` must not set title/statement/from/to",
                            op.req_id
                        ),
                    });
                }
            }
            "rename_requirement" => {
                if op.title.is_some() || op.statement.is_some() || op.name.is_some() {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: entry_path.clone(),
                        message: format!(
                            "rename_requirement op for req_id `{}` must not set title/statement/name",
                            op.req_id
                        ),
                    });
                }
                if op.from.as_deref().unwrap_or("").trim().is_empty()
                    || op.to.as_deref().unwrap_or("").trim().is_empty()
                {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: entry_path.clone(),
                        message: format!(
                            "rename_requirement op for req_id `{}` must include non-empty from/to",
                            op.req_id
                        ),
                    });
                }
            }
            _ => {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    path: entry_path.clone(),
                    message: format!(
                        "Unsupported op `{}` (expected add_requirement/modify_requirement/remove_requirement/rename_requirement)",
                        op.op
                    ),
                });
            }
        }
    }

    let mut scenario_key_seen = std::collections::HashSet::new();
    let mut scenario_count_by_req: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for scenario in &doc.op_scenarios {
        let req_id = scenario.req_id.trim();
        if req_id.is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: "Delta op scenario req_id must not be empty".to_string(),
            });
            continue;
        }
        let Some(op_kind) = op_by_req.get(req_id).copied() else {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: format!(
                    "op_scenarios references unknown `req_id` `{}` (must match an add/modify op)",
                    scenario.req_id
                ),
            });
            continue;
        };
        let op_kind = op_kind.to_ascii_lowercase();
        if op_kind != "add_requirement" && op_kind != "modify_requirement" {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: format!(
                    "op_scenarios is only allowed for add/modify ops; found `{}` for `req_id` `{}`",
                    op_kind, scenario.req_id
                ),
            });
        }

        let scenario_id = scenario.id.trim();
        if scenario_id.is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: "Delta op scenario `id` must not be empty".to_string(),
            });
        }
        let key = format!("{}::{}", req_id, scenario_id);
        if !scenario_key_seen.insert(key) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: format!(
                    "Duplicate delta scenario `(req_id, id)` = (`{}`, `{}`)",
                    scenario.req_id, scenario.id
                ),
            });
        }

        if scenario.when_.trim().is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: "Delta op scenario `when` must not be empty".to_string(),
            });
        }
        if scenario.then_.trim().is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: entry_path.clone(),
                message: "Delta op scenario `then` must not be empty".to_string(),
            });
        }

        *scenario_count_by_req.entry(req_id).or_insert(0) += 1;
    }

    for (req_id, op_kind) in op_by_req {
        let op_kind = op_kind.to_ascii_lowercase();
        if op_kind != "add_requirement" && op_kind != "modify_requirement" {
            continue;
        }
        let count = scenario_count_by_req.get(req_id).copied().unwrap_or(0);
        if count < 1 {
            issues.push(ValidationIssue {
                level: ValidationLevel::Warning,
                path: entry_path.clone(),
                message: scenario_missing_message(),
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
