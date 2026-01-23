use crate::sdd::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::discovery::{list_changes, list_specs};
use crate::sdd::parser::parse_spec;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::cmp::max;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ListArgs {
    pub specs: bool,
    pub changes: bool,
    pub sort: String,
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct ChangeJson {
    name: String,
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
    completed_tasks: usize,
    total_tasks: usize,
    last_modified: DateTime<Utc>,
}

pub fn run(args: ListArgs) -> Result<()> {
    let root = Path::new(".");
    let _changes_requested = args.changes; // Explicit --changes mirrors the default behavior.
    let mode = if args.specs { "specs" } else { "changes" };
    if mode == "changes" {
        list_changes_mode(root, &args)
    } else {
        list_specs_mode(root, &args)
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
            println!("{}", serde_json::json!({"changes": []}));
        } else {
            println!("{}", t!("sdd.list.no_active_changes"));
        }
        return Ok(());
    }

    let mut changes = Vec::new();
    for change in change_ids {
        let (completed, total) = task_progress(&changes_dir, &change)?;
        let last_modified = last_modified(&changes_dir.join(&change))?;
        changes.push(ChangeInfo {
            name: change,
            completed_tasks: completed,
            total_tasks: total,
            last_modified,
        });
    }

    let sort_by_name = args.sort == "name";
    if sort_by_name {
        changes.sort_by(|a, b| a.name.cmp(&b.name));
    } else {
        changes.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    }

    if args.json {
        let json_output: Vec<ChangeJson> = changes
            .iter()
            .map(|c| ChangeJson {
                name: c.name.clone(),
                completed_tasks: c.completed_tasks,
                total_tasks: c.total_tasks,
                last_modified: c.last_modified.to_rfc3339(),
                status: status_key(c.completed_tasks, c.total_tasks).to_string(),
            })
            .collect();
        println!("{}", serde_json::json!({"changes": json_output}));
        return Ok(());
    }

    println!("{}", t!("sdd.list.changes_header"));
    let name_width = changes.iter().map(|c| c.name.len()).fold(0, max);
    for change in changes {
        let padded = format!("{:<width$}", change.name, width = name_width);
        let status = format_task_status(change.completed_tasks, change.total_tasks);
        let time_ago = format_relative_time(change.last_modified);
        println!("  {}     {:<12}  {}", padded, status, time_ago);
    }

    Ok(())
}

fn list_specs_mode(root: &Path, args: &ListArgs) -> Result<()> {
    let specs_dir = root.join(LLMANSPEC_DIR_NAME).join("specs");
    if !specs_dir.exists() {
        if args.json {
            println!("[]");
        } else {
            println!("{}", t!("sdd.list.no_specs"));
        }
        return Ok(());
    }

    let spec_ids = list_specs(root)?;
    if spec_ids.is_empty() {
        if args.json {
            println!("[]");
        } else {
            println!("{}", t!("sdd.list.no_specs"));
        }
        return Ok(());
    }

    let mut specs = Vec::new();
    for id in spec_ids {
        let spec_path = specs_dir.join(&id).join("spec.md");
        let requirement_count = match fs::read_to_string(&spec_path) {
            Ok(content) => match parse_spec(&content, &id) {
                Ok(spec) => spec.requirements.len(),
                Err(_) => 0,
            },
            Err(_) => 0,
        };
        specs.push((id, requirement_count));
    }

    specs.sort_by(|a, b| a.0.cmp(&b.0));

    if args.json {
        let json_output: Vec<_> = specs
            .iter()
            .map(|(id, count)| {
                serde_json::json!({
                    "id": id,
                    "title": id,
                    "requirementCount": count
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&json_output)?);
        return Ok(());
    }

    println!("{}", t!("sdd.list.specs_header"));
    let name_width = specs.iter().map(|s| s.0.len()).fold(0, max);
    for (id, count) in specs {
        let padded = format!("{:<width$}", id, width = name_width);
        println!("  {}     requirements {}", padded, count);
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
    let content = match fs::read_to_string(tasks_path) {
        Ok(content) => content,
        Err(_) => return Ok((0, 0)),
    };
    let mut total = 0;
    let mut completed = 0;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("- [") || trimmed.starts_with("* [") {
            total += 1;
            if trimmed.to_lowercase().starts_with("- [x]")
                || trimmed.to_lowercase().starts_with("* [x]")
            {
                completed += 1;
            }
        }
    }
    Ok((completed, total))
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
