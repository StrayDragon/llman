//! `llman sdd project partition-migrate` — Partitioned SSOT migration.
//!
//! Moves executable GWT out of `spec.toon` into `.feature` (with `@req`), leaving
//! only constraints + non-executable scenarios in toon.
//!
//! Existing `.feature` files are preserved (Background / And / multi-file). We only:
//! - insert `@req:` tags before matching `场景:` / `Scenario:` lines
//! - append missing scenarios from stripped toon rows

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::spec::backend::{BACKEND, SpecBackend};
use crate::sdd::spec::ir::ScenarioEntry;
use crate::sdd::spec::partitioned::split_executable_from_toon;
use crate::sdd::spec::validation::discover_features;
use crate::sdd::spec::validation::locale_to_gherkin_lang;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(dry_run: bool) -> Result<()> {
    let root = Path::new(".");
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_dir.exists() {
        anyhow::bail!("no llmanspec directory at {}", llmanspec_dir.display());
    }
    let config = load_required_config(&llmanspec_dir)?;
    if config.bdd.is_none() {
        println!("{}", t!("sdd.solidify.partition_migrate_bdd_off"));
        return Ok(());
    }
    let lang = locale_to_gherkin_lang(Some(&config.locale), config.bdd.as_ref());
    let specs_dir = llmanspec_dir.join("specs");
    if !specs_dir.exists() {
        println!("{}", t!("sdd.solidify.partition_migrate_none"));
        return Ok(());
    }

    println!("{}", t!("sdd.solidify.partition_migrate_start"));
    let mut migrated = 0usize;
    let mut entries: Vec<PathBuf> = fs::read_dir(&specs_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();

    for spec_dir in entries {
        let name = spec_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let toon_path = spec_dir.join(SPEC_FILE);
        if !toon_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&toon_path)
            .with_context(|| format!("read {}", toon_path.display()))?;
        let mut doc = BACKEND
            .parse_main_spec(&content, &format!("spec `{name}`"))
            .with_context(|| format!("parse {}", toon_path.display()))?;

        let removed = split_executable_from_toon(&mut doc);
        let features = discover_features(&spec_dir);
        // Also drop non-executable toon rows whose ids already exist in harness
        // (Partitioned: harness owns those scenario ids).
        let harness_ids = {
            let mut ids = HashSet::new();
            for path in &features {
                if let Ok(body) = fs::read_to_string(path) {
                    ids.extend(scenario_ids_in_feature(&body));
                }
            }
            ids
        };
        let before = doc.scenarios.len();
        doc.scenarios
            .retain(|s| s.feature || !harness_ids.contains(&s.id));
        let dropped_nonexec = before - doc.scenarios.len();

        let feature_path = spec_dir.join(format!("{name}.feature"));
        let plan = plan_feature_update(&name, &removed, &feature_path, &lang)?;

        let mut other_tag_changes = false;
        if !removed.is_empty() {
            for path in &features {
                if path == &feature_path {
                    continue;
                }
                if let Ok(body) = fs::read_to_string(path) {
                    let (_, changed) = insert_req_tags(&body, &removed);
                    if changed {
                        other_tag_changes = true;
                        break;
                    }
                }
            }
        }

        // Only count / write when there is real Partitioned work left.
        let needs_work =
            !removed.is_empty() || dropped_nonexec > 0 || plan.changed || other_tag_changes;
        if !needs_work {
            continue;
        }

        if dry_run {
            println!(
                "  [dry-run] {name}: strip {} executable; drop_nonexec={}; feature touches={}; other_tags={}",
                removed.len(),
                dropped_nonexec,
                plan.changed,
                other_tag_changes
            );
            migrated += 1;
            continue;
        }

        let dumped = BACKEND.dump_main_spec(&doc)?;
        atomic_write_with_mode(&toon_path, dumped.as_bytes(), None)?;
        if plan.changed {
            if let Some(parent) = feature_path.parent() {
                fs::create_dir_all(parent)?;
            }
            atomic_write_with_mode(&feature_path, plan.body.as_bytes(), None)?;
        }

        // Tag other feature files in the directory (preserve body, insert @req).
        for path in features {
            if path == feature_path {
                continue;
            }
            let body = fs::read_to_string(&path)?;
            let (updated, changed) = insert_req_tags(&body, &removed);
            if changed {
                atomic_write_with_mode(&path, updated.as_bytes(), None)?;
            }
        }

        println!(
            "  migrated {name}: stripped {}, feature_changed={}",
            removed.len(),
            plan.changed
        );
        migrated += 1;
    }

    if migrated == 0 {
        println!("{}", t!("sdd.solidify.partition_migrate_already_done"));
    } else if dry_run {
        println!(
            "{}",
            t!("sdd.solidify.partition_migrate_dry_run", count = migrated)
        );
    } else {
        println!(
            "{}",
            t!("sdd.solidify.partition_migrate_summary", count = migrated)
        );
    }
    Ok(())
}

struct FeaturePlan {
    body: String,
    changed: bool,
}

fn plan_feature_update(
    name: &str,
    removed: &[ScenarioEntry],
    feature_path: &Path,
    lang: &str,
) -> Result<FeaturePlan> {
    if feature_path.exists() {
        let body = fs::read_to_string(feature_path)?;
        let (mut updated, mut changed) = insert_req_tags(&body, removed);
        let existing_ids = scenario_ids_in_feature(&updated);
        let mut append = String::new();
        for sc in removed {
            if existing_ids.contains(&sc.id) {
                continue;
            }
            append.push('\n');
            append.push_str(&render_appended_scenario(sc, lang));
            changed = true;
        }
        updated.push_str(&append);
        Ok(FeaturePlan {
            body: updated,
            changed,
        })
    } else if removed.is_empty() {
        Ok(FeaturePlan {
            body: String::new(),
            changed: false,
        })
    } else {
        // Create a new primary feature from stripped rows only.
        let mut body = format!("# language: {lang}\n# managed by llman sdd partition-migrate\n");
        let feat = if lang.starts_with("zh") {
            "功能"
        } else {
            "Feature"
        };
        body.push_str(&format!("{feat}: {name}\n"));
        for sc in removed {
            body.push('\n');
            body.push_str(&render_appended_scenario(sc, lang));
        }
        Ok(FeaturePlan {
            body,
            changed: true,
        })
    }
}

fn scenario_ids_in_feature(body: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
    for line in body.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("场景:") {
            ids.insert(rest.trim().to_string());
        } else if let Some(rest) = t.strip_prefix("Scenario:") {
            ids.insert(rest.trim().to_string());
        }
    }
    ids
}

/// Insert `@req:<id>` immediately above matching scenario titles when missing.
fn insert_req_tags(body: &str, removed: &[ScenarioEntry]) -> (String, bool) {
    let req_by_id: HashMap<&str, &str> = removed
        .iter()
        .map(|s| (s.id.as_str(), s.req_id.as_str()))
        .collect();
    if req_by_id.is_empty() {
        return (body.to_string(), false);
    }

    let lines: Vec<&str> = body.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut changed = false;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let scenario_name = trimmed
            .strip_prefix("场景:")
            .or_else(|| trimmed.strip_prefix("Scenario:"))
            .map(str::trim);
        if let Some(name) = scenario_name
            && let Some(rid) = req_by_id.get(name).copied().filter(|s| !s.is_empty())
        {
            let already = out
                .last()
                .map(|l| {
                    let t = l.trim();
                    t == format!("@req:{rid}") || t.starts_with("@req:")
                })
                .unwrap_or(false);
            if !already {
                // Preserve indentation of the scenario line.
                let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                out.push(format!("{indent}@req:{rid}"));
                changed = true;
            }
        }
        out.push(line.to_string());
        i += 1;
    }
    let mut joined = out.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    (joined, changed)
}

fn render_appended_scenario(sc: &ScenarioEntry, lang: &str) -> String {
    let (scenario, given, when_, then_) = if lang.starts_with("zh") {
        ("场景", "假如", "当", "那么")
    } else {
        ("Scenario", "Given", "When", "Then")
    };
    let mut out = String::new();
    if !sc.req_id.trim().is_empty() {
        out.push_str(&format!("  @req:{}\n", sc.req_id.trim()));
    }
    out.push_str(&format!("  {scenario}: {}\n", sc.id));
    if !sc.given.trim().is_empty() {
        out.push_str(&format!("    {given} {}\n", sc.given.trim()));
    }
    if !sc.when_.trim().is_empty() {
        out.push_str(&format!("    {when_} {}\n", sc.when_.trim()));
    }
    if !sc.then_.trim().is_empty() {
        out.push_str(&format!("    {then_} {}\n", sc.then_.trim()));
    }
    out
}
