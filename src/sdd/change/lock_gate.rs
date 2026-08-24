//! Locked-rule integrity gate (spec-format r135).
//!
//! Every `@human` scenario in `llmanspec/specs/**/*.feature` is hashed
//! (normalized, design D4). A bound change MUST NOT add/remove/modify any
//! locked scenario between `base_sha` and HEAD unless its proposal
//! frontmatter carries `rules_edit_acked: true`.

use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::spec::backend::FEATURE_BACKEND;
use crate::sdd::spec::backend::feature_backend::{self};
use crate::sdd::spec::validation::{ValidationIssue, ValidationLevel};
use anyhow::Context;
use std::collections::BTreeMap;
use std::path::Path;

/// Hash multiset per feature path: hash -> count.
type Hashes = BTreeMap<String, usize>;

/// Issues for the locked-rule gate of one change.
pub fn check(root: &Path, base_sha: &str, rules_edit_acked: bool) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let specs_prefix = format!("{LLMANSPEC_DIR_NAME}/specs/");

    let changed = match changed_feature_files(root, base_sha, &specs_prefix) {
        Ok(files) => files,
        Err(err) => {
            issues.push(ValidationIssue {
                level: ValidationLevel::Warning,
                path: "lock-gate".to_string(),
                message: format!("could not diff locked rules against base: {err}"),
            });
            return issues;
        }
    };
    if changed.is_empty() {
        return issues;
    }

    let mut violations: Vec<String> = Vec::new();
    let mut touched_locked = false;
    for rel in &changed {
        let before = hashes_at(root, base_sha, rel).unwrap_or_default();
        let after = worktree_hashes(root, rel).unwrap_or_default();
        // ADDING rules is normal spec landing; only modifying/removing rules
        // that existed at base requires the human ack.
        if before.is_empty() {
            continue;
        }
        touched_locked = true;
        report_diff(rel, &before, &after, &mut violations);
    }

    if touched_locked && !violations.is_empty() {
        if rules_edit_acked {
            issues.push(ValidationIssue {
                level: ValidationLevel::Info,
                path: "lock-gate".to_string(),
                message: format!(
                    "locked @human scenarios modified with rules_edit_acked: {}",
                    violations.join("; ")
                ),
            });
        } else {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                path: "lock-gate".to_string(),
                message: format!(
                    "locked @human scenarios were modified without human ack \
                     (add `rules_edit_acked: true` to proposal frontmatter after review): {}",
                    violations.join("; ")
                ),
            });
        }
    }
    issues
}

fn report_diff(rel: &str, before: &Hashes, after: &Hashes, out: &mut Vec<String>) {
    let mut keys: std::collections::BTreeSet<&String> = before.keys().collect();
    keys.extend(after.keys());
    for hash in keys {
        let b = before.get(hash).copied().unwrap_or(0);
        let a = after.get(hash).copied().unwrap_or(0);
        // Additions are normal spec landing; only removal/modification of a
        // rule that existed at base is a locked-edit.
        if b == 0 || b == a {
            continue;
        }
        let kind = if b == 0 {
            "added"
        } else if a == 0 {
            "removed"
        } else {
            "modified"
        };
        let short = &hash[..hash.len().min(12)];
        out.push(format!("{rel}: {kind} rule ({short})"));
    }
}

/// Paths under `<prefix>` (`.feature` only) that differ between base and HEAD.
fn changed_feature_files(root: &Path, base_sha: &str, prefix: &str) -> anyhow::Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", base_sha, "HEAD", "--", prefix])
        .current_dir(root)
        .output()
        .context("git diff --name-only")?;
    ensure_success(&output)?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| l.ends_with(".feature"))
        .map(str::to_string)
        .collect())
}

/// Rule-scenario hash multiset from a git object (`base_sha:path`).
fn hashes_at(root: &Path, base_sha: &str, rel: &str) -> Option<Hashes> {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{base_sha}:{rel}")])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return Some(BTreeMap::new()); // file did not exist at base
    }
    let content = String::from_utf8(output.stdout).ok()?;
    Some(hashes_from_content(&content))
}

/// Rule-scenario hash multiset from the current working tree.
fn worktree_hashes(root: &Path, rel: &str) -> Option<Hashes> {
    let content = fs_read(root.join(rel)).ok()?;
    Some(hashes_from_content(&content))
}

fn fs_read(path: std::path::PathBuf) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

fn hashes_from_content(content: &str) -> Hashes {
    let mut hashes: Hashes = BTreeMap::new();
    if let Ok(parsed) = FEATURE_BACKEND.parse_content(content, "lock-gate") {
        for sc in parsed
            .scenarios
            .iter()
            .filter(|sc| sc.tier.map(|t| t.is_locked()).unwrap_or(false))
        {
            *hashes.entry(feature_backend::lock_hash(sc)).or_insert(0) += 1;
        }
    }
    // Unparseable legacy content yields an empty set; the diff then reports the
    // file as gaining all its current rules, which is the safe direction.
    hashes
}

fn ensure_success(output: &std::process::Output) -> anyhow::Result<()> {
    if !output.status.success() {
        anyhow::bail!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Read `rules_edit_acked` from the change's proposal frontmatter.
pub fn rules_edit_acked_for(root: &Path, change_name: &str) -> bool {
    let proposal = root
        .join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join(change_name)
        .join("proposal.md");
    let Ok(content) = std::fs::read_to_string(proposal) else {
        return false;
    };
    let (yaml, _body) = crate::sdd::spec::frontmatter::split_frontmatter(&content);
    let Some(yaml) = yaml else {
        return false;
    };
    serde_yaml::from_str::<serde_yaml::Value>(&yaml)
        .ok()
        .map(|v| crate::sdd::spec::validation::parse_yaml_optional_bool(&v, "rules_edit_acked"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEATURE_V1: &str = "\
# language: en\n# capability: demo\n# purpose: p\n# scope: src/\n\nFeature: demo\n\n  @req:r1 @human\n  Scenario: R1\n    System MUST do X.\n";
    fn feature_v2_modified() -> String {
        FEATURE_V1.replace("do X.", "do Y.")
    }
    const FEATURE_V2_ADDED: &str = concat!(
        "# language: en\n# capability: demo\n# purpose: p\n# scope: src/\n\nFeature: demo\n\n",
        "  @req:r1 @human\n  Scenario: R1\n    System MUST do X.\n\n",
        "  @req:r2 @human\n  Scenario: R2\n    System MUST do Z.\n"
    );

    fn git(root: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@x"])
            .arg(args[0])
            .args(&args[1..])
            .current_dir(root)
            .output()
            .expect("git");
        assert!(out.status.success(), "git {:?} failed", args);
    }

    #[test]
    fn modify_and_delete_require_ack_but_adding_does_not() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let dir = root.join(LLMANSPEC_DIR_NAME).join("specs").join("demo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("demo.feature"), FEATURE_V1).unwrap();
        git(root, &["init", "-q"]);
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "base"]);
        let base = String::from_utf8(
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(root)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();

        // Case 1: modify the locked rule -> ERROR without ack, INFO with ack.
        std::fs::write(dir.join("demo.feature"), feature_v2_modified()).unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "v2"]);
        let issues = check(root, &base, false);
        assert!(
            issues.iter().any(
                |i| i.level == ValidationLevel::Error && i.message.contains("rules_edit_acked")
            ),
            "{issues:?}"
        );
        let issues = check(root, &base, true);
        assert!(issues.iter().all(|i| i.level != ValidationLevel::Error));

        // Case 2: ADDING a new rule needs no ack.
        std::fs::write(dir.join("demo.feature"), FEATURE_V2_ADDED).unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "v3"]);
        let issues = check(root, &base, false);
        assert!(
            issues.iter().all(|i| i.level != ValidationLevel::Error),
            "adding rules must not require ack: {issues:?}"
        );

        // Case 3: DELETING the locked rule requires ack.
        std::fs::remove_file(dir.join("demo.feature")).unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "v4"]);
        let issues = check(root, &base, false);
        assert!(
            issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error && i.message.contains("removed")),
            "{issues:?}"
        );
    }
}
