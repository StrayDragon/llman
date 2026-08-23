use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::{discover_changes, list_specs};
use crate::sdd::shared::tasks;
use crate::sdd::spec::backend::FEATURE_BACKEND;
use crate::sdd::spec::backend::feature_backend::compute_rule_morphology;
use crate::sdd::spec::validation::{ChangeStage, determine_stage, resolve_spec_file};
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
    /// Relative to `llmanspec/changes/` (r128).
    path: String,
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
    path: String,
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

    let locs = discover_changes(root)?;
    if locs.is_empty() {
        if args.json {
            print_json(&serde_json::json!({"changes": []}), args.compact_json)?;
        } else {
            println!("{}", t!("sdd.list.no_active_changes"));
        }
        return Ok(());
    }

    let mut changes = Vec::new();
    for loc in locs {
        let change_dir = loc.abs_dir(root);
        let (completed, total) = task_progress(&change_dir)?;
        let last_modified = last_modified(&change_dir)?;
        let stage = determine_stage(&change_dir);
        changes.push(ChangeInfo {
            name: loc.id,
            path: loc.path,
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
                path: c.path.clone(),
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
    for id in spec_ids {
        let content = resolve_spec_file(&specs_dir, &id)
            .and_then(|p| fs::read_to_string(&p).map_err(|e| anyhow!("{}", e)));
        let Ok(content) = content else {
            // Legacy toon / unreadable specs surface via validate; keep listing
            // resilient by emitting an empty morphology row.
            specs.push((id, 0, String::new(), Vec::new(), None));
            continue;
        };
        let parsed = match FEATURE_BACKEND.parse_content(&content, &format!("spec `{id}`")) {
            Ok(parsed) => parsed,
            Err(_) => {
                specs.push((id, 0, String::new(), Vec::new(), None));
                continue;
            }
        };
        let count = parsed.rule_scenarios().count();
        let purpose = parsed.purpose.clone();
        let valid_scope = parsed.valid_scope.clone();
        let morphology = Some(compute_rule_morphology(&parsed));

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
                "  {}     rules {}  enforced {}  manual {}  pending {}  acceptance {}",
                padded,
                m.rule_count,
                m.rule_enforced_count,
                m.rule_manual_count,
                m.rule_pending_count,
                m.acceptance_count
            );
        } else {
            println!("  {}     rules {}", padded, count);
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

fn task_progress(change_dir: &Path) -> Result<(usize, usize)> {
    let tasks_path = change_dir.join("tasks.md");
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
