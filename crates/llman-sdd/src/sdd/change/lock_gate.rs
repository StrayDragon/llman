//! Locked-rule integrity gate (spec-format r135, git-native-v2 D1/D2).
//!
//! Every `@human` scenario in `llmanspec/specs/**/*.feature` is hashed
//! (normalized, design D4). A bound change MUST NOT add/remove/modify any
//! locked scenario between the **effective range base** (live merge-base of
//! the local default branch with HEAD; falls back to the stored `base_sha`)
//! and the worktree unless its proposal frontmatter carries `rules_touched`
//! (per-req-id granular) or the legacy `rules_edit_acked: true` (blanket).

use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::spec::backend::FEATURE_BACKEND;
use crate::sdd::spec::backend::feature_backend::{self};
use crate::sdd::spec::validation::{ValidationIssue, ValidationLevel};
use anyhow::{Context, anyhow};
use std::collections::BTreeMap;
use std::path::Path;

/// Hash multiset per feature path: hash -> count.
type Hashes = BTreeMap<String, usize>;

/// req-id -> lock hash, so violations can be reported and exempted by
/// requirement id instead of opaque hashes (design D2).
type IdHashes = BTreeMap<String, String>;

/// Locked-rule acknowledgement carrier (design D2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LockedAck {
    /// Legacy `rules_edit_acked: true`: blanket exemption for every edit.
    All,
    /// New granular `rules_touched: [<req-id>...]`: exempt exactly these ids.
    Some(Vec<String>),
    /// No exemption declared.
    None,
}

impl LockedAck {
    /// Build from a parsed proposal frontmatter (bool compat: `true` ≈ All).
    pub(crate) fn from_frontmatter(fm: &crate::sdd::spec::validation::ProposalFrontmatter) -> Self {
        if fm.rules_edit_acked {
            LockedAck::All
        } else if fm.rules_touched.is_empty() {
            LockedAck::None
        } else {
            LockedAck::Some(fm.rules_touched.clone())
        }
    }

    /// True when the given req-id edit is exempted.
    fn exempts(&self, req_id: &str) -> bool {
        match self {
            LockedAck::All => true,
            LockedAck::Some(ids) => ids.iter().any(|id| id == req_id),
            LockedAck::None => false,
        }
    }
}

/// Issues for the locked-rule gate of one change.
pub(crate) fn check(root: &Path, base_sha: &str, ack: &LockedAck) -> Vec<ValidationIssue> {
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

    let mut exempted: Vec<String> = Vec::new();
    let mut violations: Vec<String> = Vec::new();
    for rel in &changed {
        let (before, ids_before) = hashes_at(root, base_sha, rel).unwrap_or_default();
        let (after, _ids_after) = worktree_hashes(root, rel).unwrap_or_default();
        // ADDING rules is normal spec landing; only modifying/removing rules
        // that existed at base requires the human ack.
        if before.is_empty() {
            continue;
        }
        collect_diff(
            rel,
            &before,
            &ids_before,
            &after,
            ack,
            &mut exempted,
            &mut violations,
        );
    }

    if let Some(msg) = lock_gate_message(&exempted, &violations) {
        issues.push(msg);
    }
    issues
}

/// Assemble the single gate issue: ERROR with hints when unexempted edits
/// exist, INFO when every edit is covered by an ack.
fn lock_gate_message(exempted: &[String], violations: &[String]) -> Option<ValidationIssue> {
    if violations.is_empty() && exempted.is_empty() {
        return None;
    }
    let detail = if violations.is_empty() {
        exempted.join("; ")
    } else {
        format!(
            "{}; exempted: {}",
            violations.join("; "),
            exempted.join("; ")
        )
    };
    if violations.is_empty() {
        Some(ValidationIssue {
            level: ValidationLevel::Info,
            path: "lock-gate".to_string(),
            message: format!(
                "locked @human scenarios modified with rules_touched/rules_edit_acked: {detail}"
            ),
        })
    } else {
        Some(ValidationIssue {
            level: ValidationLevel::Error,
            path: "lock-gate".to_string(),
            message: format!(
                "locked @human scenarios were modified without human ack \
                 (add `rules_touched: [<req-id>]` to proposal frontmatter after review; \
                 legacy `rules_edit_acked: true` still accepted): {detail}"
            ),
        })
    }
}

fn collect_diff(
    rel: &str,
    before: &Hashes,
    ids_before: &IdHashes,
    after: &Hashes,
    ack: &LockedAck,
    exempted: &mut Vec<String>,
    violations: &mut Vec<String>,
) {
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
        let kind = if a == 0 { "removed" } else { "modified" };
        // Map the hash back to the req id it carried at base (D2); fall back
        // to the opaque short hash when the id is unknown. One scan serves
        // both the report label and the exemption decision.
        let matched = ids_before.iter().find(|(_, h)| *h == hash);
        let label = matched
            .map(|(id, _)| format!("@req:{id}"))
            .unwrap_or_else(|| {
                let short = &hash[..hash.len().min(12)];
                format!("({short})")
            });
        let entry = format!("{rel}: {kind} rule {label}");
        match matched {
            Some((id, _)) if ack.exempts(id) => exempted.push(entry),
            _ => violations.push(entry),
        }
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

/// Rule-scenario hash multiset from a git object (`base_sha:path`), plus the
/// req-id → hash map captured at that revision.
fn hashes_at(root: &Path, base_sha: &str, rel: &str) -> Option<(Hashes, IdHashes)> {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{base_sha}:{rel}")])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return Some((BTreeMap::new(), BTreeMap::new())); // file did not exist at base
    }
    let content = String::from_utf8(output.stdout).ok()?;
    Some(hashes_from_content(&content))
}

/// Rule-scenario hash multiset from the current working tree (plus id map).
fn worktree_hashes(root: &Path, rel: &str) -> Option<(Hashes, IdHashes)> {
    let content = std::fs::read_to_string(root.join(rel)).ok()?;
    Some(hashes_from_content(&content))
}

fn hashes_from_content(content: &str) -> (Hashes, IdHashes) {
    let mut hashes: Hashes = BTreeMap::new();
    let mut ids: IdHashes = BTreeMap::new();
    if let Ok(parsed) = FEATURE_BACKEND.parse_content(content, "lock-gate") {
        for sc in parsed
            .scenarios
            .iter()
            .filter(|sc| sc.tier.map(|t| t.is_locked()).unwrap_or(false))
        {
            let hash = feature_backend::lock_hash(sc);
            *hashes.entry(hash.clone()).or_insert(0) += 1;
            if let Some(rid) = sc.req_ids.first() {
                ids.entry(rid.clone()).or_insert(hash);
            }
        }
    }
    // Unparseable legacy content yields an empty set; the diff then reports the
    // file as gaining all its current rules, which is the safe direction.
    (hashes, ids)
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

/// Read the locked-rule ack from the change's proposal frontmatter
/// (`rules_touched` list, with legacy `rules_edit_acked: true` = blanket).
pub(crate) fn locked_ack_for(root: &Path, change_name: &str) -> LockedAck {
    let proposal = root
        .join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join(change_name)
        .join("proposal.md");
    let Ok(content) = std::fs::read_to_string(proposal) else {
        return LockedAck::None;
    };
    let (yaml, _body) = crate::sdd::spec::frontmatter::split_frontmatter(&content);
    let Some(yaml) = yaml else {
        return LockedAck::None;
    };
    let Ok(v) = serde_saphyr::from_str::<serde_json::Value>(&yaml) else {
        return LockedAck::None;
    };
    if crate::sdd::spec::validation::parse_yaml_optional_bool(&v, "rules_edit_acked") {
        return LockedAck::All;
    }
    let touched: Vec<String> = v
        .get("rules_touched")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if touched.is_empty() {
        LockedAck::None
    } else {
        LockedAck::Some(touched)
    }
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
        let issues = check(root, &base, &LockedAck::None);
        assert!(
            issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error && i.message.contains("rules_touched")),
            "{issues:?}"
        );
        let issues = check(root, &base, &LockedAck::All);
        assert!(issues.iter().all(|i| i.level != ValidationLevel::Error));

        // Case 2: ADDING a new rule needs no ack.
        std::fs::write(dir.join("demo.feature"), FEATURE_V2_ADDED).unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "v3"]);
        let issues = check(root, &base, &LockedAck::None);
        assert!(
            issues.iter().all(|i| i.level != ValidationLevel::Error),
            "adding rules must not require ack: {issues:?}"
        );

        // Case 3: DELETING the locked rule requires ack.
        std::fs::remove_file(dir.join("demo.feature")).unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "v4"]);
        let issues = check(root, &base, &LockedAck::None);
        assert!(
            issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error && i.message.contains("removed")),
            "{issues:?}"
        );
    }

    #[test]
    fn rules_touched_exempts_only_listed_ids() {
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

        // Modify r1: ERROR when the list does not contain r1 …
        std::fs::write(dir.join("demo.feature"), feature_v2_modified()).unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "v2"]);
        let issues = check(root, &base, &LockedAck::Some(vec!["r2".into()]));
        assert!(
            issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error && i.message.contains("@req:r1")),
            "non-listed id must stay a violation: {issues:?}"
        );
        // … and passes when the list contains r1 (granular exemption).
        let issues = check(root, &base, &LockedAck::Some(vec!["r1".into()]));
        assert!(
            issues.iter().all(|i| i.level != ValidationLevel::Error),
            "listed id must be exempted: {issues:?}"
        );
        assert!(
            issues
                .iter()
                .any(|i| i.level == ValidationLevel::Info && i.message.contains("@req:r1")),
            "exempted edits surface as INFO: {issues:?}"
        );
    }

    /// git-native-v2 D1 regression: with the LIVE merge-base anchor, a change
    /// that merges the default branch (which gained a previous change's
    /// locked-rule edit) stays zero-drift — no ack needed. The STORED
    /// attach-time base would have flagged the merged edit (blanket-ack era).
    #[test]
    fn merged_default_rule_edits_are_immune_with_live_anchor() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let dir = root.join(LLMANSPEC_DIR_NAME).join("specs").join("demo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("demo.feature"), FEATURE_V1).unwrap();
        git(root, &["init", "-q", "-b", "main"]);
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "base"]);

        // Previous change: locked-rule edit on its branch, ff-merged into main.
        git(root, &["checkout", "-q", "-b", "feat/prev"]);
        std::fs::write(dir.join("demo.feature"), feature_v2_modified()).unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "prev rule edit"]);
        git(root, &["checkout", "-q", "main"]);
        git(root, &["merge", "-q", "--ff-only", "feat/prev"]);

        // This change binds at the current merge-base (stored anchor), with
        // docs only — zero spec edits, zero ack.
        let stored_base = String::from_utf8(
            std::process::Command::new("git")
                .args(["rev-parse", "main"])
                .current_dir(root)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        git(root, &["checkout", "-q", "-b", "feat/next"]);
        std::fs::write(root.join("docs.md"), "# next\n").unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "next docs"]);

        // Main moves again with ANOTHER rule edit; next merges it back.
        git(root, &["checkout", "-q", "main"]);
        std::fs::write(
            dir.join("demo.feature"),
            FEATURE_V1.replace("do X.", "do Z."),
        )
        .unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "later rule edit"]);
        git(root, &["checkout", "-q", "feat/next"]);
        git(root, &["merge", "-q", "--no-edit", "main"]);

        // The stored anchor WOULD flag the merged later edit as this change's
        // own violation (old behavior)…
        let stored_issues = check(root, &stored_base, &LockedAck::None);
        assert!(
            stored_issues
                .iter()
                .any(|i| i.level == ValidationLevel::Error),
            "stored anchor must still report the merged edit: {stored_issues:?}"
        );
        // …but the live anchor (D1) is immune: zero drift, gate green.
        let live_base = effective_range_base(root, Some(&stored_base)).unwrap();
        let live_issues = check(root, &live_base, &LockedAck::None);
        assert!(
            live_issues
                .iter()
                .all(|i| i.level != ValidationLevel::Error),
            "live anchor must be accumulation-immune: {live_issues:?}"
        );
    }

    #[test]
    fn rules_edit_acked_accepts_yaml_string_bool_and_list() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let change_dir = root.join(LLMANSPEC_DIR_NAME).join("changes").join("c-ack");
        std::fs::create_dir_all(&change_dir).unwrap();
        std::fs::write(
            change_dir.join("proposal.md"),
            "---\ndepends_on: []\nrules_edit_acked: \"true\"\n---\n## Why\nx\n",
        )
        .unwrap();
        assert!(
            matches!(locked_ack_for(root, "c-ack"), LockedAck::All),
            "string \"true\" must be honored like a YAML bool (H2)"
        );

        // Granular list round-trips through the file reader.
        let change_dir = root
            .join(LLMANSPEC_DIR_NAME)
            .join("changes")
            .join("c-touched");
        std::fs::create_dir_all(&change_dir).unwrap();
        std::fs::write(
            change_dir.join("proposal.md"),
            "---\ndepends_on: []\nrules_touched: [r131, r135]\n---\n## Why\nx\n",
        )
        .unwrap();
        assert_eq!(
            locked_ack_for(root, "c-touched"),
            LockedAck::Some(vec!["r131".into(), "r135".into()])
        );
    }
}
