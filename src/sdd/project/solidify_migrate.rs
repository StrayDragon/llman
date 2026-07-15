//! One-shot migration tool: upgrade BDD-on `spec.toon` files from the legacy
//! minimal form (`kind`/`name`/`purpose` only, behavior in sibling `.feature`
//! files) to the unified full structure (`valid_scope` + `requirements` +
//! `scenarios`, with all migrated scenarios `feature=true`).
//!
//! See `llmanspec/changes/add-bdd-solidify-workflow/design.md` §7.
//!
//! For each spec under `llmanspec/specs/<id>/`:
//! - parse every `.feature` file with the `gherkin` crate
//! - reverse each Gherkin scenario into a TOON `ScenarioEntry` (feature=true)
//! - synthesize one `RequirementEntry` (`r1`) when the spec has none, using the
//!   spec `purpose` (annotated with MUST if needed) so the unified validation
//!   passes
//! - set `valid_scope` to `llmanspec/specs/<id>`
//! - keep the original `.feature` files in place (solidify regenerates them)

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::solidify::locale_to_gherkin_lang;
use crate::sdd::spec::backend::{BACKEND, SpecBackend};
use crate::sdd::spec::ir::{MainSpecDoc, RequirementEntry, ScenarioEntry};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// A spec slated for migration.
struct Candidate {
    id: String,
    spec_path: PathBuf,
    doc: MainSpecDoc,
    feature_files: Vec<PathBuf>,
}

/// Whether a spec is a migration candidate: it has a `spec.toon` whose
/// `requirements` is empty AND the directory contains at least one `.feature`
/// file. Specs that already declare requirements are left untouched.
fn is_candidate(doc: &MainSpecDoc, feature_files: &[PathBuf]) -> bool {
    doc.requirements.is_empty() && !feature_files.is_empty()
}

/// Discover `*.feature` files in a spec directory, sorted (mirrors
/// `spec::validation::discover_features`).
fn discover_features(spec_dir: &Path) -> Vec<PathBuf> {
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

/// Reverse-parse a single Gherkin scenario into a TOON ScenarioEntry. Steps of
/// each type (Given/When/Then) are joined with newlines; `feature` is set to
/// `true` (migrated scenarios are executable).
fn scenario_from_gherkin(sc: &gherkin::Scenario, req_id: &str) -> ScenarioEntry {
    let mut given = Vec::new();
    let mut when_ = Vec::new();
    let mut then_ = Vec::new();
    for step in &sc.steps {
        match step.ty {
            gherkin::StepType::Given => given.push(step.value.clone()),
            gherkin::StepType::When => when_.push(step.value.clone()),
            gherkin::StepType::Then => then_.push(step.value.clone()),
        }
    }
    ScenarioEntry {
        req_id: req_id.to_string(),
        id: sc.name.clone(),
        given: given.join("\n"),
        when_: when_.join("\n"),
        then_: then_.join("\n"),
        feature: true,
    }
}

/// Ensure a requirement statement contains SHALL or MUST (validation requires
/// it). Appends a generic MUST clause when neither keyword is present.
fn ensure_shall_or_must(statement: &str) -> String {
    let s = statement.trim();
    if s.to_uppercase().contains("MUST") || s.to_uppercase().contains("SHALL") {
        s.to_string()
    } else {
        format!("{s} The system MUST satisfy the scenarios in this spec.")
    }
}

/// Build the migrated `MainSpecDoc` for one candidate: synthesize requirement
/// `r1` (if needed), reverse all feature scenarios, set valid_scope.
fn build_migrated_doc(candidate: &Candidate, lang: &str) -> Result<MainSpecDoc> {
    // All migrated scenarios attach to a single synthesized requirement r1.
    // (Existing BDD-on specs carry no req_id information in .feature files.)
    let req_id = "r1";
    let mut scenarios = Vec::new();
    for feature_path in &candidate.feature_files {
        let content = fs::read_to_string(feature_path)
            .with_context(|| format!("read feature {}", feature_path.display()))?;
        // GherkinEnv is not Clone; rebuild per feature.
        let env = gherkin::GherkinEnv::new(lang)
            .with_context(|| format!("build gherkin env for language `{lang}`"))?;
        let parsed = gherkin::Feature::parse(&content, env)
            .with_context(|| format!("parse feature {}", feature_path.display()))?;
        for sc in &parsed.scenarios {
            scenarios.push(scenario_from_gherkin(sc, req_id));
        }
    }

    let mut doc = candidate.doc.clone();
    doc.valid_scope = vec![format!("{LLMANSPEC_DIR_NAME}/specs/{}", candidate.id)];
    if doc.requirements.is_empty() {
        doc.requirements = vec![RequirementEntry {
            req_id: req_id.to_string(),
            title: candidate.id.clone(),
            statement: ensure_shall_or_must(&candidate.doc.purpose),
        }];
    }
    doc.scenarios = scenarios;
    Ok(doc)
}

/// Scan `llmanspec/specs/*/` and return migration candidates.
fn collect_candidates(root: &Path) -> Result<Vec<Candidate>> {
    let specs_dir = root.join(LLMANSPEC_DIR_NAME).join("specs");
    let mut candidates = Vec::new();
    if !specs_dir.exists() {
        return Ok(candidates);
    }
    for entry in fs::read_dir(&specs_dir)
        .with_context(|| format!("read specs dir {}", specs_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        let spec_dir = entry.path();
        let spec_path = spec_dir.join(SPEC_FILE);
        if !spec_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&spec_path)
            .with_context(|| format!("read spec {}", spec_path.display()))?;
        let doc = BACKEND
            .parse_main_spec(&content, &format!("spec `{id}`"))
            .with_context(|| format!("parse spec {}", spec_path.display()))?;
        let feature_files = discover_features(&spec_dir);
        if is_candidate(&doc, &feature_files) {
            candidates.push(Candidate {
                id,
                spec_path,
                doc,
                feature_files,
            });
        }
    }
    Ok(candidates)
}

/// CLI entry: `llman sdd project solidify-migrate [--dry-run]`.
pub fn run(dry_run: bool) -> Result<()> {
    let root = Path::new(".");
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    let locale = config.locale.as_str();
    let lang = locale_to_gherkin_lang(Some(locale), config.bdd.as_ref());

    println!("{}", t!("sdd.solidify.migrate_start"));

    let candidates = collect_candidates(root)?;
    if candidates.is_empty() {
        println!("{}", t!("sdd.solidify.migrate_no_candidates"));
        return Ok(());
    }

    let mut migrated = 0usize;
    for candidate in &candidates {
        let doc = build_migrated_doc(candidate, &lang)?;
        if dry_run {
            println!(
                "  {}",
                t!(
                    "sdd.solidify.migrate_spec_done",
                    spec = candidate.id,
                    reqs = doc.requirements.len(),
                    scenarios = doc.scenarios.len(),
                )
            );
        } else {
            let payload = BACKEND.dump_main_spec(&doc)?;
            atomic_write_with_mode(&candidate.spec_path, payload.as_bytes(), None)?;
            println!(
                "  {}",
                t!(
                    "sdd.solidify.migrate_spec_done",
                    spec = candidate.id,
                    reqs = doc.requirements.len(),
                    scenarios = doc.scenarios.len(),
                )
            );
        }
        migrated += 1;
    }

    if dry_run {
        println!(
            "{}",
            t!("sdd.solidify.migrate_dry_run_summary", count = migrated)
        );
    } else {
        println!("{}", t!("sdd.solidify.migrate_summary", count = migrated));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_shall_or_must_passes_through_when_present() {
        assert_eq!(ensure_shall_or_must("System MUST do x"), "System MUST do x");
        assert_eq!(
            ensure_shall_or_must("System SHALL do y"),
            "System SHALL do y"
        );
    }

    #[test]
    fn ensure_shall_or_must_appends_when_absent() {
        let result = ensure_shall_or_must("describes behavior");
        assert!(result.contains("MUST"));
        assert!(result.starts_with("describes behavior"));
    }

    #[test]
    fn is_candidate_requires_empty_reqs_and_features() {
        let doc = MainSpecDoc {
            kind: "llman.sdd.spec".to_string(),
            name: "x".to_string(),
            purpose: "p".to_string(),
            valid_scope: Vec::new(),
            requirements: Vec::new(),
            scenarios: Vec::new(),
        };
        let feature = PathBuf::from("/tmp/x.feature");
        // Empty reqs + features present → candidate.
        assert!(is_candidate(&doc, std::slice::from_ref(&feature)));
        // No features → not a candidate.
        assert!(!is_candidate(&doc, &[]));
    }

    #[test]
    fn is_candidate_false_when_requirements_present() {
        let mut doc = MainSpecDoc {
            kind: "llman.sdd.spec".to_string(),
            name: "x".to_string(),
            purpose: "p".to_string(),
            valid_scope: Vec::new(),
            requirements: Vec::new(),
            scenarios: Vec::new(),
        };
        doc.requirements = vec![RequirementEntry {
            req_id: "r1".to_string(),
            title: "T".to_string(),
            statement: "MUST x".to_string(),
        }];
        let feature = PathBuf::from("/tmp/x.feature");
        // Already has requirements → not a candidate (leave untouched).
        assert!(!is_candidate(&doc, &[feature]));
    }
}
