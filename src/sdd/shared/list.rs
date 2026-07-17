use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::shared::discovery::{list_changes, list_specs};
use crate::sdd::shared::tasks;
use crate::sdd::spec::parser::parse_spec;
use crate::sdd::spec::validation::{ChangeStage, determine_stage};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::cmp::{Reverse, max};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ListArgs {
    pub specs: bool,
    pub changes: bool,
    pub sort: String,
    pub json: bool,
    pub compact_json: bool,
    pub no_interactive: bool,
}

#[derive(Debug, Serialize)]
struct ChangeJson {
    name: String,
    stage: String,
    #[serde(rename = "completedTasks")]
    completed_tasks: usize,
    #[serde(rename = "totalTasks")]
    total_tasks: usize,
    #[serde(rename = "lastModified")]
    last_modified: String,
    status: String,
}

#[derive(Debug)]
struct ChangeInfo {
    name: String,
    stage: ChangeStage,
    completed_tasks: usize,
    total_tasks: usize,
    last_modified: DateTime<Utc>,
}

pub fn run(args: ListArgs) -> Result<()> {
    if args.specs && args.changes {
        return Err(anyhow!(t!("sdd.list.conflicting_flags")));
    }
    let root = Path::new(".");
    let _changes_requested = args.changes; // Explicit --changes mirrors the default behavior.
    let mode = if args.specs { "specs" } else { "changes" };
    if mode == "changes" {
        list_changes_mode(root, &args)
    } else {
        list_specs_mode(root, &args)
    }
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_conflicting_flags() {
        let result = run(ListArgs {
            specs: true,
            changes: true,
            sort: "recent".to_string(),
            json: false,
            compact_json: false,
            no_interactive: false,
        });
        assert!(result.is_err());
    }
}

fn list_changes_mode(root: &Path, args: &ListArgs) -> Result<()> {
    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");
    if !changes_dir.exists() {
        return Err(anyhow!(t!("sdd.list.no_changes_dir")));
    }

    let change_ids = list_changes(root)?;
    if change_ids.is_empty() {
        if args.json {
            print_json(&serde_json::json!({"changes": []}), args.compact_json)?;
        } else {
            println!("{}", t!("sdd.list.no_active_changes"));
        }
        return Ok(());
    }

    let mut changes = Vec::new();
    for change in change_ids {
        let (completed, total) = task_progress(&changes_dir, &change)?;
        let last_modified = last_modified(&changes_dir.join(&change))?;
        let stage = determine_stage(&changes_dir.join(&change));
        changes.push(ChangeInfo {
            name: change,
            stage,
            completed_tasks: completed,
            total_tasks: total,
            last_modified,
        });
    }

    let sort_by_name = args.sort == "name";
    if sort_by_name {
        changes.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    } else {
        changes.sort_by_key(|change| Reverse(change.last_modified));
    }

    if args.json {
        let json_output: Vec<ChangeJson> = changes
            .iter()
            .map(|c| ChangeJson {
                name: c.name.clone(),
                stage: c.stage.as_str().to_string(),
                completed_tasks: c.completed_tasks,
                total_tasks: c.total_tasks,
                last_modified: c.last_modified.to_rfc3339(),
                status: status_key(c.completed_tasks, c.total_tasks).to_string(),
            })
            .collect();
        print_json(
            &serde_json::json!({"changes": json_output}),
            args.compact_json,
        )?;
        return Ok(());
    }

    println!("{}", t!("sdd.list.changes_header"));
    let name_width = changes.iter().map(|c| c.name.len()).fold(0, max);
    for change in changes {
        let padded = format!("{:<width$}", change.name, width = name_width);
        let stage = format!("{:<10}", change.stage.as_str());
        let status = format_task_status(change.completed_tasks, change.total_tasks);
        let time_ago = format_relative_time(change.last_modified);
        println!("  {}  {}  {:<12}  {}", padded, stage, status, time_ago);
    }

    Ok(())
}

fn list_specs_mode(root: &Path, args: &ListArgs) -> Result<()> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let _config = load_required_config(&llmanspec_dir)?;

    let specs_dir = root.join(LLMANSPEC_DIR_NAME).join("specs");
    if !specs_dir.exists() {
        if args.json {
            print_json(&serde_json::json!([]), args.compact_json)?;
        } else {
            println!("{}", t!("sdd.list.no_specs"));
        }
        return Ok(());
    }

    let spec_ids = list_specs(root)?;
    if spec_ids.is_empty() {
        if args.json {
            print_json(&serde_json::json!([]), args.compact_json)?;
        } else {
            println!("{}", t!("sdd.list.no_specs"));
        }
        return Ok(());
    }

    let mut specs = Vec::new();
    let config = load_required_config(&root.join(LLMANSPEC_DIR_NAME)).ok();
    let lang = config
        .as_ref()
        .map(|c| {
            crate::sdd::spec::validation::locale_to_gherkin_lang(Some(&c.locale), c.bdd.as_ref())
        })
        .unwrap_or_else(|| "en".to_string());

    for id in spec_ids {
        let spec_path = specs_dir.join(&id).join(SPEC_FILE);
        let content = fs::read_to_string(&spec_path)
            .map_err(|err| anyhow!("failed to read spec {}: {}", spec_path.display(), err))?;
        let spec =
            parse_spec(&content, &id).map_err(|err| anyhow!("{}: {}", spec_path.display(), err))?;
        let count = spec.requirements.len();
        let purpose = spec.overview.clone();

        // Extract valid_scope from raw TOON content
        let valid_scope: Vec<String> = content
            .lines()
            .find_map(|line| {
                let line = line.trim();
                if line.starts_with("valid_scope") || line.starts_with("validScope") {
                    if let Some(eq_idx) = line.find(':') {
                        let vals = line[eq_idx + 1..].trim();
                        Some(
                            vals.split(',')
                                .map(|v| v.trim().trim_matches('"').to_string())
                                .filter(|v| !v.is_empty())
                                .collect(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let morphology = {
            use crate::sdd::spec::backend::{BACKEND, SpecBackend};
            use crate::sdd::spec::partitioned::{compute_morphology, load_spec_harness_soft};
            let mut soft = Vec::new();
            let harness = load_spec_harness_soft(&specs_dir.join(&id), &lang, &mut soft);
            match BACKEND.parse_main_spec(&content, &format!("spec `{id}`")) {
                Ok(doc) => Some(compute_morphology(&doc, &harness)),
                Err(_) => None,
            }
        };

        specs.push((id, count, purpose, valid_scope, morphology));
    }

    specs.sort_by(|a, b| natural_cmp(&a.0, &b.0));

    if args.json {
        let json_output: Vec<_> = specs
            .iter()
            .map(|(id, count, purpose, scope, morphology)| {
                serde_json::json!({
                    "id": id,
                    "title": id,
                    "purpose": purpose,
                    "validScope": scope,
                    "requirementCount": count,
                    "health": null,
                    "staleness": null,
                    "morphology": morphology,
                })
            })
            .collect();
        print_json(&serde_json::json!(json_output), args.compact_json)?;
        return Ok(());
    }

    println!("{}", t!("sdd.list.specs_header"));
    let name_width = specs.iter().map(|s| s.0.len()).fold(0, max);
    for (id, count, _purpose, _scope, morphology) in specs {
        let padded = format!("{:<width$}", id, width = name_width);
        if let Some(m) = morphology {
            println!(
                "  {}     requirements {}  harness {}  dual-write {}",
                padded, count, m.harness_scenario_count, m.dual_write_count
            );
        } else {
            println!("  {}     requirements {}", padded, count);
        }
    }

    Ok(())
}

fn print_json(value: &serde_json::Value, compact: bool) -> Result<()> {
    if compact {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

fn status_key(completed: usize, total: usize) -> &'static str {
    if total == 0 {
        return "no-tasks";
    }
    if completed == total {
        return "complete";
    }
    "in-progress"
}

fn task_progress(changes_dir: &Path, change_name: &str) -> Result<(usize, usize)> {
    let tasks_path = changes_dir.join(change_name).join("tasks.md");
    match tasks::parse_tasks_file(&tasks_path)? {
        Some(report) => Ok((report.completed, report.total())),
        None => Ok((0, 0)),
    }
}

fn format_task_status(completed: usize, total: usize) -> String {
    if total == 0 {
        return t!("sdd.list.no_tasks_status").to_string();
    }
    if completed == total {
        return t!("sdd.list.complete_status").to_string();
    }
    format!("{}/{} tasks", completed, total)
}

fn last_modified(dir: &Path) -> Result<DateTime<Utc>> {
    let mut latest: Option<DateTime<Utc>> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                let modified: DateTime<Utc> = meta.modified()?.into();
                if latest.map(|l| modified > l).unwrap_or(true) {
                    latest = Some(modified);
                }
            }
        }
    }
    Ok(latest.unwrap_or_else(Utc::now))
}

fn format_relative_time(time: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(time);
    if diff.num_days() > 30 {
        return time.format("%Y-%m-%d").to_string();
    }
    if diff.num_days() > 0 {
        return format!("{}d ago", diff.num_days());
    }
    if diff.num_hours() > 0 {
        return format!("{}h ago", diff.num_hours());
    }
    if diff.num_minutes() > 0 {
        return format!("{}m ago", diff.num_minutes());
    }
    t!("sdd.list.just_now").to_string()
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();

    loop {
        let ac = ai.next();
        let bc = bi.next();

        match (ac, bc) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let num_ord = compare_number_runs(&mut ai, &mut bi, x, y);
                if num_ord != std::cmp::Ordering::Equal {
                    return num_ord;
                }
            }
            (Some(x), Some(y)) if x != y => return x.cmp(&y),
            _ => continue,
        }
    }
}

fn compare_number_runs<I: Iterator<Item = char>>(
    ai: &mut std::iter::Peekable<I>,
    bi: &mut std::iter::Peekable<I>,
    x: char,
    y: char,
) -> std::cmp::Ordering {
    // Collect digit runs, starting with the already-consumed char
    let mut an = (x as u32).wrapping_sub(b'0' as u32);
    let mut bn = (y as u32).wrapping_sub(b'0' as u32);

    while let Some(&c) = ai.peek() {
        if c.is_ascii_digit() {
            an = an * 10 + (c as u32).wrapping_sub(b'0' as u32);
            ai.next();
        } else {
            break;
        }
    }
    while let Some(&c) = bi.peek() {
        if c.is_ascii_digit() {
            bn = bn * 10 + (c as u32).wrapping_sub(b'0' as u32);
            bi.next();
        } else {
            break;
        }
    }

    an.cmp(&bn)
}

#[cfg(test)]
mod sort_tests {
    use super::*;

    #[test]
    fn natural_sort_digits() {
        let mut items = vec![
            "c100-xxx".to_string(),
            "c10-xxx".to_string(),
            "c3-xxx".to_string(),
            "c20-xxx".to_string(),
        ];
        items.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(items, vec!["c3-xxx", "c10-xxx", "c20-xxx", "c100-xxx"]);
    }

    #[test]
    fn natural_sort_mixed() {
        let mut items = vec!["c05-init".to_string(), "c5-short".to_string()];
        items.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(items, vec!["c05-init", "c5-short"]);
    }

    #[test]
    fn natural_sort_pure_alpha() {
        let mut items = vec!["beta".to_string(), "alpha".to_string(), "gamma".to_string()];
        items.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(items, vec!["alpha", "beta", "gamma"]);
    }
}
