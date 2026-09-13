//! Locked-rule report (spec-format r135 report-only, git-native-v2 D1/D2).
//!
//! Every `@human` scenario in `llmanspec/specs/**/*.feature` is hashed
//! (normalized, design D4). Locked-scenario add/remove/modify between the
//! **effective range base** (live merge-base of the local default branch with
//! HEAD; falls back to the stored `base_sha`) and the worktree is REPORTED as
//! WARNING — it never blocks validate/finalize/diff. The human control points
//! are the git branch diff and the surfaced report (`llman sdd review`,
//! `change diff`). Ack metadata (`rules_touched` / `agent_acked` / `@agent`)
//! is removed (S0, q9 no-compat).

use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::spec::backend::FEATURE_BACKEND;
use crate::sdd::spec::backend::feature_backend::{self};
use crate::sdd::spec::validation::{ValidationIssue, ValidationLevel};
use anyhow::{Context, anyhow};
use std::collections::BTreeMap;
use std::path::Path;

/// Hash multiset per feature path: hash -> count.
type Hashes = BTreeMap<String, usize>;

/// lock hash -> req-id (first locked scenario carrying the hash wins), so
/// reports can name the requirement id instead of opaque hashes (design D2).
/// Inverse of the former id->hash map, which lost the id for every
/// duplicate-req-id scenario beyond the first (issue #18).
type HashIds = BTreeMap<String, String>;

/// Locked-rule parse of one feature revision (base or worktree).
#[derive(Default)]
struct LockedContent {
    hashes: Hashes,
    hash_ids: HashIds,
}

/// One locked-rule edit detected between base and worktree (r135).
pub(crate) struct RuleEdit {
    pub(crate) rel: String,
    pub(crate) kind: &'static str, // "removed" | "modified"
    pub(crate) req_id: Option<String>,
    pub(crate) hash: String,
}

/// Locked-rule edits of one change, reported as WARNING (never blocking).
pub(crate) fn check(root: &Path, base_sha: &str) -> Vec<ValidationIssue> {
    let specs_prefix = format!("{LLMANSPEC_DIR_NAME}/specs/");

    let edits = match diff_edits(root, base_sha, &specs_prefix) {
        Ok(edits) => edits,
        Err(err) => {
            return vec![ValidationIssue {
                level: ValidationLevel::Warning,
                path: "lock-gate".to_string(),
                message: format!("could not diff locked rules against base: {err}"),
            }];
        }
    };
    if edits.is_empty() {
        return Vec::new();
    }

    let mut entries: Vec<String> = Vec::new();
    let mut hash_only_edits = 0usize;
    for edit in &edits {
        match &edit.req_id {
            Some(id) => entries.push(format!("{}: {} rule @req:{id}", edit.rel, edit.kind)),
            None => {
                let short = &edit.hash[..edit.hash.len().min(12)];
                entries.push(format!("{}: {} rule ({short})", edit.rel, edit.kind));
                hash_only_edits += 1;
            }
        }
    }

    let mut detail = entries.join("; ");
    if hash_only_edits > 0 {
        detail.push_str(&format!(
            "; {hash_only_edits} edited rule(s) carry no @req tag — restore them \
             or add `@req:<id>` tags"
        ));
    }
    vec![ValidationIssue {
        level: ValidationLevel::Warning,
        path: "lock-gate".to_string(),
        message: format!(
            "locked @human scenarios were modified (spec-format r135 report-only; \
             compare via git branch diff or `llman sdd review`; {} edited): {detail}",
            edits.len()
        ),
    }]
}

/// Number of locked-rule edits between base and worktree (0 when git fails).
pub(crate) fn edited_locked_rule_count(root: &Path, base_sha: &str) -> usize {
    let specs_prefix = format!("{LLMANSPEC_DIR_NAME}/specs/");
    diff_edits(root, base_sha, &specs_prefix)
        .map(|edits| edits.len())
        .unwrap_or(0)
}

/// Detect locked-rule edits (removal/modification of rules that existed at
/// base) between `base_sha` and the working tree. ADDING rules is normal
/// spec landing and never reported. Shared by [`check`] and
/// [`edited_locked_rule_count`].
fn diff_edits(root: &Path, base_sha: &str, prefix: &str) -> anyhow::Result<Vec<RuleEdit>> {
    let changed = changed_feature_files(root, base_sha, prefix)?;
    let mut edits = Vec::new();
    for rel in &changed {
        let before = hashes_at(root, base_sha, rel).unwrap_or_default();
        let after = worktree_hashes(root, rel).unwrap_or_default();
        if before.hashes.is_empty() {
            continue;
        }
        let mut keys: std::collections::BTreeSet<&String> = before.hashes.keys().collect();
        keys.extend(after.hashes.keys());
        for hash in keys {
            let b = before.hashes.get(hash).copied().unwrap_or(0);
            let a = after.hashes.get(hash).copied().unwrap_or(0);
            if b == 0 || b == a {
                continue;
            }
            let kind = if a == 0 { "removed" } else { "modified" };
            let req_id = before.hash_ids.get(hash).cloned();
            edits.push(RuleEdit {
                rel: rel.clone(),
                kind,
                req_id,
                hash: hash.clone(),
            });
        }
    }
    Ok(edits)
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

/// Locked-rule content snapshot from a git object (`base_sha:path`).
fn hashes_at(root: &Path, base_sha: &str, rel: &str) -> Option<LockedContent> {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{base_sha}:{rel}")])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return Some(LockedContent::default()); // file did not exist at base
    }
    let content = String::from_utf8(output.stdout).ok()?;
    Some(hashes_from_content(&content))
}

/// Locked-rule content snapshot from the current working tree.
fn worktree_hashes(root: &Path, rel: &str) -> Option<LockedContent> {
    let content = std::fs::read_to_string(root.join(rel)).ok()?;
    Some(hashes_from_content(&content))
}

fn hashes_from_content(content: &str) -> LockedContent {
    let mut hashes: Hashes = BTreeMap::new();
    let mut hash_ids: HashIds = BTreeMap::new();
    if let Ok(parsed) = FEATURE_BACKEND.parse_content(content, "lock-gate") {
        for sc in parsed
            .scenarios
            .iter()
            .filter(|sc| sc.tier.map(|t| t.is_locked()).unwrap_or(false))
        {
            let hash = feature_backend::lock_hash(sc);
            *hashes.entry(hash.clone()).or_insert(0) += 1;
            if let Some(rid) = sc.req_ids.first() {
                hash_ids.entry(hash.clone()).or_insert(rid.clone());
            }
        }
    }
    // Unparseable legacy content yields an empty set; the diff then reports the
    // file as gaining all its current rules, which is the safe direction.
    LockedContent { hashes, hash_ids }
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

/// Range base for locked-rule diffing (git-native-v2 D1): the live
/// merge-base of the local default branch with HEAD; falls back to the stored
/// attach `base_sha` when git is unavailable (fail-open, pre-v2 behavior).
pub(crate) fn effective_range_base(root: &Path, stored: Option<&str>) -> anyhow::Result<String> {
    if let Ok(base) = crate::git_utils::effective_range_base(root) {
        return Ok(base);
    }
    stored
        .filter(|b| !b.trim().is_empty())
        .map(|b| b.trim().to_string())
        .ok_or_else(|| anyhow!("no git range base available"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str =
        "# language: en\n# capability: demo\n# purpose: p\n# scope: src/\n\nFeature: demo\n";

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

    fn commit_all(root: &Path, msg: &str) {
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", msg]);
    }

    fn head_sha(root: &Path) -> String {
        String::from_utf8(
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(root)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string()
    }

    fn seed(root: &Path, body: &str) -> String {
        let dir = root.join(LLMANSPEC_DIR_NAME).join("specs").join("demo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("demo.feature"), body).unwrap();
        git(root, &["init", "-q"]);
        commit_all(root, "base");
        head_sha(root)
    }

    fn first_warning(issues: &[ValidationIssue]) -> &ValidationIssue {
        issues
            .iter()
            .find(|i| i.level == ValidationLevel::Warning)
            .expect("expected a lock-gate WARNING")
    }

    /// Modify + delete are reported; adding is never reported.
    #[test]
    fn modify_and_delete_report_but_adding_does_not() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let feature = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join("demo")
            .join("demo.feature");
        let base = seed(
            root,
            &format!("{HEADER}\n  @req:r1 @human\n  Scenario: R1\n    System MUST do X.\n"),
        );

        // Modify the locked rule -> WARNING.
        std::fs::write(
            &feature,
            format!("{HEADER}\n  @req:r1 @human\n  Scenario: R1\n    System MUST do Y.\n"),
        )
        .unwrap();
        commit_all(root, "v2");
        let issues = check(root, &base);
        assert!(
            first_warning(&issues).message.contains("@req:r1"),
            "{issues:?}"
        );
        assert!(
            first_warning(&issues).message.contains("1 edited"),
            "{issues:?}"
        );

        // ADDING a new rule (content back to base state + r2) needs no report.
        std::fs::write(
            &feature,
            format!("{HEADER}\n  @req:r1 @human\n  Scenario: R1\n    System MUST do X.\n\n  @req:r2 @human\n  Scenario: R2\n    System MUST do Z.\n"),
        )
        .unwrap();
        commit_all(root, "v3");
        assert!(check(root, &base).is_empty(), "adding must not report");
    }

    /// Issue #18 fix: two locked scenarios sharing one req-id (different
    /// content) must both be reported by that req-id — never as hash-only
    /// entries, which no reader could act on.
    #[test]
    fn duplicate_req_id_removals_report_by_id() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let feature = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join("demo")
            .join("demo.feature");
        let dup = format!(
            "{HEADER}\n  @req:r1 @human\n  Scenario: A\n    System MUST do X quickly.\n\n  @req:r1 @human\n  Scenario: B\n    System MUST do X slowly.\n"
        );
        let base = seed(root, &dup);

        std::fs::write(&feature, HEADER).unwrap();
        commit_all(root, "compact: drop duplicate r1 rules");

        let issues = check(root, &base);
        let warning = first_warning(&issues);
        assert_eq!(warning.message.matches("@req:r1").count(), 2, "{warning:?}");
        assert!(
            !warning.message.contains("removed rule ("),
            "hash-only entries are forbidden: {warning:?}"
        );
        assert!(warning.message.contains("2 edited"), "{warning:?}");
        assert_eq!(edited_locked_rule_count(root, &base), 2);
    }

    /// Different req-ids with the same statement still hash apart (id and
    /// name feed the hash) and report separately.
    #[test]
    fn same_statement_different_ids_report_separately() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let feature = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join("demo")
            .join("demo.feature");
        let base = seed(
            root,
            &format!(
                "{HEADER}\n  @req:r5 @human\n  Scenario: R5\n    System MUST do X.\n\n  @req:r6 @human\n  Scenario: R6\n    System MUST do X.\n"
            ),
        );

        std::fs::write(&feature, HEADER).unwrap();
        commit_all(root, "compact: drop identical rules");

        let issues = check(root, &base);
        let warning = first_warning(&issues);
        assert!(warning.message.contains("@req:r5"), "{warning:?}");
        assert!(warning.message.contains("@req:r6"), "{warning:?}");
    }

    /// r135: locked scenarios without an `@req` tag are reported with a
    /// restore-or-tag hint (there is no req-id to name for them).
    #[test]
    fn reqless_locked_rule_edit_hints_restore_or_tag() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let feature = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join("demo")
            .join("demo.feature");
        let base = seed(
            root,
            &format!("{HEADER}\n  @human\n  Scenario: Tagless\n    System MUST do X.\n"),
        );

        std::fs::write(&feature, HEADER).unwrap();
        commit_all(root, "compact: drop tagless rule");

        let issues = check(root, &base);
        let warning = first_warning(&issues);
        assert!(warning.message.contains("carry no @req tag"), "{warning:?}");
    }

    /// git-native-v2 D1 regression: with the LIVE merge-base anchor, a change
    /// that merges the default branch (which gained a previous change's
    /// locked-rule edit) stays zero-drift — no report. The STORED attach-time
    /// base would have flagged the merged edit.
    #[test]
    fn merged_default_rule_edits_are_immune_with_live_anchor() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let dir = root.join(LLMANSPEC_DIR_NAME).join("specs").join("demo");
        std::fs::create_dir_all(&dir).unwrap();
        let feature = dir.join("demo.feature");
        std::fs::write(
            &feature,
            format!("{HEADER}\n  @req:r1 @human\n  Scenario: R1\n    System MUST do X.\n"),
        )
        .unwrap();
        git(root, &["init", "-q", "-b", "main"]);
        commit_all(root, "base");

        // Previous change: locked-rule edit on its branch, ff-merged into main.
        git(root, &["checkout", "-q", "-b", "feat/prev"]);
        std::fs::write(
            &feature,
            format!("{HEADER}\n  @req:r1 @human\n  Scenario: R1\n    System MUST do Y.\n"),
        )
        .unwrap();
        commit_all(root, "prev rule edit");
        git(root, &["checkout", "-q", "main"]);
        git(root, &["merge", "-q", "--ff-only", "feat/prev"]);

        let stored_base = head_sha(root);
        git(root, &["checkout", "-q", "-b", "feat/next"]);
        std::fs::write(root.join("docs.md"), "# next\n").unwrap();
        commit_all(root, "next docs");

        // Main moves again with ANOTHER rule edit; next merges it back.
        git(root, &["checkout", "-q", "main"]);
        std::fs::write(
            &feature,
            format!("{HEADER}\n  @req:r1 @human\n  Scenario: R1\n    System MUST do Z.\n"),
        )
        .unwrap();
        commit_all(root, "later rule edit");
        git(root, &["checkout", "-q", "feat/next"]);
        git(root, &["merge", "-q", "--no-edit", "main"]);

        // The stored anchor WOULD report the merged later edit…
        assert!(!check(root, &stored_base).is_empty());
        // …but the live anchor (D1) is immune: zero drift, no report.
        let live_base = effective_range_base(root, Some(&stored_base)).unwrap();
        assert!(check(root, &live_base).is_empty());
    }
}
