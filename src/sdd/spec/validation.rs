use crate::sdd::spec::backend::{SpecBackend, BACKEND};
use crate::sdd::spec::frontmatter::split_frontmatter;
use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
use serde::Serialize;
use std::collections::HashMap;
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

#[derive(Debug, Clone, Default)]
pub struct ProposalFrontmatter {
    pub depends_on: Vec<String>,
    pub blocks: Vec<String>,
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

    let context = format!("spec `{}`", spec_name);
    match BACKEND.parse_main_spec(&body, &context) {
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

    // --- Change-level validation tests ---

    fn setup_change_dir(tmp: &tempfile::TempDir, files: &[(&str, &str)]) -> std::path::PathBuf {
        let change_dir = tmp.path().join("test-change");
        fs::create_dir_all(&change_dir).unwrap();
        for (name, content) in files {
            let path = change_dir.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, content).unwrap();
        }
        change_dir
    }

    #[test]
    fn proposal_missing_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[]);
        let issues = check_proposal_exists(&change_dir);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Error);
        assert!(issues[0].message.contains("proposal.md"));
    }

    #[test]
    fn proposal_present_no_error() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[("proposal.md", "## Why\nTest")]);
        let issues = check_proposal_exists(&change_dir);
        assert!(issues.is_empty());
    }

    #[test]
    fn proposal_frontmatter_valid_depends_on() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[(
                "proposal.md",
                "---\ndepends_on:\n  - other-change\n---\n## Why\nTest",
            )],
        );
        let all_ids = vec!["other-change".to_string(), "test-change".to_string()];
        let (issues, fm) = check_proposal_frontmatter(&change_dir, &all_ids);
        assert!(issues.is_empty());
        assert_eq!(fm.depends_on, vec!["other-change"]);
    }

    #[test]
    fn proposal_frontmatter_unknown_depends_on() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[(
                "proposal.md",
                "---\ndepends_on:\n  - nonexistent\n---\n## Why\nTest",
            )],
        );
        let all_ids = vec!["test-change".to_string()];
        let (issues, _) = check_proposal_frontmatter(&change_dir, &all_ids);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Error);
        assert!(issues[0].message.contains("nonexistent"));
    }

    #[test]
    fn proposal_frontmatter_no_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[("proposal.md", "## Why\nTest")]);
        let all_ids = vec!["test-change".to_string()];
        let (issues, fm) = check_proposal_frontmatter(&change_dir, &all_ids);
        assert!(issues.is_empty());
        assert!(fm.depends_on.is_empty());
    }

    #[test]
    fn proposal_frontmatter_invalid_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[(
                "proposal.md",
                "---\ndepends_on: [not closed\n---\n## Why\nTest",
            )],
        );
        let all_ids = vec!["test-change".to_string()];
        let (issues, _) = check_proposal_frontmatter(&change_dir, &all_ids);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Error);
    }

    #[test]
    fn tasks_missing_is_warning() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[("proposal.md", "## Why\nTest")]);
        let issues = check_tasks_exists(&change_dir);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Warning);
    }

    #[test]
    fn tasks_present_no_warning() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", "## Why\nTest"),
                ("tasks.md", "- [ ] Do thing"),
            ],
        );
        let issues = check_tasks_exists(&change_dir);
        assert!(issues.is_empty());
    }

    #[test]
    fn design_present_is_info() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", "## Why\nTest"),
                ("design.md", "# Design\nTradeoffs here"),
            ],
        );
        let issues = check_design_md(&change_dir);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Info);
    }

    #[test]
    fn design_absent_no_issue() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[("proposal.md", "## Why\nTest")]);
        let issues = check_design_md(&change_dir);
        assert!(issues.is_empty());
    }

    #[test]
    fn specs_missing_no_error() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[("proposal.md", "## Why\nTest")]);
        let report = validate_change_delta_specs(&change_dir, false);
        assert!(report.valid);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn dag_cycle_detected() {
        let frontmatters = vec![
            (
                "a".to_string(),
                ProposalFrontmatter {
                    depends_on: vec!["b".to_string()],
                    blocks: vec![],
                },
            ),
            (
                "b".to_string(),
                ProposalFrontmatter {
                    depends_on: vec!["a".to_string()],
                    blocks: vec![],
                },
            ),
        ];
        let issues_map = check_dag_cycles(&frontmatters);
        assert!(!issues_map.is_empty());
        assert!(issues_map.contains_key("a"));
        assert!(issues_map.contains_key("b"));
        assert_eq!(issues_map["a"][0].level, ValidationLevel::Error);
    }

    #[test]
    fn dag_no_cycle_ok() {
        let frontmatters = vec![
            (
                "a".to_string(),
                ProposalFrontmatter {
                    depends_on: vec![],
                    blocks: vec![],
                },
            ),
            (
                "b".to_string(),
                ProposalFrontmatter {
                    depends_on: vec!["a".to_string()],
                    blocks: vec![],
                },
            ),
        ];
        let issues_map = check_dag_cycles(&frontmatters);
        assert!(issues_map.is_empty());
    }
}

pub fn validate_change_delta_specs(change_dir: &Path, strict: bool) -> ValidationReport {
    let mut issues = Vec::new();
    let specs_dir = change_dir.join("specs");
    let mut total_deltas = 0usize;
    if !specs_dir.exists() {
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
        let doc = match BACKEND.parse_delta_spec(&content, &format!("delta spec `{}`", spec_name)) {
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

    let _ = total_deltas;
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

// --- Change-level validation check functions ---

pub fn check_proposal_exists(change_dir: &Path) -> Vec<ValidationIssue> {
    if change_dir.join("proposal.md").exists() {
        return Vec::new();
    }
    vec![ValidationIssue {
        level: ValidationLevel::Error,
        path: "proposal.md".to_string(),
        message: t!("sdd.validate.proposal_missing").to_string(),
    }]
}

pub fn check_proposal_frontmatter(
    change_dir: &Path,
    all_change_ids: &[String],
) -> (Vec<ValidationIssue>, ProposalFrontmatter) {
    let content = match fs::read_to_string(change_dir.join("proposal.md")) {
        Ok(content) => content,
        Err(_) => return (Vec::new(), ProposalFrontmatter::default()),
    };

    let (yaml_str, _body) = split_frontmatter(&content);
    let Some(yaml_str) = yaml_str else {
        return (Vec::new(), ProposalFrontmatter::default());
    };

    let parsed: serde_yaml::Value = match serde_yaml::from_str(&yaml_str) {
        Ok(value) => value,
        Err(err) => {
            return (
                vec![ValidationIssue {
                    level: ValidationLevel::Error,
                    path: "proposal.md/frontmatter".to_string(),
                    message: t!(
                        "sdd.validate.proposal_frontmatter_invalid_yaml",
                        error = err
                    )
                    .to_string(),
                }],
                ProposalFrontmatter::default(),
            );
        }
    };

    let mut issues = Vec::new();
    let known_ids: std::collections::HashSet<&str> =
        all_change_ids.iter().map(|s| s.as_str()).collect();

    let depends_on = parse_yaml_string_list(&parsed, "depends_on", &mut issues);
    let blocks = parse_yaml_string_list(&parsed, "blocks", &mut issues);

    for id in &depends_on {
        if !known_ids.contains(id.as_str()) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: "proposal.md/frontmatter.depends_on".to_string(),
                message: t!("sdd.validate.proposal_depends_on_unknown", id = id).to_string(),
            });
        }
    }

    for id in &blocks {
        if !known_ids.contains(id.as_str()) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: "proposal.md/frontmatter.blocks".to_string(),
                message: t!("sdd.validate.proposal_blocks_unknown", id = id).to_string(),
            });
        }
    }

    (issues, ProposalFrontmatter { depends_on, blocks })
}

fn parse_yaml_string_list(
    doc: &serde_yaml::Value,
    key: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<String> {
    let Some(value) = doc.get(key) else {
        return Vec::new();
    };
    match value {
        serde_yaml::Value::Sequence(values) => {
            let mut result = Vec::new();
            for item in values {
                match item {
                    serde_yaml::Value::String(s) => {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() {
                            result.push(trimmed.to_string());
                        }
                    }
                    _ => {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Error,
                            path: format!("proposal.md/frontmatter.{}", key),
                            message: t!("sdd.validate.proposal_depends_on_format").to_string(),
                        });
                        return Vec::new();
                    }
                }
            }
            result
        }
        _ => {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("proposal.md/frontmatter.{}", key),
                message: if key == "depends_on" {
                    t!("sdd.validate.proposal_depends_on_format").to_string()
                } else {
                    t!("sdd.validate.proposal_blocks_format").to_string()
                },
            });
            Vec::new()
        }
    }
}

pub fn check_dag_cycles(
    change_frontmatters: &[(String, ProposalFrontmatter)],
) -> HashMap<String, Vec<ValidationIssue>> {
    let mut result: HashMap<String, Vec<ValidationIssue>> = HashMap::new();

    // Build owned adjacency list: change_id -> Vec<String> of dependencies
    let graph: HashMap<String, Vec<String>> = change_frontmatters
        .iter()
        .map(|(id, fm)| (id.clone(), fm.depends_on.clone()))
        .collect();
    let all_ids: std::collections::HashSet<String> = change_frontmatters
        .iter()
        .map(|(id, _)| id.clone())
        .collect();

    // Three-color DFS: WHITE=unvisited, GRAY=on stack, BLACK=done
    #[derive(Clone, Copy, PartialEq)]
    enum Color {
        White,
        Gray,
        Black,
    }
    let mut colors: HashMap<String, Color> = all_ids
        .iter()
        .map(|id| (id.clone(), Color::White))
        .collect();

    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        colors: &mut HashMap<String, Color>,
        result: &mut HashMap<String, Vec<ValidationIssue>>,
        path: &mut Vec<String>,
    ) {
        colors.insert(node.to_string(), Color::Gray);
        path.push(node.to_string());

        if let Some(deps) = graph.get(node) {
            for dep in deps {
                if !colors.contains_key(dep.as_str()) {
                    continue;
                }
                match colors.get(dep) {
                    Some(Color::Gray) => {
                        let cycle_start = path.iter().position(|p| p == dep).unwrap_or(0);
                        let cycle: Vec<&str> =
                            path[cycle_start..].iter().map(|s| s.as_str()).collect();
                        let cycle_str = cycle.join(" -> ");
                        let issue = ValidationIssue {
                            level: ValidationLevel::Error,
                            path: "proposal.md/frontmatter.depends_on".to_string(),
                            message: t!("sdd.validate.dag_cycle_detected", cycle = cycle_str)
                                .to_string(),
                        };
                        for node_id in &cycle {
                            result
                                .entry(node_id.to_string())
                                .or_default()
                                .push(issue.clone());
                        }
                    }
                    Some(Color::White) => {
                        dfs(dep, graph, colors, result, path);
                    }
                    Some(Color::Black) | None => {}
                }
            }
        }

        path.pop();
        colors.insert(node.to_string(), Color::Black);
    }

    for id in &all_ids {
        if colors.get(id) == Some(&Color::White) {
            dfs(id, &graph, &mut colors, &mut result, &mut Vec::new());
        }
    }

    result
}

pub fn check_tasks_exists(change_dir: &Path) -> Vec<ValidationIssue> {
    if change_dir.join("tasks.md").exists() {
        return Vec::new();
    }
    vec![ValidationIssue {
        level: ValidationLevel::Warning,
        path: "tasks.md".to_string(),
        message: t!("sdd.validate.tasks_missing").to_string(),
    }]
}

pub fn check_design_md(change_dir: &Path) -> Vec<ValidationIssue> {
    if !change_dir.join("design.md").exists() {
        return Vec::new();
    }
    vec![ValidationIssue {
        level: ValidationLevel::Info,
        path: "design.md".to_string(),
        message: t!("sdd.validate.design_present").to_string(),
    }]
}

pub fn build_report(issues: Vec<ValidationIssue>, strict: bool) -> ValidationReport {
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
