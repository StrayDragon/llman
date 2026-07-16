use crate::env_safety::validate_user_git_ref;
use crate::sdd::spec::validation::{SpecFrontmatter, ValidationIssue, ValidationLevel};
use serde::Serialize;
use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum StalenessStatus {
    Ok,
    Stale,
    Info,
    Warn,
    NotApplicable,
}

impl StalenessStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StalenessStatus::Ok => "OK",
            StalenessStatus::Stale => "STALE",
            StalenessStatus::Info => "INFO",
            StalenessStatus::Warn => "WARN",
            StalenessStatus::NotApplicable => "NOT_APPLICABLE",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StalenessInfo {
    pub status: StalenessStatus,
    #[serde(rename = "baseRef")]
    pub base_ref: Option<String>,
    pub scope: Vec<String>,
    #[serde(rename = "touchedPaths")]
    pub touched_paths: Vec<String>,
    #[serde(rename = "specUpdated")]
    pub spec_updated: bool,
    pub dirty: bool,
    pub notes: Vec<String>,
}

pub struct StalenessResult {
    pub info: StalenessInfo,
    pub issues: Vec<ValidationIssue>,
}

impl StalenessInfo {
    pub fn not_applicable() -> Self {
        StalenessInfo {
            status: StalenessStatus::NotApplicable,
            base_ref: None,
            scope: Vec::new(),
            touched_paths: Vec::new(),
            spec_updated: false,
            dirty: false,
            notes: Vec::new(),
        }
    }
}

pub struct StalenessEvaluator {
    root: PathBuf,
    base_ref: Option<String>,
    base_ref_invalid: Option<String>,
    merge_base: Option<Result<String, String>>,
    diff_paths: Option<Result<Vec<String>, String>>,
    dirty: Result<bool, String>,
}

impl StalenessEvaluator {
    pub fn new(root: &Path) -> Self {
        let root = root.to_path_buf();
        let dirty = git_status_dirty(&root);
        match resolve_base_ref(&root) {
            Ok(base_ref) => {
                let merge_base = base_ref
                    .as_deref()
                    .map(|reference| resolve_merge_base(&root, reference));
                let diff_paths = match merge_base.as_ref() {
                    Some(Ok(base)) => Some(git_diff_names(&root, base)),
                    _ => None,
                };
                Self {
                    root,
                    base_ref,
                    base_ref_invalid: None,
                    merge_base,
                    diff_paths,
                    dirty,
                }
            }
            Err(err) => Self {
                root,
                base_ref: None,
                base_ref_invalid: Some(err),
                merge_base: None,
                diff_paths: None,
                dirty,
            },
        }
    }

    pub fn evaluate(
        &self,
        spec_id: &str,
        spec_path: &Path,
        frontmatter: Option<&SpecFrontmatter>,
        spec_updated_override: Option<bool>,
    ) -> StalenessResult {
        let mut issues = Vec::new();
        let mut notes = Vec::new();
        let mut status = StalenessStatus::Ok;

        if let Some(msg) = &self.base_ref_invalid {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: format!("{spec_id}/staleness"),
                message: msg.clone(),
            });
            notes.push(msg.clone());
            let dirty = self.dirty.clone().unwrap_or(true);
            return StalenessResult {
                info: StalenessInfo {
                    status: StalenessStatus::Warn,
                    base_ref: None,
                    scope: frontmatter
                        .map(|fm| normalize_scope_list(&fm.valid_scope))
                        .unwrap_or_default(),
                    touched_paths: Vec::new(),
                    spec_updated: false,
                    dirty,
                    notes,
                },
                issues,
            };
        }

        let scope = frontmatter
            .map(|fm| normalize_scope_list(&fm.valid_scope))
            .unwrap_or_default();
        if scope.is_empty() {
            status = StalenessStatus::Warn;
            notes.push(t!("sdd.validate.staleness_scope_missing").to_string());
            issues.push(ValidationIssue {
                level: ValidationLevel::Warning,
                path: format!("{spec_id}/staleness"),
                message: t!("sdd.validate.staleness_scope_missing").to_string(),
            });
        }

        let base_ref = self.base_ref.clone();
        if base_ref.is_none() {
            status = StalenessStatus::Warn;
            notes.push(t!("sdd.validate.staleness_base_missing").to_string());
            issues.push(ValidationIssue {
                level: ValidationLevel::Warning,
                path: format!("{spec_id}/staleness"),
                message: t!("sdd.validate.staleness_base_missing").to_string(),
            });
        }

        let mut touched_paths = Vec::new();
        let mut spec_updated = false;

        if status != StalenessStatus::Warn && base_ref.is_some() {
            match self.merge_base.as_ref() {
                Some(Ok(_base)) => {
                    let diff_paths = match self.diff_paths.as_ref() {
                        Some(Ok(paths)) => paths.clone(),
                        Some(Err(err)) => {
                            status = StalenessStatus::Warn;
                            notes.push(err.clone());
                            issues.push(ValidationIssue {
                                level: ValidationLevel::Warning,
                                path: format!("{spec_id}/staleness"),
                                message: err.clone(),
                            });
                            Vec::new()
                        }
                        None => Vec::new(),
                    };

                    if !diff_paths.is_empty() {
                        let spec_rel = spec_relative_path(&self.root, spec_path);
                        spec_updated = diff_paths.iter().any(|path| path == &spec_rel);
                        touched_paths = diff_paths
                            .iter()
                            .filter(|path| scope_matches(path, &scope))
                            .cloned()
                            .collect();
                    }

                    let mut spec_updated_effective = spec_updated;
                    if let Some(value) = spec_updated_override {
                        spec_updated_effective = value;
                    }

                    if !touched_paths.is_empty() && !spec_updated_effective {
                        status = StalenessStatus::Stale;
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Warning,
                            path: format!("{spec_id}/staleness"),
                            message: t!("sdd.validate.staleness_stale").to_string(),
                        });
                    } else if spec_updated_effective && touched_paths.is_empty() {
                        status = StalenessStatus::Info;
                        notes.push(t!("sdd.validate.staleness_spec_updated").to_string());
                    }
                    spec_updated = spec_updated_effective;
                }
                Some(Err(err)) => {
                    status = StalenessStatus::Warn;
                    notes.push(err.clone());
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Warning,
                        path: format!("{spec_id}/staleness"),
                        message: err.clone(),
                    });
                }
                None => {
                    status = StalenessStatus::Warn;
                    let err = t!("sdd.validate.staleness_git_failed").to_string();
                    notes.push(err.clone());
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Warning,
                        path: format!("{spec_id}/staleness"),
                        message: err,
                    });
                }
            }
        }

        if let Some(value) = spec_updated_override {
            spec_updated = value;
            if value && status == StalenessStatus::Ok && touched_paths.is_empty() {
                status = StalenessStatus::Info;
                notes.push(t!("sdd.validate.staleness_spec_updated").to_string());
            }
        }

        let dirty = match &self.dirty {
            Ok(dirty) => *dirty,
            Err(err) => {
                notes.push(err.clone());
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    path: format!("{spec_id}/staleness"),
                    message: err.clone(),
                });
                true
            }
        };

        if dirty {
            if status == StalenessStatus::Ok {
                status = StalenessStatus::Info;
            }
            notes.push(t!("sdd.validate.staleness_dirty").to_string());
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: format!("{spec_id}/staleness"),
                message: t!("sdd.validate.staleness_dirty").to_string(),
            });
        }

        let info = StalenessInfo {
            status,
            base_ref,
            scope,
            touched_paths,
            spec_updated,
            dirty,
            notes,
        };

        StalenessResult { info, issues }
    }
}

pub fn evaluate_staleness(
    root: &Path,
    spec_id: &str,
    spec_path: &Path,
    frontmatter: Option<&SpecFrontmatter>,
) -> StalenessResult {
    evaluate_staleness_with_override(root, spec_id, spec_path, frontmatter, None)
}

pub fn evaluate_staleness_with_override(
    root: &Path,
    spec_id: &str,
    spec_path: &Path,
    frontmatter: Option<&SpecFrontmatter>,
    spec_updated_override: Option<bool>,
) -> StalenessResult {
    StalenessEvaluator::new(root).evaluate(spec_id, spec_path, frontmatter, spec_updated_override)
}

fn resolve_base_ref(root: &Path) -> Result<Option<String>, String> {
    let env_ref = env::var("LLMANSPEC_BASE_REF")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(env_ref) = env_ref {
        validate_user_git_ref(&env_ref).map_err(|_| {
            t!(
                "sdd.validate.staleness_base_ref_invalid",
                value = env_ref.clone()
            )
            .to_string()
        })?;
        return Ok(Some(env_ref));
    }
    if git_ref_exists(root, "origin/main") {
        return Ok(Some("origin/main".to_string()));
    }
    if git_ref_exists(root, "origin/master") {
        return Ok(Some("origin/master".to_string()));
    }
    if git_ref_exists(root, "main") {
        return Ok(Some("main".to_string()));
    }
    if git_ref_exists(root, "master") {
        return Ok(Some("master".to_string()));
    }
    Ok(None)
}

fn git_ref_exists(root: &Path, reference: &str) -> bool {
    // NOTE: do NOT insert `--` before `reference` here. `rev-parse --verify`
    // treats `--` as an end-of-options separator, which makes git interpret the
    // following argument as a PATH rather than a ref — so `-- origin/main` would
    // always fail. All callers pass validated refs (hardcoded literals or values
    // sanitized by `validate_user_git_ref`), so option injection is not a concern.
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .current_dir(root)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn resolve_merge_base(root: &Path, reference: &str) -> Result<String, String> {
    run_git(root, &["merge-base", "--", reference, "HEAD"])
}

fn git_diff_names(root: &Path, base: &str) -> Result<Vec<String>, String> {
    let output = run_git(
        root,
        &["diff", "--name-only", "--", &format!("{base}..HEAD")],
    )?;
    let mut paths = BTreeSet::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        paths.insert(trimmed.to_string());
    }
    Ok(paths.into_iter().collect())
}

fn git_status_dirty(root: &Path) -> Result<bool, String> {
    let output = run_git(root, &["status", "--porcelain"])?;
    Ok(!output.trim().is_empty())
}

fn run_git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err(t!("sdd.validate.staleness_git_failed").to_string());
        }
        return Err(stderr);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn normalize_scope_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| normalize_path(value))
        .filter(|value| !value.is_empty())
        .collect()
}

fn normalize_path(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_string()
}

fn scope_matches(path: &str, scope: &[String]) -> bool {
    let normalized_path = normalize_path(path);
    scope
        .iter()
        .any(|scope| normalized_path == *scope || normalized_path.starts_with(&format!("{scope}/")))
}

fn spec_relative_path(root: &Path, spec_path: &Path) -> String {
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let spec_path = std::fs::canonicalize(spec_path).unwrap_or_else(|_| spec_path.to_path_buf());
    let rel = spec_path.strip_prefix(&root).unwrap_or(&spec_path);
    normalize_path(&path_to_slash(rel))
}

fn path_to_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_matches_exact_or_prefix() {
        let scope = vec!["src".to_string(), "README.md".to_string()];
        assert!(scope_matches("src/lib.rs", &scope));
        assert!(scope_matches("src/sub/file.rs", &scope));
        assert!(scope_matches("README.md", &scope));
        assert!(!scope_matches("docs/readme.md", &scope));
    }

    #[test]
    fn invalid_llmanspec_base_ref_is_error_without_git() {
        let mut proc = crate::test_utils::TestProcess::new();
        proc.set_var("LLMANSPEC_BASE_REF", "-c");
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let result = StalenessEvaluator::new(root).evaluate(
            "sample",
            &root.join("llmanspec/specs/sample/spec.toon"),
            None,
            None,
        );
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error
                    && i.message.contains("LLMANSPEC_BASE_REF")),
            "expected invalid base-ref error, got {:?}",
            result.issues
        );
        assert!(result.info.base_ref.is_none());
    }

    /// Regression for 5909cfc: `git_ref_exists` must NOT pass `--` before the
    /// ref, otherwise git treats the ref as a PATH and resolution always fails
    /// even when `origin/main` clearly exists. Build a real temp git repo with
    /// an `origin/main` ref and assert it is resolved.
    #[test]
    fn base_ref_resolves_when_origin_main_exists() {
        let mut proc = crate::test_utils::TestProcess::new();
        proc.remove_var("LLMANSPEC_BASE_REF");
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        // init repo + a commit on the default branch
        run_test_git(root, &["init", "-q", "-b", "main"]);
        std::fs::write(root.join("README"), "hi").unwrap();
        run_test_git(root, &["add", "README"]);
        run_test_git(root, &["commit", "-q", "-m", "init"]);
        // create origin/main as a remote-tracking ref pointing at the same commit
        run_test_git(root, &["remote", "add", "origin", root.to_str().unwrap()]);
        run_test_git(root, &["fetch", "-q", "origin"]);
        // resolve and assert
        let evaluator = StalenessEvaluator::new(root);
        assert_eq!(evaluator.base_ref.as_deref(), Some("origin/main"));
        assert!(
            evaluator.base_ref_invalid.is_none(),
            "expected no base-ref error, got {:?}",
            evaluator.base_ref_invalid
        );
    }

    fn run_test_git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .output()
            .expect("git command succeeds");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
