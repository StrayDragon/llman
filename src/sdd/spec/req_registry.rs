//! Global `req_id` registry for main-library specs under `llmanspec/specs/`.
//!
//! `req_id` values are short aliases that MUST be unique across all capabilities.
//! Ownership / display is resolved via CLI (`resolve-req`), not encoded in the id.

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::shared::discovery::list_specs;
use crate::sdd::spec::backend::{BACKEND, SpecBackend};
use crate::sdd::spec::ir::MainSpecDoc;
use crate::sdd::spec::partitioned::parse_feature_scenarios;
use crate::sdd::spec::validation::{ValidationIssue, ValidationLevel, discover_features};
use anyhow::{Result, anyhow, bail};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// One requirement row located in the main library.
#[derive(Debug, Clone)]
pub struct ReqLocation {
    pub capability: String,
    pub title: String,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveReqJson {
    pub req_id: String,
    pub capability: String,
    pub title: String,
    pub statement: String,
    pub harness: Vec<HarnessRefJson>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessRefJson {
    pub feature: String,
    pub scenario: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextReqIdJson {
    pub req_id: String,
}

/// Index: req_id → all owning capabilities (normally one).
pub fn build_req_index(root: &Path) -> Result<BTreeMap<String, Vec<ReqLocation>>> {
    let mut index: BTreeMap<String, Vec<ReqLocation>> = BTreeMap::new();
    for capability in list_specs(root)? {
        let doc = load_main_doc(root, &capability)?;
        for req in &doc.requirements {
            let id = req.req_id.trim().to_string();
            if id.is_empty() {
                continue;
            }
            index.entry(id).or_default().push(ReqLocation {
                capability: capability.clone(),
                title: req.title.clone(),
                statement: req.statement.clone(),
            });
        }
    }
    Ok(index)
}

pub fn load_main_doc(root: &Path, capability: &str) -> Result<MainSpecDoc> {
    let path = spec_toon_path(root, capability);
    let content = fs::read_to_string(&path)
        .map_err(|err| anyhow!("failed to read {}: {err}", path.display()))?;
    BACKEND.parse_main_spec(&content, &format!("spec `{capability}`"))
}

fn spec_toon_path(root: &Path, capability: &str) -> PathBuf {
    root.join(LLMANSPEC_DIR_NAME)
        .join("specs")
        .join(capability)
        .join(SPEC_FILE)
}

fn specs_dir(root: &Path) -> PathBuf {
    root.join(LLMANSPEC_DIR_NAME).join("specs")
}

/// ERROR issues for every globally colliding `req_id`, with fix suggestions.
pub fn global_req_id_uniqueness_issues(root: &Path) -> Vec<ValidationIssue> {
    match build_req_index(root) {
        Ok(index) => uniqueness_issues_from_index(&index),
        Err(err) => vec![ValidationIssue {
            level: ValidationLevel::Error,
            path: "llmanspec/specs".to_string(),
            message: format!("Failed to scan req_id index: {err}"),
        }],
    }
}

/// Issues relevant to a single capability (subset of global collisions).
pub fn global_req_id_uniqueness_issues_for_capability(
    root: &Path,
    capability: &str,
) -> Vec<ValidationIssue> {
    global_req_id_uniqueness_issues(root)
        .into_iter()
        .filter(|issue| issue.message.contains(capability) || issue.path.contains(capability))
        .collect()
}

fn uniqueness_issues_from_index(
    index: &BTreeMap<String, Vec<ReqLocation>>,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for (req_id, locs) in index {
        let mut caps: BTreeSet<&str> = BTreeSet::new();
        for loc in locs {
            caps.insert(loc.capability.as_str());
        }
        if caps.len() < 2 {
            continue;
        }
        let cap_list = caps.iter().cloned().collect::<Vec<_>>().join(", ");
        issues.push(ValidationIssue {
            level: ValidationLevel::Error,
            path: format!("llmanspec/specs (req_id `{req_id}`)"),
            message: format!(
                "Global duplicate req_id `{req_id}` used by capabilities: {cap_list}. \
                 Fix: keep one owner, remap others with a fresh short id from \
                 `llman sdd spec next-req-id` (or `llman sdd project dedupe-req-ids`); \
                 inspect ownership via `llman sdd spec resolve-req {req_id}`."
            ),
        });
    }
    issues
}

/// Next free short id: smallest unused `rN` (N ≥ 1) among `^r(\d+)$` ids.
pub fn next_req_id(root: &Path) -> Result<String> {
    let index = build_req_index(root)?;
    Ok(next_req_id_from_index(&index))
}

pub fn next_req_id_from_index(index: &BTreeMap<String, Vec<ReqLocation>>) -> String {
    let used: BTreeSet<u64> = index.keys().filter_map(|id| parse_r_number(id)).collect();
    let mut n = 1u64;
    while used.contains(&n) {
        n += 1;
    }
    format!("r{n}")
}

fn parse_r_number(id: &str) -> Option<u64> {
    let rest = id.strip_prefix('r')?;
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

/// True if `req_id` is already used by any main-library capability.
pub fn find_global_owner<'a>(
    index: &'a BTreeMap<String, Vec<ReqLocation>>,
    req_id: &str,
) -> Option<&'a ReqLocation> {
    index.get(req_id.trim()).and_then(|locs| locs.first())
}

pub fn resolve_req(root: &Path, req_id: &str, lang: &str) -> Result<ResolveReqJson> {
    let id = req_id.trim();
    let index = build_req_index(root)?;
    let locs = index.get(id).ok_or_else(|| {
        anyhow!("req_id `{id}` not found in llmanspec/specs (try `llman sdd spec next-req-id`)")
    })?;
    if locs.len() > 1 {
        let caps: Vec<_> = locs.iter().map(|l| l.capability.as_str()).collect();
        bail!(
            "req_id `{id}` is globally duplicated across: {}. Fix collisions before resolve.",
            caps.join(", ")
        );
    }
    let loc = &locs[0];
    let harness = harness_refs_for_req(root, &loc.capability, id, lang)?;
    Ok(ResolveReqJson {
        req_id: id.to_string(),
        capability: loc.capability.clone(),
        title: loc.title.clone(),
        statement: loc.statement.clone(),
        harness,
    })
}

fn harness_refs_for_req(
    root: &Path,
    capability: &str,
    req_id: &str,
    lang: &str,
) -> Result<Vec<HarnessRefJson>> {
    let spec_dir = specs_dir(root).join(capability);
    let mut out = Vec::new();
    for feature_path in discover_features(&spec_dir) {
        let feature_name = feature_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("feature")
            .to_string();
        let scenarios = match parse_feature_scenarios(&feature_path, lang) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for sc in scenarios {
            if sc.req_ids.iter().any(|r| r.trim() == req_id) {
                out.push(HarnessRefJson {
                    feature: feature_name.clone(),
                    scenario: sc.id.clone(),
                });
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Default)]
pub struct DedupeReport {
    pub remapped: Vec<(String, String, String)>, // capability, from, to
}

/// Keep lexicographically first capability for each colliding id; remap others
/// to fresh `rN` short ids. Updates `spec.toon` and `@req:` in `.feature` files.
pub fn dedupe_colliding_req_ids(root: &Path, dry_run: bool) -> Result<DedupeReport> {
    let mut index = build_req_index(root)?;
    let mut report = DedupeReport::default();
    let collisions: Vec<(String, Vec<String>)> = index
        .iter()
        .filter_map(|(id, locs)| {
            let caps: BTreeSet<String> = locs.iter().map(|l| l.capability.clone()).collect();
            if caps.len() < 2 {
                return None;
            }
            let mut ordered: Vec<String> = caps.into_iter().collect();
            ordered.sort();
            Some((id.clone(), ordered))
        })
        .collect();

    for (old_id, caps) in collisions {
        let keep = &caps[0];
        for capability in caps.iter().skip(1) {
            let new_id = next_req_id_from_index(&index);
            report
                .remapped
                .push((capability.clone(), old_id.clone(), new_id.clone()));
            if !dry_run {
                rewrite_capability_req_id(root, capability, &old_id, &new_id)?;
            }
            // Update in-memory index so subsequent allocations stay unique.
            if let Some(locs) = index.get_mut(&old_id) {
                locs.retain(|l| l.capability != *capability);
            }
            index.entry(new_id.clone()).or_default().push(ReqLocation {
                capability: capability.clone(),
                title: String::new(),
                statement: String::new(),
            });
        }
        let _ = keep;
    }
    Ok(report)
}

fn rewrite_capability_req_id(root: &Path, capability: &str, from: &str, to: &str) -> Result<()> {
    let path = spec_toon_path(root, capability);
    let content = fs::read_to_string(&path)?;
    let mut doc = BACKEND.parse_main_spec(&content, &format!("spec `{capability}`"))?;
    for req in &mut doc.requirements {
        if req.req_id.trim() == from {
            req.req_id = to.to_string();
        }
    }
    for sc in &mut doc.scenarios {
        if sc.req_id.trim() == from {
            sc.req_id = to.to_string();
        }
    }
    let payload = BACKEND.dump_main_spec(&doc)?;
    atomic_write_with_mode(&path, payload.as_bytes(), None)?;

    let spec_dir = specs_dir(root).join(capability);
    for feature_path in discover_features(&spec_dir) {
        let body = fs::read_to_string(&feature_path)?;
        let updated = replace_req_tags(&body, from, to);
        if updated != body {
            atomic_write_with_mode(&feature_path, updated.as_bytes(), None)?;
        }
    }
    Ok(())
}

/// Replace `@req:<from>` without matching longer ids (`r1` must not hit `r10`).
fn replace_req_tags(body: &str, from: &str, to: &str) -> String {
    let needle = format!("@req:{from}");
    let replacement = format!("@req:{to}");
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(idx) = rest.find(&needle) {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + needle.len()..];
        let boundary_ok = match after.chars().next() {
            None => true,
            Some(c) => !(c.is_ascii_alphanumeric() || c == '_' || c == '-'),
        };
        if boundary_ok {
            out.push_str(&replacement);
        } else {
            out.push_str(&needle);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

pub fn run_next_req_id(root: &Path, json: bool) -> Result<()> {
    let id = next_req_id(root)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&NextReqIdJson { req_id: id })?
        );
    } else {
        println!("{id}");
    }
    Ok(())
}

pub fn run_resolve_req(root: &Path, req_id: &str, json: bool, lang: &str) -> Result<()> {
    let resolved = resolve_req(root, req_id, lang)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&resolved)?);
    } else {
        println!("reqId: {}", resolved.req_id);
        println!("capability: {}", resolved.capability);
        println!("title: {}", resolved.title);
        println!("statement: {}", resolved.statement);
        if resolved.harness.is_empty() {
            println!("harness: (none)");
        } else {
            println!("harness:");
            for h in &resolved.harness {
                println!("  - {}:{}", h.feature, h.scenario);
            }
        }
    }
    Ok(())
}

pub fn run_dedupe_req_ids(root: &Path, dry_run: bool) -> Result<()> {
    let report = dedupe_colliding_req_ids(root, dry_run)?;
    if report.remapped.is_empty() {
        println!("No colliding req_id values in llmanspec/specs.");
        return Ok(());
    }
    for (cap, from, to) in &report.remapped {
        let prefix = if dry_run { "[dry-run] " } else { "" };
        println!("{prefix}{cap}: {from} → {to}");
    }
    println!(
        "{} remapping(s){}",
        report.remapped.len(),
        if dry_run { " (dry-run)" } else { "" }
    );
    Ok(())
}

/// Guard for authoring: fail if `req_id` already exists anywhere in main library.
pub fn ensure_req_id_globally_free(root: &Path, req_id: &str) -> Result<()> {
    let index = build_req_index(root)?;
    if let Some(owner) = find_global_owner(&index, req_id) {
        bail!(
            "req_id `{}` already used by capability `{}` (title: {}). \
             Use `llman sdd spec next-req-id` or `llman sdd spec resolve-req {}`.",
            req_id.trim(),
            owner.capability,
            owner.title,
            req_id.trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdd::spec::ir::{MainSpecDoc, RequirementEntry, ScenarioEntry};
    use tempfile::tempdir;

    fn write_spec(root: &Path, name: &str, reqs: &[(&str, &str)]) {
        let dir = root.join(LLMANSPEC_DIR_NAME).join("specs").join(name);
        fs::create_dir_all(&dir).unwrap();
        let requirements: Vec<RequirementEntry> = reqs
            .iter()
            .map(|(id, title)| RequirementEntry {
                req_id: id.to_string(),
                title: title.to_string(),
                statement: format!("MUST {title}"),
            })
            .collect();
        let scenarios: Vec<ScenarioEntry> = requirements
            .iter()
            .map(|r| ScenarioEntry {
                req_id: r.req_id.clone(),
                id: "baseline".into(),
                given: String::new(),
                when_: "trigger".into(),
                then_: "result".into(),
                feature: false,
            })
            .collect();
        let doc = MainSpecDoc {
            kind: "llman.sdd.spec".into(),
            name: name.into(),
            purpose: "test".into(),
            valid_scope: vec!["src/".into()],
            requirements,
            scenarios,
        };
        let payload = BACKEND.dump_main_spec(&doc).unwrap();
        fs::write(dir.join(SPEC_FILE), payload).unwrap();
        // minimal config
        let cfg = root.join(LLMANSPEC_DIR_NAME);
        if !cfg.join("config.yaml").exists() {
            fs::write(cfg.join("config.yaml"), "schema: spec-driven\nlocale: en\n").unwrap();
        }
    }

    #[test]
    fn next_req_id_fills_gaps() {
        let mut index = BTreeMap::new();
        index.insert(
            "r1".into(),
            vec![ReqLocation {
                capability: "a".into(),
                title: String::new(),
                statement: String::new(),
            }],
        );
        index.insert(
            "r3".into(),
            vec![ReqLocation {
                capability: "b".into(),
                title: String::new(),
                statement: String::new(),
            }],
        );
        assert_eq!(next_req_id_from_index(&index), "r2");
    }

    #[test]
    fn uniqueness_and_dedupe() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write_spec(root, "alpha", &[("r1", "one")]);
        write_spec(root, "beta", &[("r1", "two")]);
        let issues = global_req_id_uniqueness_issues(root);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("r1"));
        assert!(issues[0].message.contains("next-req-id"));

        let report = dedupe_colliding_req_ids(root, false).unwrap();
        assert_eq!(report.remapped.len(), 1);
        assert_eq!(report.remapped[0].0, "beta");
        assert_eq!(report.remapped[0].1, "r1");
        assert_eq!(report.remapped[0].2, "r2");
        assert!(global_req_id_uniqueness_issues(root).is_empty());
    }

    #[test]
    fn resolve_and_guard() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write_spec(root, "alpha", &[("r1", "one")]);
        let resolved = resolve_req(root, "r1", "en").unwrap();
        assert_eq!(resolved.capability, "alpha");
        assert!(ensure_req_id_globally_free(root, "r1").is_err());
        assert!(ensure_req_id_globally_free(root, "r2").is_ok());
    }

    #[test]
    fn req_tag_replace_respects_id_boundary() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write_spec(root, "beta", &[("r1", "two")]);
        let feature = root
            .join(LLMANSPEC_DIR_NAME)
            .join("specs")
            .join("beta")
            .join("beta.feature");
        fs::write(
            &feature,
            "# language: en\nFeature: Beta\n  @req:r1\n  Scenario: a\n    When x\n    Then y\n  @req:r10\n  Scenario: b\n    When x\n    Then y\n",
        )
        .unwrap();
        rewrite_capability_req_id(root, "beta", "r1", "r2").unwrap();
        let body = fs::read_to_string(&feature).unwrap();
        assert!(body.contains("@req:r2"));
        assert!(body.contains("@req:r10"));
        assert!(!body.contains("@req:r1\n"));
    }
}
