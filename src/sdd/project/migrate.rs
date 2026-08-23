//! One-shot, idempotent conversion: legacy Partitioned pair (`spec.toon` +
//! optional `*.feature`) → single-track `<capability>.feature` (r136).
//!
//! - `requirements[]` become `@req:<id> @human` rule scenarios with the
//!   statement preserved verbatim as the scenario description (lossless).
//! - `feature: false` note rows are dropped (they were pointer noise).
//! - Existing `.feature` acceptance scenarios (`@executable`) are carried over.
//! - `spec.toon` is deleted after a successful write; re-running is a no-op.

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::spec::backend::FEATURE_BACKEND;
use crate::sdd::spec::backend::SpecBackend;
use crate::sdd::spec::backend::toon_backend;
use crate::sdd::spec::ir::MainSpecDoc;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MigrateArgs {
    pub dry_run: bool,
    #[allow(dead_code)]
    pub force: bool,
    /// Skip the confirmation prompt and apply (for agents/scripts).
    pub yes: bool,
    /// Treat the terminal as non-interactive even when stdin is a TTY.
    pub no_interactive: bool,
}

/// One capability's migration plan.
struct Plan {
    dir: PathBuf,
    capability: String,
    toon: Option<MainSpecDoc>,
}

pub fn run(args: MigrateArgs) -> Result<()> {
    run_at(Path::new("."), args)
}

pub fn run_at(root: &Path, args: MigrateArgs) -> Result<()> {
    let specs_root = root.join(LLMANSPEC_DIR_NAME).join("specs");
    if !specs_root.is_dir() {
        println!("No specs directory found; nothing to migrate.");
        return Ok(());
    }

    // Phase 1: scan legacy dirs.
    let mut plans: Vec<Plan> = Vec::new();
    for dir in collect_capability_dirs(&specs_root)? {
        let capability = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if !dir.join(SPEC_FILE).exists() {
            continue; // already single-track → idempotent no-op
        }
        let raw = fs::read_to_string(dir.join(SPEC_FILE))
            .with_context(|| format!("read {}", dir.join(SPEC_FILE).display()))?;
        let doc: MainSpecDoc = toon_backend::BACKEND
            .parse_main_spec(&raw, &format!("legacy spec `{capability}`"))
            .with_context(|| format!("parse legacy `{capability}`"))?;
        plans.push(Plan {
            dir,
            capability,
            toon: Some(doc),
        });
    }

    if plans.is_empty() {
        println!("Nothing to migrate; all capabilities are already single-track.");
        return Ok(());
    }
    println!("Legacy capabilities to migrate: {}", plans.len());

    if args.dry_run {
        for p in &plans {
            let acceptance_note = if p.dir.join(format!("{}.feature", p.capability)).exists() {
                "keep existing"
            } else {
                "none"
            };
            println!(
                "  {} → {}.feature (rules {}, acceptance {})",
                p.dir.display(),
                p.capability,
                p.toon.as_ref().map(|d| d.requirements.len()).unwrap_or(0),
                acceptance_note,
            );
        }
        println!("\n(dry-run: no files written)");
        return Ok(());
    }

    if !args.yes {
        let interactive = crate::sdd::shared::interactive::is_interactive(args.no_interactive);
        if !interactive {
            return Err(anyhow!(
                "non-interactive terminal: re-run with --yes to apply, or --dry-run to preview"
            ));
        }
        let confirmed = inquire::Confirm::new(&format!(
            "Migrate {} capability(s) to the single-track format?",
            plans.len()
        ))
        .with_default(false)
        .prompt()
        .map_err(|e| anyhow!("confirmation prompt failed: {e}"))?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Phase 2: apply.
    let mut migrated = 0usize;
    let mut failures = Vec::new();
    for p in &plans {
        match migrate_capability(p) {
            Ok(summary) => {
                println!("  ✔ {}: {summary}", p.capability);
                migrated += 1;
            }
            Err(e) => failures.push(format!("{}: {e}", p.capability)),
        }
    }

    if !failures.is_empty() {
        eprintln!("Failures ({}):", failures.len());
        for f in &failures {
            eprintln!("  - {f}");
        }
        return Err(anyhow!(
            "migration completed with {} failure(s)",
            failures.len()
        ));
    }

    // Coverage summary over the migrated tree.
    println!("\nMigration complete: {migrated} capability(s) converted to single-track features.");
    Ok(())
}

fn migrate_capability(plan: &Plan) -> Result<String> {
    let Some(toon_doc) = &plan.toon else {
        return Ok("skipped".to_string());
    };
    let feature_path = plan.dir.join(format!("{}.feature", plan.capability));

    // Merge: toon requirements + any rules already authored in the feature;
    // executable scenarios come from the existing feature untouched.
    let mut merged = MainSpecDoc {
        kind: "llman.sdd.spec".to_string(),
        name: toon_doc.name.trim().to_string(),
        purpose: toon_doc.purpose.clone(),
        valid_scope: toon_doc.valid_scope.clone(),
        requirements: toon_doc.requirements.clone(),
        scenarios: Vec::new(),
    };

    if feature_path.exists() {
        let raw = fs::read_to_string(&feature_path)?;
        let parsed = FEATURE_BACKEND
            .parse_content(&raw, &format!("existing `{}`", plan.capability))
            .context("merge existing .feature")?;
        let existing =
            FEATURE_BACKEND.parse_main_spec(&raw, &format!("existing `{}`", plan.capability))?;
        let known: std::collections::HashSet<String> = merged
            .requirements
            .iter()
            .map(|r| r.req_id.trim().to_string())
            .collect();
        for req in &existing.requirements {
            if !known.contains(req.req_id.trim()) {
                merged.requirements.push(req.clone());
            }
        }
        merged.scenarios.extend(existing.scenarios);
        let _ = parsed;
    }

    let payload = FEATURE_BACKEND.dump_main_spec(&merged)?;
    atomic_write_with_mode(&feature_path, payload.as_bytes(), None)?;
    fs::remove_file(plan.dir.join(SPEC_FILE))
        .with_context(|| format!("remove legacy {}", plan.dir.join(SPEC_FILE).display()))?;

    // Tier summary for the migration report (r136: three-tier initial values).
    let reparsed =
        FEATURE_BACKEND.parse_main_spec(&payload, &format!("migrated `{}`", plan.capability))?;
    let rules = merged.requirements.len();
    let enforced = reparsed
        .scenarios
        .iter()
        .filter(|sc| sc.feature)
        .filter(|sc| {
            merged
                .requirements
                .iter()
                .any(|r| r.req_id == sc.req_id || sc.req_id.is_empty())
        })
        .count()
        .min(rules);
    Ok(format!(
        "wrote {} (rules {rules}, enforced ≥{enforced}, acceptance {}); removed legacy spec.toon",
        feature_path.display(),
        reparsed.scenarios.iter().filter(|s| s.feature).count(),
    ))
}

fn collect_capability_dirs(specs_root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let entries =
        fs::read_dir(specs_root).with_context(|| format!("read {}", specs_root.display()))?;
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            out.push(entry.path());
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY_TOON: &str = concat!(
        "kind: llman.sdd.spec\n",
        "name: \"demo\"\n",
        "purpose: \"demo purpose\"\n",
        "valid_scope[1]: \"src/\"\n",
        "requirements[2]{req_id,title,statement}:\n",
        "  r1,First,\"System MUST do X.\"\n",
        "  r2,Second,\"System MUST do Y.\"\n",
        "scenarios[1]{req_id,id,given,when,then,feature}:\n",
        "  r1,note,\"\",\"a trigger\",\"an outcome\",false\n",
    );

    #[test]
    fn toon2features_is_lossless_idempotent_and_cleans_toon() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let dir = root.join(LLMANSPEC_DIR_NAME).join("specs").join("demo");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(SPEC_FILE), LEGACY_TOON).unwrap();

        let args = MigrateArgs {
            dry_run: false,
            force: false,
            yes: true,
            no_interactive: true,
        };
        run_at(root, args.clone()).unwrap();

        assert!(!dir.join(SPEC_FILE).exists(), "legacy toon must be removed");
        let feature = fs::read_to_string(dir.join("demo.feature")).unwrap();
        assert!(feature.contains("# capability: demo"));
        assert!(feature.contains("@req:r1 @human"));
        assert!(feature.contains("System MUST do X."));
        // Note row (feature:false) is dropped.
        assert!(!feature.contains("note"));

        // Idempotent: second run is a no-op.
        run_at(root, args).unwrap();

        // Migrated output parses and validates under the new gates.
        use crate::sdd::spec::backend::SpecBackend;
        let doc = FEATURE_BACKEND
            .parse_main_spec(&feature, "migrated")
            .unwrap();
        assert_eq!(doc.requirements.len(), 2);
        assert!(
            doc.requirements
                .iter()
                .all(|r| r.statement.contains("MUST"))
        );
    }
}
