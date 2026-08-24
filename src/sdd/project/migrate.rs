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
        let doc: MainSpecDoc =
            parse_legacy_toon(&raw).with_context(|| format!("parse legacy `{capability}`"))?;
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
            Err(e) => failures.push(format!("{}: {e:#}", p.capability)),
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

    // Rules come from legacy toon requirements (statement verbatim).
    let merged = MainSpecDoc {
        kind: "llman.sdd.spec".to_string(),
        name: toon_doc.name.trim().to_string(),
        purpose: toon_doc.purpose.clone(),
        valid_scope: toon_doc.valid_scope.clone(),
        requirements: toon_doc.requirements.clone(),
        scenarios: Vec::new(),
    };

    // Acceptance scenarios are copied VERBATIM from every legacy feature:
    // the IR loses step boundaries within a type, and rstest-bdd binds by
    // exact step text, so byte-faithful blocks are the only safe carrier.
    let legacy_features = crate::sdd::spec::validation::discover_features(&plan.dir);
    let merged_files = legacy_features.len();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut acceptance_blocks: Vec<String> = Vec::new();
    let mut dropped = 0usize;
    for legacy in &legacy_features {
        let raw = fs::read_to_string(legacy)?;
        for (name, tags, block) in extract_scenario_blocks(&raw) {
            let executable = tags.iter().any(|t| {
                t.trim()
                    .trim_start_matches('@')
                    .eq_ignore_ascii_case("executable")
            });
            // Bare `@req:` (empty id) marks obsolete-contract scenarios — drop.
            let bare_req = tags.iter().any(|t| {
                t.trim()
                    .trim_start_matches('@')
                    .eq_ignore_ascii_case("req:")
            });
            let has_req = tags.iter().any(|t| {
                let t = t.trim().trim_start_matches('@');
                t.starts_with("req:") && t.len() > "req:".len()
            });
            if !executable || bare_req || !has_req {
                dropped = dropped.saturating_add(1);
                continue;
            }
            if seen_names.insert(name.clone()) {
                acceptance_blocks.push(block);
            } else {
                eprintln!(
                    "Warning: duplicate scenario `{name}` across merged features of `{}`; kept first",
                    plan.capability
                );
            }
        }
    }

    // Header + rules via the canonical renderer, then verbatim acceptance.
    let mut payload = FEATURE_BACKEND.dump_main_spec(&merged)?;
    if !payload.ends_with('\n') {
        payload.push('\n');
    }
    for block in &acceptance_blocks {
        payload.push_str(block);
        payload.push('\n');
    }
    atomic_write_with_mode(&feature_path, payload.as_bytes(), None)?;

    // Remove every legacy source file EXCEPT the freshly written target.
    // Compare canonically: glob paths may differ textually (`./` prefix).
    let target_canon = feature_path
        .canonicalize()
        .unwrap_or_else(|_| feature_path.clone());
    for legacy in &legacy_features {
        if legacy
            .canonicalize()
            .map(|c| c == target_canon)
            .unwrap_or(false)
        {
            continue;
        }
        fs::remove_file(legacy).with_context(|| format!("remove merged {}", legacy.display()))?;
    }
    fs::remove_file(plan.dir.join(SPEC_FILE))
        .with_context(|| format!("remove legacy {}", plan.dir.join(SPEC_FILE).display()))?;

    // r136: the report MUST carry the three-tier initial values.
    let tiers = FEATURE_BACKEND
        .parse_content(&payload, &format!("migrated `{}`", plan.capability))
        .map(|p| crate::sdd::spec::backend::feature_backend::compute_rule_morphology(&p));
    match tiers {
        Ok(t) => Ok(format!(
            "wrote {} (merged {merged_files} feature file(s); rules {} enforced {} manual {} pending {}; acceptance {}; dropped {dropped}); removed legacy spec.toon",
            feature_path.display(),
            t.rule_count,
            t.rule_enforced_count,
            t.rule_manual_count,
            t.rule_pending_count,
            t.acceptance_count,
        )),
        Err(e) => Ok(format!(
            "wrote {} (merged {merged_files} feature file(s); rules {}, acceptance {}, dropped {dropped}); removed legacy spec.toon; tier report unavailable: {e}",
            feature_path.display(),
            merged.requirements.len(),
            acceptance_blocks.len(),
        )),
    }
}

/// Extract top-level scenario blocks (`tags + 场景/Scenario + body`) from a
/// legacy feature file, verbatim. Returns `(scenario name, tags, block)` with
/// the block starting at its first tag line.
fn extract_scenario_blocks(content: &str) -> Vec<(String, Vec<String>, String)> {
    use std::fmt::Write as _;
    let mut out: Vec<(String, Vec<String>, String)> = Vec::new();
    let mut current_tags: Vec<String> = Vec::new();
    let mut current: Option<(String, Vec<String>, String)> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        let is_tag = trimmed.starts_with('@');
        let is_scenario = trimmed.starts_with("场景:") || trimmed.starts_with("Scenario:");
        if is_tag {
            if let Some(done) = current.take() {
                out.push(done);
            }
            current_tags.push(trimmed.to_string());
            continue;
        }
        if is_scenario {
            if let Some(done) = current.take() {
                out.push(done);
            }
            let name = trimmed
                .split_once(':')
                .map(|(_, rest)| rest.trim().to_string())
                .unwrap_or_default();
            let mut block = String::new();
            for t in &current_tags {
                let _ = writeln!(block, "  {t}");
            }
            let _ = writeln!(block, "  {trimmed}");
            current = Some((name, current_tags.clone(), block));
            current_tags.clear();
            continue;
        }
        if let Some((_, _, block)) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        } else if !current_tags.is_empty() && !trimmed.is_empty() {
            // Tags followed by a non-scenario line: orphaned tag run — reset.
            current_tags.clear();
        }
    }
    if let Some(done) = current.take() {
        out.push(done);
    }
    out
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

/// Minimal reader for the LEGACY strict-TOON spec shape (migration-only).
/// Handles exactly the keys the old writer emitted; anything else fails loudly.
fn parse_legacy_toon(content: &str) -> Result<MainSpecDoc> {
    fn split_row(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    if in_quotes && chars.peek() == Some(&'"') {
                        cur.push('"');
                        chars.next();
                    } else {
                        in_quotes = !in_quotes;
                    }
                }
                ',' if !in_quotes => out.push(std::mem::take(&mut cur)),
                _ => cur.push(c),
            }
        }
        out.push(cur);
        out
    }

    fn unquote(v: String) -> String {
        let t = v.trim();
        if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
            t[1..t.len() - 1].to_string()
        } else {
            t.to_string()
        }
    }

    let mut kind = String::new();
    let mut name = String::new();
    let mut purpose = String::new();
    let mut valid_scope: Vec<String> = Vec::new();
    let mut requirements: Vec<crate::sdd::spec::ir::RequirementEntry> = Vec::new();
    let mut scenarios: Vec<crate::sdd::spec::ir::ScenarioEntry> = Vec::new();
    let mut section: Option<&'static str> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, rest)) = trimmed.split_once(':') {
            match key.trim() {
                "kind" => kind = unquote(rest.to_string()),
                "name" => name = unquote(rest.to_string()),
                "purpose" => purpose = unquote(rest.to_string()),
                k if k.starts_with("valid_scope") => {
                    valid_scope = rest
                        .split(',')
                        .map(|v| unquote(v.to_string()).trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect();
                    section = None;
                    continue;
                }
                k if k.starts_with("requirements") => {
                    section = Some("requirements");
                    continue;
                }
                k if k.starts_with("scenarios") => {
                    section = Some("scenarios");
                    continue;
                }
                _ => {}
            }
            // Tabular row under the active section.
            if section == Some("requirements") {
                let cells = split_row(trimmed);
                if cells.len() >= 3 {
                    requirements.push(crate::sdd::spec::ir::RequirementEntry {
                        req_id: unquote(cells[0].clone()).trim().to_string(),
                        title: unquote(cells[1].clone()).trim().to_string(),
                        statement: unquote(cells[2..].join(",")),
                    });
                }
                continue;
            }
            if section == Some("scenarios") {
                let cells = split_row(trimmed);
                if cells.len() >= 6 {
                    scenarios.push(crate::sdd::spec::ir::ScenarioEntry {
                        req_id: unquote(cells[0].clone()).trim().to_string(),
                        id: unquote(cells[1].clone()).trim().to_string(),
                        given: unquote(cells[2].clone()),
                        when_: unquote(cells[3].clone()),
                        then_: unquote(cells[4].clone()),
                        feature: unquote(cells[5].clone()).trim() != "false",
                    });
                }
                continue;
            }
        }
        // Rows are indented under their section header.
        if line.starts_with(char::is_whitespace) {
            if section == Some("requirements") {
                let cells = split_row(trimmed);
                if cells.len() >= 3 {
                    requirements.push(crate::sdd::spec::ir::RequirementEntry {
                        req_id: unquote(cells[0].clone()).trim().to_string(),
                        title: unquote(cells[1].clone()).trim().to_string(),
                        statement: unquote(cells[2..].join(",")),
                    });
                }
                continue;
            }
            if section == Some("scenarios") {
                let cells = split_row(trimmed);
                if cells.len() >= 6 {
                    scenarios.push(crate::sdd::spec::ir::ScenarioEntry {
                        req_id: unquote(cells[0].clone()).trim().to_string(),
                        id: unquote(cells[1].clone()).trim().to_string(),
                        given: unquote(cells[2].clone()),
                        when_: unquote(cells[3].clone()),
                        then_: unquote(cells[4].clone()),
                        feature: unquote(cells[5].clone()).trim() != "false",
                    });
                }
                continue;
            }
        }
    }

    if kind.trim() != "llman.sdd.spec" {
        anyhow::bail!("legacy spec kind must be `llman.sdd.spec`, got `{kind}`");
    }
    Ok(MainSpecDoc {
        kind,
        name,
        purpose,
        valid_scope,
        requirements,
        scenarios,
    })
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
