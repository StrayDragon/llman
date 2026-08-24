use crate::sdd::project::config::{ArchiveConfig, BddConfig};
use crate::sdd::shared::constants::SPEC_FILE;
use crate::sdd::shared::tasks::{self, TaskStatus};
use crate::sdd::spec::backend::feature_backend::{
    self, FeatureBackend, ParsedFeatureSpec, ScenarioTier,
};
use crate::sdd::spec::frontmatter::split_frontmatter;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// One tip per process when full-mode BDD runner starts (bulk validate reuses cache).
static FULL_MODE_QUIET_HINT_SHOWN: AtomicBool = AtomicBool::new(false);

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
    /// When true, apply-ready does not require a live `llmanspec/specs/**` diff
    /// on the bound branch (docs/governance changes with no contract edit).
    pub skip_specs_landing: bool,
    /// Human acknowledgement (spec-format r135): allows the change to modify
    /// locked `@human` rule scenarios under `llmanspec/specs/**/*.feature`.
    pub rules_edit_acked: bool,
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
    let _ = locale;
    let _ = project_root; // retained for upcoming lock-hash git context (t5)
    let spec_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("spec")
        .to_string();

    let context = format!("spec `{}`", spec_name);
    let bdd_enabled = bdd_config.is_some();

    // Single-track feature-as-spec (r131): `content` is the capability's
    // `.feature` text. The rich parse covers Gherkin legality, header metadata
    // and the tag grammar in one pass.
    let parse_result = FeatureBackend.parse_content(content, &context);
    match parse_result {
        Ok(parsed) => {
            let mut issues = Vec::new();

            validate_spec_meta(&parsed.valid_scope, &spec_name, &mut issues);
            let frontmatter = if has_meta_errors(&issues) {
                None
            } else {
                Some(SpecFrontmatter {
                    valid_scope: parsed.valid_scope.clone(),
                })
            };

            if parsed.name.trim() != spec_name {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    path: format!("{}/meta.name", spec_name),
                    message: format!(
                        "Spec `# capability:` header must match spec directory name: `{}` != `{}`",
                        parsed.name.trim(),
                        spec_name
                    ),
                });
            }

            // Single-track grammar gates run regardless of the runner switch:
            // with no `bdd:` section the feature is still the spec (r83).
            issues.extend(validate_single_track(&parsed, &spec_name));

            if bdd_enabled
                && check_mode
                && let Some(spec_dir) = path.parent()
                && let Some(bdd) = bdd_config
            {
                issues.extend(run_full_mode_cached(spec_dir, bdd, full_mode_cache));
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

/// Resolve a capability's single-track spec file (r131).
///
/// - exactly one `*.feature` → that file;
/// - a legacy `spec.toon` present → error pointing at `toon2features`;
/// - zero or multiple `.feature` files → error.
pub fn resolve_spec_file(specs_root: &Path, id: &str) -> Result<std::path::PathBuf, anyhow::Error> {
    let dir = specs_root.join(id);
    if !dir.is_dir() {
        return Err(anyhow::anyhow!(
            "spec directory not found: {}",
            dir.display()
        ));
    }
    if dir.join(SPEC_FILE).exists() {
        return Err(anyhow::anyhow!(
            "legacy `spec.toon` found for `{id}`; the single-track format no longer reads it — \
             run `llman sdd project migrate --kind toon2features`"
        ));
    }
    let features = discover_features(&dir);
    let mut features = features;
    match features.len() {
        1 => Ok(features.remove(0)),
        0 => Err(anyhow::anyhow!(
            "no `.feature` spec found under {} (r131: one .feature per capability)",
            dir.display()
        )),
        n => Err(anyhow::anyhow!(
            "{n} `.feature` files found under {} but r131 mandates exactly one; merge them",
            dir.display()
        )),
    }
}

/// Validate the rich parse of one capability against the single-track grammar
/// (spec-format r132 / r135 semantics).
fn validate_single_track(parsed: &ParsedFeatureSpec, spec_name: &str) -> Vec<ValidationIssue> {
    use ValidationLevel::{Error, Warning};
    let mut issues = Vec::new();
    if parsed.feature_title.trim().is_empty() {
        issues.push(ValidationIssue {
            level: Error,
            path: format!("{spec_name}/feature"),
            message: "Feature line must carry a title".to_string(),
        });
    }
    if parsed.rule_scenarios().count() == 0 {
        issues.push(ValidationIssue {
            level: Error,
            path: format!("{spec_name}/rules"),
            message: "spec must define at least one @human constraint scenario".to_string(),
        });
    }

    let mut rule_req_ids: Vec<String> = Vec::new();
    let mut rule_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut rule_hashes: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for sc in &parsed.scenarios {
        match sc.tier {
            Some(ScenarioTier::Constraint) | Some(ScenarioTier::Manual) => {
                let path = format!("{}/rule/{}", spec_name, sc.name);
                match sc.req_ids.len() {
                    0 => issues.push(ValidationIssue {
                        level: Error,
                        path: path.clone(),
                        message: "@human constraint scenario must carry an @req:<req_id> tag"
                            .to_string(),
                    }),
                    1 => {}
                    _ => issues.push(ValidationIssue {
                        level: Error,
                        path: path.clone(),
                        message: format!(
                            "constraint scenario carries multiple @req tags {:?}; split the scenario per requirement",
                            sc.req_ids
                        ),
                    }),
                }
                if !rule_names.insert(sc.name.clone()) {
                    issues.push(ValidationIssue {
                        level: Error,
                        path: path.clone(),
                        message: format!("duplicate constraint scenario name `{}`", sc.name),
                    });
                }
                let hash = feature_backend::lock_hash(sc);
                if let Some(prev) = rule_hashes.get(&hash) {
                    issues.push(ValidationIssue {
                        level: Error,
                        path: path.clone(),
                        message: format!(
                            "constraint scenarios `{prev}` and `{}` are identical after normalization",
                            sc.name
                        ),
                    });
                } else {
                    rule_hashes.insert(hash, sc.name.clone());
                }
                let statement = feature_backend::rule_statement(sc);
                if !contains_normative_keyword(&statement) {
                    issues.push(ValidationIssue {
                        level: Error,
                        path: path.clone(),
                        message: format!(
                            "constraint statement must contain MUST/SHALL (or 必须/不得/禁止): {}",
                            statement.trim()
                        ),
                    });
                }
                if let Some(rid) = sc.req_ids.first() {
                    rule_req_ids.push(rid.clone());
                }
            }
            Some(ScenarioTier::Acceptance) => {
                let path = format!("{}/acceptance/{}", spec_name, sc.name);
                if sc.req_ids.is_empty() {
                    issues.push(ValidationIssue {
                        level: Warning,
                        path: path.clone(),
                        message: format!(
                            "orphan acceptance scenario `{}` has no @req:<req_id> link",
                            sc.name
                        ),
                    });
                }
                if sc.when_.is_empty() || sc.then_.is_empty() {
                    issues.push(ValidationIssue {
                        level: Warning,
                        path: path.clone(),
                        message: format!(
                            "acceptance scenario `{}` is missing When/Then steps (runner will not bind)",
                            sc.name
                        ),
                    });
                }
            }
            None => {
                issues.push(ValidationIssue {
                    level: Warning,
                    path: format!("{}/scenario/{}", spec_name, sc.name),
                    message: format!(
                        "scenario `{}` carries neither @human nor @executable; tag it or drop it",
                        sc.name
                    ),
                });
            }
        }
    }

    // Every acceptance @req must point at a defined rule (dangling link gate).
    let defined: std::collections::HashSet<&String> = rule_req_ids.iter().collect();
    for sc in parsed.acceptance_scenarios() {
        for rid in &sc.req_ids {
            if !defined.contains(rid) {
                issues.push(ValidationIssue {
                    level: Error,
                    path: format!("{}/acceptance/{}/@req", spec_name, sc.name),
                    message: format!(
                        "@req:{rid} on acceptance scenario `{}` has no matching @human constraint",
                        sc.name
                    ),
                });
            }
        }
    }

    // Rule coverage tiers (r134): pending rules surface as INFO so gaps stay
    // visible without blocking spec-first workflows under --strict.
    for req_id in &rule_req_ids {
        let enforced = parsed
            .acceptance_scenarios()
            .any(|sc| sc.req_ids.iter().any(|r| r == req_id));
        if !enforced {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: format!("{spec_name}/coverage"),
                message: format!("rule {req_id} is pending: no @executable acceptance scenario and no @manual waiver"),
            });
        }
    }

    issues
}

/// Normative-keyword check tolerant of CJK statements.
fn contains_normative_keyword(text: &str) -> bool {
    ["MUST", "SHALL", "必须", "不得", "禁止"]
        .iter()
        .any(|kw| text.contains(kw))
}

/// Validate the in-document scope (header `# scope:` line). Must be present
/// and non-empty for a main spec; drives the staleness check.
fn validate_spec_meta(valid_scope: &[String], spec_name: &str, issues: &mut Vec<ValidationIssue>) {
    validate_meta_list(valid_scope, spec_name, "valid_scope", issues);
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
        // A spec with no `# scope:` header is invalid (r133).
        let mut issues = Vec::new();
        validate_spec_meta(&[], "sample", &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(issues.iter().all(|i| i.level == ValidationLevel::Error));
    }

    #[test]
    fn meta_present_no_error() {
        let mut issues = Vec::new();
        validate_spec_meta(
            &["src/".to_string(), "tests/".to_string()],
            "sample",
            &mut issues,
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn single_track_grammar_gates() {
        let content = "\
# language: zh-CN
# capability: sample
# purpose: p
# scope: src

功能: sample

  @req:r1 @human
  场景: rule-a
    系统 MUST do x。

  @req:r1 @executable
  场景: acc-a
    假如 given
    当 when step
    那么 then step
";
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("sample");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("spec.feature");
        fs::write(&path, content).unwrap();

        let validation = validate_spec_content_with_frontmatter_and_bdd(
            &path, content, false, None, None, None, false, None,
        );
        assert!(
            validation.report.valid,
            "issues: {:?}",
            validation.report.issues
        );
        assert_eq!(validation.frontmatter.unwrap().valid_scope, vec!["src"]);

        // Dangling acceptance link is an ERROR.
        let dangling = content.replace("@req:r1 @executable", "@req:r404 @executable");
        let v2 = validate_spec_content_with_frontmatter_and_bdd(
            &path, &dangling, false, None, None, None, false, None,
        );
        assert!(
            v2.report
                .issues
                .iter()
                .any(|i| i.message.contains("no matching @human"))
        );

        // Orphan acceptance (no @req) is a WARNING.
        let orphan = content.replace("@req:r1 @executable", "@executable");
        let v3 = validate_spec_content_with_frontmatter_and_bdd(
            &path, &orphan, false, None, None, None, false, None,
        );
        assert!(v3.report.issues.iter().any(
            |i| i.level == ValidationLevel::Warning && i.message.contains("orphan acceptance")
        ));
    }

    #[test]
    fn resolve_spec_file_rejects_legacy_toon_and_multi_features() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_root = tmp.path().join("specs");
        let legacy = specs_root.join("legacy");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join(SPEC_FILE), "kind: llman.sdd.spec\n").unwrap();
        let err = resolve_spec_file(&specs_root, "legacy")
            .unwrap_err()
            .to_string();
        assert!(err.contains("toon2features"), "got: {err}");

        let multi = specs_root.join("multi");
        fs::create_dir_all(&multi).unwrap();
        fs::write(multi.join("a.feature"), "# capability: multi\n").unwrap();
        fs::write(multi.join("b.feature"), "# capability: multi\n").unwrap();
        let err = resolve_spec_file(&specs_root, "multi")
            .unwrap_err()
            .to_string();
        assert!(err.contains("exactly one"), "got: {err}");
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
    fn proposal_frontmatter_unknown_field_status_reports_error() {
        // r124: `status` is a spurious field that crept in via example imitation;
        // the lifecycle stage is inferred (r93), not stored in frontmatter.
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[(
                "proposal.md",
                "---\nstatus: purpose-draft\ndepends_on: []\n---\n## Why\nTest",
            )],
        );
        let (issues, _) = check_proposal_frontmatter(&change_dir, &["x".to_string()], &[], false);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, ValidationLevel::Error);
        assert!(issues[0].message.contains("status"));
        assert!(issues[0].message.contains("depends_on"));
    }

    #[test]
    fn proposal_frontmatter_unknown_field_title_reports_error() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[(
                "proposal.md",
                "---\ntitle: Some Title\npriority: 5\nauthor: agent\n---\n## Why\nTest",
            )],
        );
        let (issues, _) = check_proposal_frontmatter(&change_dir, &["x".to_string()], &[], false);
        // three unknown fields: title, priority, author
        assert_eq!(issues.len(), 3);
        assert!(issues.iter().all(|i| i.level == ValidationLevel::Error));
        assert!(issues.iter().any(|i| i.message.contains("title")));
        assert!(issues.iter().any(|i| i.message.contains("priority")));
        assert!(issues.iter().any(|i| i.message.contains("author")));
    }

    #[test]
    fn proposal_frontmatter_allowed_fields_no_unknown_error() {
        // All recognized keys (incl. camelCase attach/checkpoint aliases) are accepted.
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[(
                "proposal.md",
                "---\ndepends_on: []\nblocks: []\nbranch: sdd/x\nbaseSha: abc123\ncheckpointed: true\ncheckpointSha: def456\nskip_specs_landing: true\n---\n## Why\nTest",
            )],
        );
        let (issues, _) = check_proposal_frontmatter(&change_dir, &["x".to_string()], &[], false);
        assert!(
            issues.is_empty(),
            "expected no issues for allowed fields, got: {issues:?}"
        );
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
            bindings: None,
            run_command: None,
            verify_prompt: None,
        };
        assert_eq!(locale_to_gherkin_lang(Some("zh-Hans"), Some(&bdd)), "ja");
    }

    /// Test helper: parse single-track content and run the grammar gates.
    fn validate_single_track_content(content: &str, name: &str) -> Vec<ValidationIssue> {
        match FeatureBackend.parse_content(content, &format!("spec `{name}`")) {
            Ok(parsed) => validate_single_track(&parsed, name),
            Err(err) => vec![ValidationIssue {
                level: ValidationLevel::Error,
                path: "file".to_string(),
                message: err.to_string(),
            }],
        }
    }

    #[test]
    fn empty_rules_is_error() {
        // Single-track: a capability with no @human rule scenario is invalid.
        let content = "\
# capability: cli
# purpose: x
# scope: src

Feature: cli
  Scenario: plain
    Given a
";
        let issues = validate_single_track_content(content, "cli");
        assert!(
            issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error
                    && i.message.contains("at least one @human")),
            "{issues:?}"
        );
    }

    #[test]
    fn rule_without_normative_keyword_is_error() {
        let content = "\
# capability: cli
# purpose: x
# scope: src

Feature: cli
  @req:r1 @human
  Scenario: weak rule
    System does something nice sometimes.
";
        let issues = validate_single_track_content(content, "cli");
        assert!(
            issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error && i.message.contains("MUST/SHALL"))
        );
    }

    #[test]
    fn dangling_acceptance_link_is_error() {
        let content = "\
# capability: cli
# purpose: x
# scope: src

Feature: cli
  @req:r1 @human
  Scenario: real rule
    MUST behave.

  @req:r404 @executable
  Scenario: ghost link
    Given a
    When b
    Then c
";
        let issues = validate_single_track_content(content, "cli");
        assert!(
            issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error
                    && i.message.contains("no matching @human"))
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
            bindings: None,
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

    // --- determine_stage: unified three-state flow (r93) ---

    const PROPOSAL_NO_FM: &str = "## Why\nTest";
    const PROPOSAL_ATTACHED: &str = "---\nbranch: feat/x\nbase_sha: abc123\n---\n## Why\nTest";

    #[test]
    fn stage_draft_when_only_proposal() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[("proposal.md", PROPOSAL_NO_FM)]);
        assert_eq!(determine_stage(&change_dir), ChangeStage::Draft);
    }

    #[test]
    fn stage_designed_when_proposal_design_tasks_without_attach() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", PROPOSAL_NO_FM),
                ("design.md", "# design"),
                ("tasks.md", "- [ ] t1"),
            ],
        );
        assert_eq!(determine_stage(&change_dir), ChangeStage::Designed);
    }

    #[test]
    fn stage_full_when_proposal_design_tasks_and_attach() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", PROPOSAL_ATTACHED),
                ("design.md", "# design"),
                ("tasks.md", "- [ ] t1"),
            ],
        );
        assert_eq!(determine_stage(&change_dir), ChangeStage::Full);
    }

    #[test]
    fn stage_draft_when_attached_but_missing_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        // attached + proposal + design, but no tasks → Draft: Full requires all
        // three artifacts; Designed requires tasks too. Missing tasks is an
        // incomplete state, reported as Draft (not Full, not Designed).
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", PROPOSAL_ATTACHED),
                ("design.md", "# design"),
            ],
        );
        assert_eq!(determine_stage(&change_dir), ChangeStage::Draft);
    }

    #[test]
    fn stage_draft_when_partial_attach_only_branch() {
        let tmp = tempfile::tempdir().unwrap();
        // Only `branch`, missing `base_sha` → not a complete attach binding.
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", "---\nbranch: feat/x\n---\n## Why\nx"),
                ("design.md", "# design"),
                ("tasks.md", "- [ ] t1"),
            ],
        );
        // proposal+design+tasks but no valid attach → Designed (not Draft):
        // artifacts are ready, just not bound to a branch yet.
        assert_eq!(determine_stage(&change_dir), ChangeStage::Designed);
    }

    #[test]
    fn stage_ignores_change_specs_dir_unified_flow() {
        let tmp = tempfile::tempdir().unwrap();
        // Unified flow: change/specs/ is abolished and MUST NOT affect stage.
        // Only the attach binding matters for Full.
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", PROPOSAL_NO_FM),
                ("design.md", "# design"),
                ("tasks.md", "- [ ] t1"),
                ("specs/cap/spec.toon", "kind: llman.sdd.spec\n"),
            ],
        );
        // specs/ present but no attach → Designed (specs/ is ignored).
        assert_eq!(determine_stage(&change_dir), ChangeStage::Designed);
    }

    #[test]
    fn has_attach_binding_detects_complete_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(&tmp, &[("proposal.md", PROPOSAL_ATTACHED)]);
        assert!(has_attach_binding(&change_dir));
    }

    #[test]
    fn has_attach_binding_rejects_incomplete_or_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let only_branch = setup_change_dir(
            &tmp,
            &[("proposal.md", "---\nbranch: feat/x\n---\n## Why\nx")],
        );
        assert!(!has_attach_binding(&only_branch));

        let no_fm = setup_change_dir(&tmp, &[("proposal.md", "## Why\nx")]);
        assert!(!has_attach_binding(&no_fm));

        let missing = setup_change_dir(&tmp, &[]);
        assert!(!has_attach_binding(&missing));
    }

    #[test]
    fn completeness_designed_hint_mentions_change_start() {
        let tmp = tempfile::tempdir().unwrap();
        let change_dir = setup_change_dir(
            &tmp,
            &[
                ("proposal.md", PROPOSAL_NO_FM),
                ("design.md", "# design"),
                ("tasks.md", "- [ ] t1"),
            ],
        );
        let issues = check_completeness_stage(&change_dir, false, None, true);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("change start"));
    }
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
    if !FULL_MODE_QUIET_HINT_SHOWN.swap(true, Ordering::Relaxed) {
        eprintln!(
            "{}",
            t!(
                "sdd.validate.full_mode_quiet_hint",
                command = expanded.as_str()
            )
        );
    }
    let mut shell = std::process::Command::new("sh");
    shell
        .args(["-c", &expanded])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Prefer running from the project root (parent of llmanspec/) when possible.
    // Guard the empty relative ancestor (`""` for paths like
    // `llmanspec/specs/<cap>`): current_dir("") is ENOENT.
    if let Some(root) = project_root_from_spec_dir(spec_dir).filter(|p| !p.as_os_str().is_empty()) {
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

/// Allowed top-level keys in a change `proposal.md` frontmatter (r124). Any
/// other key (e.g. `status`, `title`, `priority`, `author`) is rejected as an
/// ERROR by [`check_proposal_frontmatter`] to keep frontmatter the single
/// source of truth for change metadata. `baseSha` / `checkpointSha` are
/// accepted camelCase aliases of the snake_case attach/checkpoint bindings.
const PROPOSAL_FRONTMATTER_ALLOWED_FIELDS: &[&str] = &[
    "depends_on",
    "blocks",
    "branch",
    "base_sha",
    "baseSha",
    "checkpointed",
    "checkpoint_sha",
    "checkpointSha",
    "skip_specs_landing",
    "rules_edit_acked",
];

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
    let skip_specs_landing = parse_yaml_optional_bool(&parsed, "skip_specs_landing");
    let rules_edit_acked = parse_yaml_optional_bool(&parsed, "rules_edit_acked");

    // r124: reject unknown frontmatter fields (e.g. `status`, `title`,
    // `priority`, `author`). The allowed set is exactly the keys this parser
    // already recognizes; anything else is a spurious field that crept in via
    // example imitation and would undermine frontmatter as the metadata SSOT.
    if let Some(mapping) = parsed.as_mapping() {
        for key in mapping.keys() {
            if let serde_yaml::Value::String(name) = key
                && !PROPOSAL_FRONTMATTER_ALLOWED_FIELDS.contains(&name.as_str())
            {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    path: format!("proposal.md/frontmatter.{name}"),
                    message: t!(
                        "sdd.validate.proposal_frontmatter_unknown_field",
                        field = name,
                        allowed = PROPOSAL_FRONTMATTER_ALLOWED_FIELDS.join(", ")
                    )
                    .to_string(),
                });
            }
        }
    }

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
            skip_specs_landing,
            rules_edit_acked,
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
    Designed,
    Full,
}

impl ChangeStage {
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeStage::Draft => "draft",
            ChangeStage::Designed => "designed",
            ChangeStage::Full => "full",
        }
    }
}

/// Infer the change stage from on-disk artifacts under the unified Git-native
/// flow (r93). Three states only — `Specified` is removed:
/// - **Draft**: only `proposal.md` (or no attach binding).
/// - **Designed**: `proposal.md` + `design.md` + `tasks.md` present, but not
///   yet attached to a feature branch (no `branch`/`base_sha` in frontmatter).
/// - **Full**: `proposal.md` + `design.md` + `tasks.md` present **and** an
///   attach binding exists (via `change start` / `change attach`).
///
/// The spec signal is always the Git-native attach binding; `changes/<id>/specs/`
/// is no longer consulted (the directory is abolished, see r115).
pub fn determine_stage(change_dir: &Path) -> ChangeStage {
    let has_proposal = change_dir.join("proposal.md").exists();
    let has_design = change_dir.join("design.md").exists();
    let has_tasks = change_dir.join("tasks.md").exists();
    let attached = has_attach_binding(change_dir);

    match (has_proposal, attached, has_design, has_tasks) {
        (true, true, true, true) => ChangeStage::Full,
        (true, _, true, true) => ChangeStage::Designed,
        _ => ChangeStage::Draft,
    }
}

/// Read proposal.md frontmatter and report whether a Git-native BDD-on attach
/// binding is present (non-empty `branch` **and** `base_sha`). Best-effort: any
/// parse failure or missing file returns `false`, matching the historical
/// "no specs signal" semantics.
pub fn has_attach_binding(change_dir: &Path) -> bool {
    let Ok(content) = fs::read_to_string(change_dir.join("proposal.md")) else {
        return false;
    };
    let (yaml_str, _body) = split_frontmatter(&content);
    let Some(yaml_str) = yaml_str else {
        return false;
    };
    let Ok(parsed) = serde_yaml::from_str::<serde_yaml::Value>(&yaml_str) else {
        return false;
    };
    let branch = parsed
        .get("branch")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .unwrap_or("");
    let base_sha = parsed
        .get("base_sha")
        .or_else(|| parsed.get("baseSha"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .unwrap_or("");
    !branch.is_empty() && !base_sha.is_empty()
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
    _bdd_on: bool,
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
        ChangeStage::Draft => {
            // Unified Git-native flow: the "grow up" signal is `change start`
            // (or manual `change attach`), not "add specs/".
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
