use crate::sdd::project::config::{ArchiveConfig, BddConfig};
use crate::sdd::shared::constants::SPEC_FILE;
use crate::sdd::shared::tasks::{self, TaskStatus};
use crate::sdd::spec::backend::{BACKEND, SpecBackend};
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
}

pub struct SpecValidation {
    pub report: ValidationReport,
    pub frontmatter: Option<SpecFrontmatter>,
}

#[derive(Debug, Clone, Default)]
pub struct ProposalFrontmatter {
    pub depends_on: Vec<String>,
    pub blocks: Vec<String>,
    /// BDD-on Git-native binding: feature branch name.
    pub branch: Option<String>,
    /// BDD-on Git-native binding: immutable merge-base SHA at attach time.
    pub base_sha: Option<String>,
    /// Whether `sdd change checkpoint` has succeeded.
    pub checkpointed: bool,
    pub checkpoint_sha: Option<String>,
}

pub fn validate_spec_content_with_frontmatter(
    path: &Path,
    content: &str,
    strict: bool,
) -> SpecValidation {
    validate_spec_content_with_frontmatter_and_bdd(
        path, content, strict, None, None, None, false, None,
    )
}

/// Cache of BDD full-mode results keyed by the expanded `run_command` string.
/// Used by bulk validate (`--all` / `--specs`) so project-wide runners without
/// differentiating `{feature_*}` placeholders execute at most once per process.
#[derive(Debug, Clone)]
pub struct FullModeCacheEntry {
    pub success: bool,
    pub issues: Vec<ValidationIssue>,
}

pub type FullModeCache = HashMap<String, FullModeCacheEntry>;

#[allow(clippy::too_many_arguments)]
pub fn validate_spec_content_with_frontmatter_and_bdd(
    path: &Path,
    content: &str,
    strict: bool,
    project_root: Option<&Path>,
    bdd_config: Option<&BddConfig>,
    locale: Option<&str>,
    check_mode: bool,
    full_mode_cache: Option<&mut FullModeCache>,
) -> SpecValidation {
    let spec_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("spec")
        .to_string();

    let context = format!("spec `{}`", spec_name);
    let bdd_enabled = bdd_config.is_some();

    // Specs are standalone TOON documents: parse the whole file directly.
    let parse_result = if strict {
        BACKEND.parse_main_spec_strict(content, &context)
    } else {
        BACKEND.parse_main_spec(content, &context)
    };
    match parse_result {
        Ok(doc) => {
            let mut issues = Vec::new();

            // Validation scope lives inside the TOON document (valid_scope),
            // replacing the YAML frontmatter. Drives the staleness check.
            // Unified path: all specs (BDD-on or off) must declare valid_scope.
            validate_spec_meta(&doc, &spec_name, &mut issues);
            let frontmatter = if has_meta_errors(&issues) {
                None
            } else {
                Some(SpecFrontmatter {
                    valid_scope: doc.valid_scope.clone(),
                })
            };

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

            // BDD-on: Gherkin fast/full mode + Partitioned SSOT link/dual-write gates.
            if bdd_enabled
                && let Some(root) = project_root
                && let Some(spec_dir) = path.parent()
            {
                let lang = locale_to_gherkin_lang(locale, bdd_config);
                issues.extend(validate_features_dir(spec_dir, root, &lang));
                let harness = crate::sdd::spec::partitioned::load_spec_harness_soft(
                    spec_dir,
                    &lang,
                    &mut issues,
                );
                issues.extend(crate::sdd::spec::partitioned::validate_partitioned(
                    &spec_name, &doc, &harness, strict,
                ));
                issues.extend(validate_main_spec_doc_partitioned(
                    &doc, &spec_name, &harness,
                ));
                if check_mode && let Some(bdd) = bdd_config {
                    issues.extend(run_full_mode_cached(spec_dir, bdd, full_mode_cache));
                }
            } else {
                issues.extend(validate_main_spec_doc(&doc, &spec_name));
            }

            SpecValidation {
                report: build_report(issues, strict),
                frontmatter,
            }
        }
        Err(err) => {
            let issues = vec![ValidationIssue {
                level: ValidationLevel::Error,
                path: "file".to_string(),
                message: err.to_string(),
            }];
            SpecValidation {
                report: build_report(issues, strict),
                frontmatter: None,
            }
        }
    }
}

/// Validate the in-document scope (valid_scope). Must be present and non-empty
/// for a main spec; drives the staleness check.
fn validate_spec_meta(doc: &MainSpecDoc, spec_name: &str, issues: &mut Vec<ValidationIssue>) {
    validate_meta_list(&doc.valid_scope, spec_name, "valid_scope", issues);
}

fn validate_meta_list(
    list: &[String],
    spec_name: &str,
    key: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if list
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .count()
        == 0
    {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: format!("{spec_name}/{key}"),
            message: t!("sdd.validate.meta_field_empty", key = key).to_string(),
        });
    }
}

/// Whether any issue emitted so far is a valid_scope ERROR (used to suppress
/// populating `SpecFrontmatter` for staleness when scope is malformed).
fn has_meta_errors(issues: &[ValidationIssue]) -> bool {
    issues
        .iter()
        .any(|issue| issue.level == ValidationLevel::Error && issue.path.ends_with("/valid_scope"))
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_missing_is_error() {
        // A spec with no valid_scope is invalid.
        let doc = MainSpecDoc {
            kind: "llman.sdd.spec".to_string(),
            name: "sample".to_string(),
            purpose: "x".to_string(),
            valid_scope: Vec::new(),
            requirements: Vec::new(),
            scenarios: Vec::new(),
        };
        let mut issues = Vec::new();
        validate_spec_meta(&doc, "sample", &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(issues.iter().all(|i| i.level == ValidationLevel::Error));
    }

    #[test]
    fn meta_present_no_error() {
        let doc = MainSpecDoc {
            kind: "llman.sdd.spec".to_string(),
            name: "sample".to_string(),
            purpose: "x".to_string(),
            valid_scope: vec!["src/".to_string(), "tests/".to_string()],
            requirements: Vec::new(),
            scenarios: Vec::new(),
        };
        let mut issues = Vec::new();
        validate_spec_meta(&doc, "sample", &mut issues);
        assert!(issues.is_empty(), "{issues:?}");
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
        let (issues, fm) = check_proposal_frontmatter(&change_dir, &all_ids, &[], false);
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
        let (issues, _) = check_proposal_frontmatter(&change_dir, &all_ids, &[], false);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Error);
        assert!(issues[0].message.contains("nonexistent"));
    }

    #[test]
    fn proposal_frontmatter_no_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[("proposal.md", "## Why\nTest")]);
        let all_ids = vec!["test-change".to_string()];
        let (issues, fm) = check_proposal_frontmatter(&change_dir, &all_ids, &[], false);
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
        let (issues, _) = check_proposal_frontmatter(&change_dir, &all_ids, &[], false);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Error);
    }

    #[test]
    fn proposal_frontmatter_archived_depends_on_is_info() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[(
                "proposal.md",
                "---\ndepends_on:\n  - archived-change\n---\n## Why\nTest",
            )],
        );
        let active_ids = vec!["test-change".to_string()];
        let archived_ids = vec!["archived-change".to_string()];
        let (issues, fm) =
            check_proposal_frontmatter(&change_dir, &active_ids, &archived_ids, false);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Info);
        assert!(issues[0].message.contains("archived-change"));
        assert_eq!(fm.depends_on, vec!["archived-change"]);
    }

    #[test]
    fn proposal_frontmatter_archived_blocks_is_info() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[(
                "proposal.md",
                "---\nblocks:\n  - archived-change\n---\n## Why\nTest",
            )],
        );
        let active_ids = vec!["test-change".to_string()];
        let archived_ids = vec!["archived-change".to_string()];
        let (issues, fm) =
            check_proposal_frontmatter(&change_dir, &active_ids, &archived_ids, false);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Info);
        assert!(issues[0].message.contains("archived-change"));
        assert_eq!(fm.blocks, vec!["archived-change"]);
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
    fn task_completion_pending_is_warning_by_default() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", "## Why\nTest"),
                ("tasks.md", "- [x] Done\n- [ ] Pending"),
            ],
        );
        let config = ArchiveConfig::default();
        let issues = check_tasks_completion(&change_dir, &config);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Warning);
    }

    #[test]
    fn task_completion_pending_is_error_when_strict_defer() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", "## Why\nTest"),
                ("tasks.md", "- [ ] Pending task"),
            ],
        );
        let config = ArchiveConfig {
            strict_defer: Some(true),
            min_completion_ratio: None,
        };
        let issues = check_tasks_completion(&change_dir, &config);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Error);
    }

    #[test]
    fn task_completion_legacy_defer_is_warning() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", "## Why\nTest"),
                ("tasks.md", "- [ ] Old style (defer - some reason)"),
            ],
        );
        let config = ArchiveConfig::default();
        let issues = check_tasks_completion(&change_dir, &config);
        // Legacy annotations are now Pending, so they produce warnings
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Warning);
    }

    #[test]
    fn task_completion_cancelled_now_pending() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", "## Why\nTest"),
                (
                    "tasks.md",
                    "- [x] Done\n- [ ] Not needed (cancelled — done)",
                ),
            ],
        );
        let config = ArchiveConfig::default();
        // Cancelled tasks are now Pending, so they produce a warning
        let issues = check_tasks_completion(&change_dir, &config);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Warning);
    }

    #[test]
    fn task_completion_no_tasks_file_no_issues() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[("proposal.md", "## Why\nTest")]);
        let config = ArchiveConfig::default();
        let issues = check_tasks_completion(&change_dir, &config);
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
                    ..Default::default()
                },
            ),
            (
                "b".to_string(),
                ProposalFrontmatter {
                    depends_on: vec!["a".to_string()],
                    blocks: vec![],
                    ..Default::default()
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
                    ..Default::default()
                },
            ),
            (
                "b".to_string(),
                ProposalFrontmatter {
                    depends_on: vec!["a".to_string()],
                    blocks: vec![],
                    ..Default::default()
                },
            ),
        ];
        let issues_map = check_dag_cycles(&frontmatters);
        assert!(issues_map.is_empty());
    }

    // --- Feature-as-spec (BDD-on) tests ---

    fn spec_dir(tmp: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
        let dir = tmp.path().join(name);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn discover_features_finds_and_sorts() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = spec_dir(&tmp, "cli");
        fs::write(dir.join("zeta.feature"), "Feature: z\n").unwrap();
        fs::write(dir.join("alpha.feature"), "Feature: a\n").unwrap();
        fs::write(dir.join("spec.toon"), "kind: llman.sdd.spec\n").unwrap();

        let found = discover_features(&dir);
        let names: Vec<_> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["alpha.feature", "zeta.feature"]);
    }

    #[test]
    fn discover_features_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = spec_dir(&tmp, "cli");
        assert!(discover_features(&dir).is_empty());
    }

    #[test]
    fn validate_features_dir_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = spec_dir(&tmp, "cli");
        fs::write(
            dir.join("status.feature"),
            "Feature: Status\n  Scenario: ok\n    Given x\n    When y\n    Then z\n",
        )
        .unwrap();
        let issues = validate_features_dir(&dir, tmp.path(), "en");
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn validate_features_dir_syntax_error() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = spec_dir(&tmp, "cli");
        fs::write(dir.join("broken.feature"), "this is not gherkin at all\n").unwrap();
        let issues = validate_features_dir(&dir, tmp.path(), "en");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Error);
        assert!(issues[0].path.ends_with("broken.feature"));
    }

    #[test]
    fn validate_features_dir_chinese_keywords() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = spec_dir(&tmp, "cli");
        fs::write(
            dir.join("status.feature"),
            "功能: 状态\n  场景: 正常\n    假如 x\n    当 y\n    那么 z\n",
        )
        .unwrap();
        let issues = validate_features_dir(&dir, tmp.path(), "zh-CN");
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn validate_features_dir_empty_no_issues() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = spec_dir(&tmp, "cli");
        // No .feature files → no issues from this function (guardrail handled
        // by validate_main_spec_doc).
        let issues = validate_features_dir(&dir, tmp.path(), "en");
        assert!(issues.is_empty());
    }

    #[test]
    fn validate_features_dir_uses_relative_path() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = spec_dir(&tmp, "cli");
        fs::write(dir.join("broken.feature"), "not gherkin\n").unwrap();
        let issues = validate_features_dir(&dir, tmp.path(), "en");
        // Path should be relative to project root (tmp), not absolute.
        assert!(issues[0].path.contains("cli/broken.feature"));
        assert!(!issues[0].path.starts_with('/'));
    }

    #[test]
    fn locale_to_gherkin_lang_zh_hans_maps_to_zh_cn() {
        assert_eq!(locale_to_gherkin_lang(Some("zh-Hans"), None), "zh-CN");
        assert_eq!(locale_to_gherkin_lang(Some("zh-Hans-CN"), None), "zh-CN");
    }

    #[test]
    fn locale_to_gherkin_lang_passthrough() {
        assert_eq!(locale_to_gherkin_lang(Some("en"), None), "en");
        assert_eq!(locale_to_gherkin_lang(None, None), "en");
    }

    #[test]
    fn locale_to_gherkin_lang_bdd_default_language_wins() {
        let bdd = BddConfig {
            framework: "cucumber-rs".to_string(),
            feature_dir: None,
            default_language: Some("ja".to_string()),
            run_command: None,
            verify_prompt: None,
        };
        assert_eq!(locale_to_gherkin_lang(Some("zh-Hans"), Some(&bdd)), "ja");
    }

    fn empty_spec_doc() -> MainSpecDoc {
        MainSpecDoc {
            kind: "llman.sdd.spec".to_string(),
            name: "cli".to_string(),
            purpose: "x".to_string(),
            valid_scope: Vec::new(),
            requirements: Vec::new(),
            scenarios: Vec::new(),
        }
    }

    #[test]
    fn empty_requirements_is_error() {
        // Unified path: empty requirements is always an Error (spec.toon is SSOT).
        let doc = empty_spec_doc();
        let issues = validate_main_spec_doc(&doc, "cli");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Error);
        assert_eq!(issues[0].path, "cli/requirements");
    }

    #[test]
    fn requirement_without_scenario_is_error() {
        // A requirement MUST have at least one scenario (coverage check, unified).
        let mut doc = empty_spec_doc();
        doc.requirements = vec![crate::sdd::spec::ir::RequirementEntry {
            req_id: "r1".to_string(),
            title: "T".to_string(),
            statement: "System MUST do x".to_string(),
        }];
        let issues = validate_main_spec_doc(&doc, "cli");
        assert!(
            issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error && i.path == "cli/requirements[0]"),
            "{issues:?}"
        );
    }

    #[test]
    fn requirement_with_scenario_is_ok() {
        let mut doc = empty_spec_doc();
        doc.requirements = vec![crate::sdd::spec::ir::RequirementEntry {
            req_id: "r1".to_string(),
            title: "T".to_string(),
            statement: "System MUST do x".to_string(),
        }];
        doc.scenarios = vec![crate::sdd::spec::ir::ScenarioEntry {
            req_id: "r1".to_string(),
            id: "happy".to_string(),
            given: String::new(),
            when_: "trigger".to_string(),
            then_: "result".to_string(),
            feature: true,
        }];
        let issues = validate_main_spec_doc(&doc, "cli");
        assert!(
            issues.iter().all(|i| i.level != ValidationLevel::Error),
            "{issues:?}"
        );
    }

    // --- Full-mode (r52) exit-code mapping tests ---

    fn bdd_with_run_command(cmd: &str) -> BddConfig {
        BddConfig {
            framework: "custom".to_string(),
            feature_dir: None,
            default_language: None,
            run_command: Some(cmd.to_string()),
            verify_prompt: None,
        }
    }

    #[test]
    fn full_mode_exit_zero_is_pass() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = spec_dir(&tmp, "cli");
        fs::write(dir.join("ok.feature"), "Feature: OK\n").unwrap();
        let bdd = bdd_with_run_command("true");
        let issues = run_full_mode(&dir, &bdd);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Info);
    }

    #[test]
    fn full_mode_exit_nonzero_is_fail() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = spec_dir(&tmp, "cli");
        fs::write(dir.join("bad.feature"), "Feature: Bad\n").unwrap();
        let bdd = bdd_with_run_command("echo boom >&2; false");
        let issues = run_full_mode(&dir, &bdd);
        // 1 summary issue + 1 line of runner output ("boom").
        assert!(issues.len() >= 2);
        assert_eq!(issues[0].level, ValidationLevel::Error);
        assert!(issues[0].message.contains("Runner output"));
        // The runner's stderr line is surfaced verbatim.
        assert!(issues.iter().any(|i| i.message.contains("boom")));
    }

    #[test]
    fn full_mode_cache_runs_identical_command_once() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // Mimic <root>/llmanspec/specs/<cap> so project_root_from_spec_dir works.
        let a = root.join("llmanspec/specs/a");
        let b = root.join("llmanspec/specs/b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("a.feature"), "Feature: A\n").unwrap();
        fs::write(b.join("b.feature"), "Feature: B\n").unwrap();

        let counter = root.join("counter");
        let cmd = format!("printf x >> {}", counter.display());
        let bdd = bdd_with_run_command(&cmd);

        let mut cache = FullModeCache::new();
        let first = run_full_mode_cached(&a, &bdd, Some(&mut cache));
        let second = run_full_mode_cached(&b, &bdd, Some(&mut cache));

        assert!(
            first.iter().all(|i| i.level != ValidationLevel::Error),
            "{first:?}"
        );
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].level, ValidationLevel::Info);
        assert!(
            second[0].message.contains("reused"),
            "{}",
            second[0].message
        );
        let count = fs::read_to_string(&counter).unwrap();
        assert_eq!(
            count, "x",
            "project-wide command must run once, got {count:?}"
        );
    }

    #[test]
    fn full_mode_cache_runs_distinct_expanded_commands_separately() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("llmanspec/specs/alpha");
        let b = root.join("llmanspec/specs/beta");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("a.feature"), "Feature: A\n").unwrap();
        fs::write(b.join("b.feature"), "Feature: B\n").unwrap();

        let counter = root.join("counter");
        // `{feature_name}` expands differently per capability → cache miss → two runs.
        let bdd = bdd_with_run_command(&format!(
            "printf x >> {}; echo {{feature_name}} >/dev/null",
            counter.display()
        ));

        let mut cache = FullModeCache::new();
        let _ = run_full_mode_cached(&a, &bdd, Some(&mut cache));
        let _ = run_full_mode_cached(&b, &bdd, Some(&mut cache));
        let count = fs::read_to_string(&counter).unwrap();
        assert_eq!(
            count, "xx",
            "distinct expansions must each run, got {count:?}"
        );
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
        let spec_file = entry.path().join(SPEC_FILE);
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
        let context = format!("delta spec `{}`", spec_name);
        let parse_result = if strict {
            BACKEND.parse_delta_spec_strict(&content, &context)
        } else {
            BACKEND.parse_delta_spec(&content, &context)
        };
        let doc = match parse_result {
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

/// Discover `.feature` files in a spec directory (feature-as-spec mode, r51).
/// Returns paths sorted for deterministic output. No registration table needed:
/// dropping a file into the directory IS the registration.
pub fn discover_features(spec_dir: &Path) -> Vec<std::path::PathBuf> {
    let pattern = spec_dir.join("*.feature");
    let mut paths: Vec<_> = glob::glob(pattern.to_string_lossy().as_ref())
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .collect();
    paths.sort();
    paths
}

/// Map a config locale to a Gherkin parsing language (r53).
/// `zh-Hans*` → `zh-CN`; everything else passes through. An explicit
/// `bdd.default_language` always wins over locale derivation.
pub fn locale_to_gherkin_lang(locale: Option<&str>, bdd_config: Option<&BddConfig>) -> String {
    if let Some(bdd) = bdd_config
        && let Some(lang) = &bdd.default_language
        && !lang.trim().is_empty()
    {
        return lang.clone();
    }
    match locale.map(str::trim).filter(|l| !l.is_empty()) {
        Some(l) if l.starts_with("zh-Hans") => "zh-CN".to_string(),
        Some(l) => l.to_string(),
        None => "en".to_string(),
    }
}

/// Fast-mode feature-as-spec validation (r52): parse every `.feature` in the
/// spec directory as Gherkin. Structural legality only — no test runner.
fn validate_features_dir(spec_dir: &Path, project_root: &Path, lang: &str) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let features = discover_features(spec_dir);
    if features.is_empty() {
        return issues;
    }

    let env_result = gherkin::GherkinEnv::new(lang);
    if env_result.is_err() {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: format!("{}", spec_dir.display()),
            message: t!(
                "sdd.validate.feature_unsupported_language",
                lang = lang,
                dir = spec_dir.display()
            )
            .to_string(),
        });
        return issues;
    }

    for feature_path in &features {
        let rel = feature_path
            .strip_prefix(project_root)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| feature_path.clone());
        let rel_str = rel.display().to_string();
        match fs::read_to_string(feature_path) {
            Ok(content) => {
                // GherkinEnv is not Clone; rebuild it per feature.
                let env = match gherkin::GherkinEnv::new(lang) {
                    Ok(env) => env,
                    Err(_) => {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Error,
                            path: rel_str,
                            message: t!(
                                "sdd.validate.feature_unsupported_language",
                                lang = lang,
                                dir = spec_dir.display()
                            )
                            .to_string(),
                        });
                        continue;
                    }
                };
                if let Err(e) = gherkin::Feature::parse(&content, env) {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        path: rel_str.clone(),
                        message: t!(
                            "sdd.validate.feature_parse_error",
                            path = rel_str,
                            error = e
                        )
                        .to_string(),
                    });
                }
            }
            Err(e) => {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    path: rel_str.clone(),
                    message: t!("sdd.validate.feature_read_error", path = rel_str, error = e)
                        .to_string(),
                });
            }
        }
    }

    issues
}

/// Full-mode execution (r52 / r91): shell out the BDD run command once for the
/// entire spec directory. Exit code 0 → pass; non-zero → fail.
///
/// When `cache` is provided (bulk validate), results are keyed by the expanded
/// command string so identical project-wide runners execute at most once.
fn run_full_mode_cached(
    spec_dir: &Path,
    bdd_config: &BddConfig,
    cache: Option<&mut FullModeCache>,
) -> Vec<ValidationIssue> {
    let command = bdd_config.effective_run_command();
    let expanded = expand_run_command_placeholders(&command, spec_dir);

    if let Some(cache) = cache {
        if let Some(entry) = cache.get(&expanded) {
            let level = if entry.success {
                ValidationLevel::Info
            } else {
                ValidationLevel::Error
            };
            return vec![ValidationIssue {
                level,
                path: spec_dir.display().to_string(),
                message: t!("sdd.validate.full_mode_reused", command = expanded.as_str())
                    .to_string(),
            }];
        }
        let issues = run_full_mode(spec_dir, bdd_config);
        let success = !issues.iter().any(|i| i.level == ValidationLevel::Error);
        cache.insert(
            expanded,
            FullModeCacheEntry {
                success,
                issues: issues.clone(),
            },
        );
        return issues;
    }

    run_full_mode(spec_dir, bdd_config)
}

/// Full-mode execution (r52): shell out the BDD run command once for the entire
/// spec directory. Exit code 0 → pass; non-zero → fail.
///
/// For `cargo test` / rstest-bdd style runners, inject a per-HEAD
/// `CARGO_TARGET_DIR` so compile-time feature discovery cannot reuse a stale
/// expansion from a previous HEAD.
fn run_full_mode(spec_dir: &Path, bdd_config: &BddConfig) -> Vec<ValidationIssue> {
    let command = bdd_config.effective_run_command();
    let expanded = expand_run_command_placeholders(&command, spec_dir);
    let mut shell = std::process::Command::new("sh");
    shell
        .args(["-c", &expanded])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Prefer running from the project root (parent of llmanspec/) when possible.
    if let Some(root) = project_root_from_spec_dir(spec_dir) {
        shell.current_dir(root);
        if looks_like_cargo_test(&expanded)
            && let Ok(sha) = short_head_sha(root)
        {
            let target = root.join(format!("target/bdd-{sha}"));
            shell.env("CARGO_TARGET_DIR", &target);
        }
    }

    let shell = match shell.spawn() {
        Ok(child) => child,
        Err(e) => {
            return vec![ValidationIssue {
                level: ValidationLevel::Error,
                path: spec_dir.display().to_string(),
                message: t!(
                    "sdd.validate.full_mode_spawn_failed",
                    command = expanded,
                    error = e
                )
                .to_string(),
            }];
        }
    };
    let output = match shell.wait_with_output() {
        Ok(output) => output,
        Err(e) => {
            return vec![ValidationIssue {
                level: ValidationLevel::Error,
                path: spec_dir.display().to_string(),
                message: t!(
                    "sdd.validate.full_mode_spawn_failed",
                    command = expanded,
                    error = e
                )
                .to_string(),
            }];
        }
    };

    if output.status.success() {
        let n = discover_features(spec_dir).len();
        return vec![ValidationIssue {
            level: ValidationLevel::Info,
            path: spec_dir.display().to_string(),
            message: t!("sdd.validate.full_mode_passed", count = n).to_string(),
        }];
    }

    // Failure: surface the runner output line-by-line so the user can see
    // which feature/scenario failed (cucumber/pytest print this to stdout/stderr).
    let mut issues = vec![ValidationIssue {
        level: ValidationLevel::Error,
        path: spec_dir.display().to_string(),
        message: t!("sdd.validate.full_mode_failed", command = expanded).to_string(),
    }];
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stderr.lines().chain(stdout.lines()) {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: spec_dir.display().to_string(),
            message: trimmed.to_string(),
        });
    }
    issues
}

fn looks_like_cargo_test(command: &str) -> bool {
    let c = command.to_ascii_lowercase();
    c.contains("cargo test") || c.contains("cargo nextest")
}

fn project_root_from_spec_dir(spec_dir: &Path) -> Option<&Path> {
    // spec_dir is typically <root>/llmanspec/specs/<cap>
    spec_dir
        .parent()
        .and_then(|p| p.parent()) // llmanspec
        .and_then(|p| p.parent()) // root
}

fn short_head_sha(root: &Path) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("git rev-parse failed".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn expand_run_command_placeholders(command: &str, spec_dir: &Path) -> String {
    command
        .replace("{feature_dir}", &spec_dir.display().to_string())
        .replace("{feature_path}", &spec_dir.display().to_string())
        .replace(
            "{feature_name}",
            spec_dir.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        )
}

fn validate_main_spec_doc(doc: &MainSpecDoc, spec_name: &str) -> Vec<ValidationIssue> {
    validate_main_spec_doc_partitioned(doc, spec_name, &[])
}

/// Like [`validate_main_spec_doc`], but harness `@req` links count toward
/// "each requirement has ≥1 scenario" under Partitioned SSOT.
fn validate_main_spec_doc_partitioned(
    doc: &MainSpecDoc,
    spec_name: &str,
    harness: &[crate::sdd::spec::partitioned::FeatureScenario],
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if doc.requirements.is_empty() {
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: format!("{spec_name}/requirements"),
            message: t!("sdd.validate.empty_requirements").to_string(),
        });
    }

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

    for sc in harness {
        for rid in &sc.req_ids {
            let rid = rid.trim();
            if req_id_seen.contains(rid) {
                *scenarios_by_req.entry(rid).or_insert(0) += 1;
            }
        }
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
    archived_change_ids: &[String],
    has_frozen: bool,
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
    let active_ids: std::collections::HashSet<&str> =
        all_change_ids.iter().map(|s| s.as_str()).collect();
    let archived_ids: std::collections::HashSet<&str> =
        archived_change_ids.iter().map(|s| s.as_str()).collect();

    let depends_on = parse_yaml_string_list(&parsed, "depends_on", &mut issues);
    let blocks = parse_yaml_string_list(&parsed, "blocks", &mut issues);
    let branch = parse_yaml_optional_string(&parsed, "branch");
    let base_sha = parse_yaml_optional_string(&parsed, "base_sha")
        .or_else(|| parse_yaml_optional_string(&parsed, "baseSha"));
    let checkpointed = parse_yaml_optional_bool(&parsed, "checkpointed");
    let checkpoint_sha = parse_yaml_optional_string(&parsed, "checkpoint_sha")
        .or_else(|| parse_yaml_optional_string(&parsed, "checkpointSha"));

    for id in &depends_on {
        if active_ids.contains(id.as_str()) {
            // valid active dependency
        } else if archived_ids.contains(id.as_str()) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: "proposal.md/frontmatter.depends_on".to_string(),
                message: t!("sdd.validate.proposal_depends_on_archived", id = id).to_string(),
            });
        } else if has_frozen {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: "proposal.md/frontmatter.depends_on".to_string(),
                message: t!("sdd.validate.proposal_depends_on_may_be_frozen", id = id).to_string(),
            });
        } else {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: "proposal.md/frontmatter.depends_on".to_string(),
                message: t!("sdd.validate.proposal_depends_on_unknown", id = id).to_string(),
            });
        }
    }

    for id in &blocks {
        if active_ids.contains(id.as_str()) {
            // valid active reference
        } else if archived_ids.contains(id.as_str()) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: "proposal.md/frontmatter.blocks".to_string(),
                message: t!("sdd.validate.proposal_blocks_archived", id = id).to_string(),
            });
        } else if has_frozen {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: "proposal.md/frontmatter.blocks".to_string(),
                message: t!("sdd.validate.proposal_blocks_may_be_frozen", id = id).to_string(),
            });
        } else {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: "proposal.md/frontmatter.blocks".to_string(),
                message: t!("sdd.validate.proposal_blocks_unknown", id = id).to_string(),
            });
        }
    }

    (
        issues,
        ProposalFrontmatter {
            depends_on,
            blocks,
            branch,
            base_sha,
            checkpointed,
            checkpoint_sha,
        },
    )
}

fn parse_yaml_optional_string(doc: &serde_yaml::Value, key: &str) -> Option<String> {
    doc.get(key).and_then(|v| match v {
        serde_yaml::Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        _ => None,
    })
}

fn parse_yaml_optional_bool(doc: &serde_yaml::Value, key: &str) -> bool {
    match doc.get(key) {
        Some(serde_yaml::Value::Bool(b)) => *b,
        Some(serde_yaml::Value::String(s)) => matches!(s.trim(), "true" | "yes" | "1"),
        _ => false,
    }
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

pub fn check_tasks_completion(
    _change_dir: &Path,
    archive_config: &ArchiveConfig,
) -> Vec<ValidationIssue> {
    let tasks_path = _change_dir.join("tasks.md");
    let report = match tasks::parse_tasks_file(&tasks_path) {
        Ok(Some(r)) => r,
        _ => return Vec::new(),
    };
    if report.total() == 0 {
        return Vec::new();
    }

    let mut issues = Vec::new();

    for item in &report.items {
        match &item.status {
            TaskStatus::Completed => {}
            TaskStatus::Pending => {
                let level = if archive_config.strict_defer() {
                    ValidationLevel::Error
                } else {
                    ValidationLevel::Warning
                };
                issues.push(ValidationIssue {
                    level,
                    path: "tasks.md".to_string(),
                    message: t!(
                        "sdd.validate.task_pending",
                        line = item.line_num,
                        task = item.text
                    )
                    .to_string(),
                });
            }
        }
    }

    issues
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeStage {
    Draft,
    Specified,
    Designed,
    Full,
}

impl ChangeStage {
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeStage::Draft => "draft",
            ChangeStage::Specified => "specified",
            ChangeStage::Designed => "designed",
            ChangeStage::Full => "full",
        }
    }
}

pub fn determine_stage(change_dir: &Path) -> ChangeStage {
    let has_proposal = change_dir.join("proposal.md").exists();
    let has_specs = has_spec_files(&change_dir.join("specs"));
    let has_design = change_dir.join("design.md").exists();
    let has_tasks = change_dir.join("tasks.md").exists();

    match (has_proposal, has_specs, has_design, has_tasks) {
        (true, true, true, true) => ChangeStage::Full,
        (true, true, true, false) => ChangeStage::Designed,
        (true, true, _, _) => ChangeStage::Specified,
        _ => ChangeStage::Draft,
    }
}

pub fn has_spec_files(specs_dir: &Path) -> bool {
    if !specs_dir.is_dir() {
        return false;
    }
    match fs::read_dir(specs_dir) {
        Ok(entries) => entries.flatten().any(|e| {
            if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                return false;
            }
            let dir = e.path();
            // BDD-on (feature-as-spec): .feature file IS the spec delta.
            if dir.join(SPEC_FILE).exists() {
                return true;
            }
            // Check for .feature files via glob (BDD-on, r51).
            !discover_features(&dir).is_empty()
        }),
        Err(_) => false,
    }
}

pub fn check_design_tasks_constraint(change_dir: &Path) -> Vec<ValidationIssue> {
    let has_tasks = change_dir.join("tasks.md").exists();
    let has_design = change_dir.join("design.md").exists();

    if has_tasks && !has_design {
        return vec![ValidationIssue {
            level: ValidationLevel::Error,
            path: "tasks.md".to_string(),
            message: t!("sdd.validate.tasks_without_design").to_string(),
        }];
    }
    Vec::new()
}

pub fn check_completeness_stage(
    change_dir: &Path,
    _strict: bool,
    force_stage: Option<ChangeStage>,
) -> Vec<ValidationIssue> {
    let stage = force_stage.unwrap_or_else(|| determine_stage(change_dir));
    let mut issues = Vec::new();

    // Stage hints are always Info — they describe the current state without
    // blocking validation. Stage-aware enforcement lives in validate_change_full.
    match stage {
        ChangeStage::Full => {}
        ChangeStage::Designed => {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: "completeness".to_string(),
                message: t!("sdd.validate.stage_designed_hint").to_string(),
            });
        }
        ChangeStage::Specified => {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: "completeness".to_string(),
                message: t!("sdd.validate.stage_specified_hint").to_string(),
            });
        }
        ChangeStage::Draft => {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: "completeness".to_string(),
                message: t!("sdd.validate.stage_draft_hint").to_string(),
            });
        }
    }

    issues
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
