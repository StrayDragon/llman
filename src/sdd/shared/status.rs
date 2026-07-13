use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::shared::discovery::{extract_archived_change_id, list_changes, list_specs};
use crate::sdd::shared::tasks;
use crate::sdd::spec::backend::{BACKEND as SPEC_BACKEND, SpecBackend};
use crate::sdd::spec::validation::{ChangeStage, determine_stage};
use anyhow::{Result, anyhow};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    Toon,
    Json,
}

#[derive(Debug)]
pub struct StatusArgs {
    pub target: Option<String>,
    pub json: bool,
    pub format: Option<String>,
}

impl StatusArgs {
    fn resolved_format(&self) -> Format {
        if self.json {
            return Format::Json;
        }
        match self.format.as_deref() {
            Some("json") => Format::Json,
            Some("toon") | None => Format::Toon,
            // Invalid format is caught in run() before resolved_format() is called
            Some(_) => Format::Toon,
        }
    }
}

#[derive(Debug, Serialize)]
struct StatusJson {
    #[serde(rename = "activeChanges")]
    active_changes: usize,
    draft: usize,
    specified: usize,
    designed: usize,
    full: usize,
    #[serde(rename = "pendingValidation")]
    pending_validation: usize,
    specs: usize,
}

#[derive(Debug, Serialize)]
struct SingleChangeJson {
    change: String,
    stage: String,
    priority: String,
    #[serde(rename = "completedTasks")]
    completed_tasks: usize,
    #[serde(rename = "totalTasks")]
    total_tasks: usize,
    #[serde(rename = "nextAction")]
    next_action: String,
}

// ── Target resolution ──

#[derive(Clone)]
struct ChangeInfo {
    name: String,
    dir_name: String, // full directory name (for archives: with date prefix)
    is_archived: bool,
    stage: ChangeStage,
    tasks_done: usize,
    tasks_total: usize,
    priority: usize, // 0 = no prefix, otherwise parsed from c<N>-
}

fn extract_priority(dir_name: &str) -> usize {
    // Look for c<N>- prefix
    if dir_name.starts_with('c') || dir_name.starts_with('C') {
        let rest = &dir_name[1..];
        let num_end = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        if num_end > 0
            && rest.as_bytes().get(num_end) == Some(&b'-')
            && let Ok(n) = rest[..num_end].parse::<usize>()
        {
            return n;
        }
    }
    0
}

/// Collect all active changes with their metadata
fn collect_active_changes(root: &Path) -> Vec<ChangeInfo> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let changes_dir = llmanspec_dir.join("changes");
    let mut result = Vec::new();

    for name in list_changes(root).unwrap_or_default() {
        let change_dir = changes_dir.join(&name);
        let stage = determine_stage(&change_dir);
        let (done, total) = parse_task_counts(&change_dir);
        result.push(ChangeInfo {
            dir_name: name.clone(),
            name: name.clone(),
            is_archived: false,
            stage,
            tasks_done: done,
            tasks_total: total,
            priority: extract_priority(&name),
        });
    }

    result
}

/// Collect all archived changes with their metadata
fn collect_archived_changes(root: &Path) -> Vec<ChangeInfo> {
    let archive_dir = root
        .join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join("archive");
    let mut result = Vec::new();

    let entries = match std::fs::read_dir(&archive_dir) {
        Ok(e) => e,
        Err(_) => return result,
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if dir_name.starts_with('.') {
            continue;
        }
        // Extract the change id (after date prefix)
        let name = extract_archived_change_id(&dir_name).unwrap_or_else(|| dir_name.clone());
        let priority = extract_priority(&name);
        let change_dir = entry.path();
        // For archived changes, we still check stage from the artifacts present
        let stage = determine_stage(&change_dir);
        let (done, total) = parse_task_counts(&change_dir);
        result.push(ChangeInfo {
            dir_name,
            name,
            is_archived: true,
            stage,
            tasks_done: done,
            tasks_total: total,
            priority,
        });
    }

    result
}

fn parse_task_counts(change_dir: &Path) -> (usize, usize) {
    let tasks_path = change_dir.join("tasks.md");
    if let Ok(Some(report)) = tasks::parse_tasks_file(&tasks_path) {
        (report.completed, report.total())
    } else {
        (0, 0)
    }
}

/// Resolve TARGET to either a unique ChangeInfo or a list of matches.
enum TargetResult {
    Single(ChangeInfo),
    Multiple(Vec<ChangeInfo>),
    None,
}

fn resolve_target(root: &Path, target: &str) -> TargetResult {
    let active = collect_active_changes(root);
    let archived = collect_archived_changes(root);

    // 1. Exact match against active changes
    if let Some(ci) = active
        .iter()
        .find(|c| c.name == target || c.dir_name == target)
    {
        return TargetResult::Single(ci.clone());
    }

    // 2. Exact match against archived (by dir_name = full date-prefixed name)
    if let Some(ci) = archived.iter().find(|c| c.dir_name == target) {
        return TargetResult::Single(ci.clone());
    }

    // 3. Fuzzy / date-prefix match against all (active + archived)
    let lower = target.to_lowercase();
    let mut matches: Vec<ChangeInfo> = active
        .into_iter()
        .chain(archived)
        .filter(|c| {
            c.dir_name.to_lowercase().contains(&lower) || c.name.to_lowercase().contains(&lower)
        })
        .collect();

    matches.sort_by_key(|c| c.priority);

    match matches.len() {
        0 => TargetResult::None,
        1 => TargetResult::Single(matches.remove(0)),
        _ => TargetResult::Multiple(matches),
    }
}

// ── TOON output builders ──

fn toon_project_overview(changes: &[ChangeInfo], specs_count: usize) -> String {
    let mut out = String::new();
    out.push_str("kind: llman.sdd.status\n");
    out.push_str(&format!(
        "counts{{active,specs}}:\n  {},{}",
        changes.len(),
        specs_count
    ));
    out.push('\n');

    if !changes.is_empty() {
        out.push_str(&format!(
            "changes[{}]{{name,stage,tasks,next}}:\n",
            changes.len()
        ));
        for c in changes {
            let stage_str = match c.stage {
                ChangeStage::Draft => "draft",
                ChangeStage::Specified => "spec",
                ChangeStage::Designed => "design",
                ChangeStage::Full => "full",
            };
            let tasks_str = if c.tasks_total > 0 {
                format!("{}/{}", c.tasks_done, c.tasks_total)
            } else {
                "0/0".to_string()
            };
            let next = derive_next_action(c);
            // Quote values that may contain special chars
            let name_quoted = maybe_quote(&c.name);
            let next_quoted = if next.contains(',') || next.contains('"') {
                format!("\"{}\"", next.replace('"', r#"\""#))
            } else if next.is_empty() {
                "".to_string()
            } else {
                next
            };
            out.push_str(&format!(
                "  {},{},{},{},{}",
                name_quoted,
                stage_str,
                tasks_str,
                if c.is_archived { "archived" } else { "active" },
                next_quoted
            ));
            out.push('\n');
        }
    }
    out
}

fn toon_single_change(ci: &ChangeInfo, root: &Path) -> String {
    let mut out = String::new();
    out.push_str("kind: llman.sdd.status\n");

    let stage_str = match ci.stage {
        ChangeStage::Draft => "draft",
        ChangeStage::Specified => "spec",
        ChangeStage::Designed => "design",
        ChangeStage::Full => "full",
    };

    out.push_str(&format!(
        "change{{name,stage,priority,tasks}}:\n  {},{},{},{}",
        maybe_quote(&ci.name),
        if ci.is_archived {
            "archived"
        } else {
            stage_str
        },
        if ci.priority > 0 {
            format!("c{}", ci.priority)
        } else {
            "-".to_string()
        },
        if ci.tasks_total > 0 {
            format!("{}/{}", ci.tasks_done, ci.tasks_total)
        } else {
            "0/0".to_string()
        },
    ));
    out.push('\n');

    if ci.is_archived {
        // Archived: show ops from delta specs — parse directly for req_id + title
        let archive_dir = root
            .join(LLMANSPEC_DIR_NAME)
            .join("changes")
            .join("archive")
            .join(&ci.dir_name);
        let specs_dir = archive_dir.join("specs");
        if specs_dir.exists() {
            let mut ops: Vec<(String, String, String)> = Vec::new(); // (op_str, req_id, title)
            if let Ok(entries) = std::fs::read_dir(&specs_dir) {
                for entry in entries.flatten() {
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let spec_file = entry.path().join(SPEC_FILE);
                    if !spec_file.exists() {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(&spec_file) {
                        let ctx = format!("archived {}", ci.dir_name);
                        if let Ok(doc) = SPEC_BACKEND.parse_delta_spec(&content, &ctx) {
                            for op in &doc.ops {
                                let op_str = match op.op.trim().to_ascii_lowercase().as_str() {
                                    "add_requirement" => "add_req",
                                    "modify_requirement" => "mod_req",
                                    "remove_requirement" => "rm_req",
                                    "rename_requirement" => "ren_req",
                                    _ => continue,
                                };
                                let title = op.title.as_deref().unwrap_or("?");
                                ops.push((
                                    op_str.to_string(),
                                    op.req_id.clone(),
                                    title.to_string(),
                                ));
                            }
                        }
                    }
                }
            }
            if !ops.is_empty() {
                out.push_str(&format!("ops[{}]{{op,req_id,title}}:\n", ops.len()));
                for (op_str, req_id, title) in &ops {
                    out.push_str(&format!("  {},{},{}\n", op_str, req_id, maybe_quote(title)));
                }
            }
        }
    } else {
        // Active: show incomplete tasks
        let change_dir = root
            .join(LLMANSPEC_DIR_NAME)
            .join("changes")
            .join(&ci.dir_name);
        if let Ok(Some(report)) = tasks::parse_tasks_file(&change_dir.join("tasks.md")) {
            let incomplete: Vec<_> = report
                .items
                .iter()
                .filter(|t| matches!(t.status, tasks::TaskStatus::Pending))
                .collect();
            if !incomplete.is_empty() {
                out.push_str(&format!("tasks[{}]{{id,title,test}}:\n", incomplete.len()));
                for (i, task) in incomplete.iter().enumerate() {
                    let task_id = format!("t{}", i + 1);
                    // Try to extract a test command from the task text (look for backtick command)
                    let test_cmd = extract_test_command(&task.text);
                    out.push_str(&format!(
                        "  {},{},{}\n",
                        task_id,
                        maybe_quote(&task.text),
                        if test_cmd.is_empty() {
                            "".to_string()
                        } else {
                            maybe_quote(&test_cmd)
                        }
                    ));
                }
            }
        }
    }

    // next action
    let next = derive_next_action(ci);
    if !next.is_empty() {
        out.push_str(&format!("next: {}\n", maybe_quote(&next)));
    }

    out
}

fn toon_multiple_matches(matches: &[ChangeInfo]) -> String {
    let mut out = String::new();
    out.push_str("kind: llman.sdd.status\n");
    out.push_str(&format!("multiple_matches:\n  count: {}", matches.len()));
    out.push('\n');
    out.push_str(&format!(
        "changes[{}]{{name,type,tasks,priority}}:\n",
        matches.len()
    ));
    for c in matches {
        let tasks_str = if c.tasks_total > 0 {
            format!("{}/{}", c.tasks_done, c.tasks_total)
        } else {
            "0/0".to_string()
        };
        let typ = if c.is_archived { "archived" } else { "active" };
        out.push_str(&format!(
            "  {},{},{},{}\n",
            maybe_quote(&c.dir_name),
            typ,
            tasks_str,
            if c.priority > 0 {
                format!("c{}", c.priority)
            } else {
                "-".to_string()
            }
        ));
    }
    out
}

fn derive_next_action(ci: &ChangeInfo) -> String {
    if ci.is_archived {
        return String::new();
    }
    match ci.stage {
        ChangeStage::Draft => "propose".to_string(),
        ChangeStage::Specified => "design".to_string(),
        ChangeStage::Designed => "tasks".to_string(),
        ChangeStage::Full => {
            if ci.tasks_done < ci.tasks_total {
                format!("impl task {}", ci.tasks_done + 1)
            } else {
                "archive".to_string()
            }
        }
    }
}

fn extract_test_command(task_text: &str) -> String {
    // Look for backtick-wrapped commands in task text
    if let Some(start) = task_text.find('`') {
        let after = &task_text[start + 1..];
        if let Some(end) = after.find('`') {
            return after[..end].to_string();
        }
    }
    String::new()
}

fn maybe_quote(s: &str) -> String {
    if s.is_empty() {
        return "".to_string();
    }
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains(':') {
        format!("\"{}\"", s.replace('"', r#"\""#))
    } else {
        s.to_string()
    }
}

// ── JSON output builders ──

fn json_project_overview(changes: &[ChangeInfo], specs_count: usize) -> Result<()> {
    let mut draft = 0;
    let mut specified = 0;
    let mut designed = 0;
    let mut full = 0;
    let mut pending_validation = 0;

    for c in changes {
        match c.stage {
            ChangeStage::Draft => draft += 1,
            ChangeStage::Specified => specified += 1,
            ChangeStage::Designed => designed += 1,
            ChangeStage::Full => {
                full += 1;
                if c.tasks_done < c.tasks_total {
                    pending_validation += 1;
                }
            }
        }
    }

    let status = StatusJson {
        active_changes: changes.len(),
        draft,
        specified,
        designed,
        full,
        pending_validation,
        specs: specs_count,
    };
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

fn json_single_change(ci: &ChangeInfo) -> Result<()> {
    let stage_str = match ci.stage {
        ChangeStage::Draft => "draft",
        ChangeStage::Specified => "specified",
        ChangeStage::Designed => "designed",
        ChangeStage::Full => "full",
    };
    let next = derive_next_action(ci);

    if ci.is_archived {
        #[derive(Serialize)]
        struct ArchivedJsonOut {
            change: String,
            status: String,
            archived: bool,
            next_action: String,
        }
        let out = ArchivedJsonOut {
            change: ci.dir_name.clone(),
            status: stage_str.to_string(),
            archived: true,
            next_action: next,
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        let out = SingleChangeJson {
            change: ci.name.clone(),
            stage: stage_str.to_string(),
            priority: if ci.priority > 0 {
                format!("c{}", ci.priority)
            } else {
                "-".to_string()
            },
            completed_tasks: ci.tasks_done,
            total_tasks: ci.tasks_total,
            next_action: next,
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
    }
    Ok(())
}

fn json_multiple_matches(matches: &[ChangeInfo]) -> Result<()> {
    #[derive(Serialize)]
    struct MatchItem {
        name: String,
        #[serde(rename = "type")]
        typ: String,
        tasks: String,
        priority: String,
    }
    #[derive(Serialize)]
    struct MultipleJson {
        count: usize,
        matches: Vec<MatchItem>,
    }

    let items: Vec<MatchItem> = matches
        .iter()
        .map(|c| MatchItem {
            name: c.dir_name.clone(),
            typ: if c.is_archived {
                "archived".to_string()
            } else {
                "active".to_string()
            },
            tasks: if c.tasks_total > 0 {
                format!("{}/{}", c.tasks_done, c.tasks_total)
            } else {
                "0/0".to_string()
            },
            priority: if c.priority > 0 {
                format!("c{}", c.priority)
            } else {
                "-".to_string()
            },
        })
        .collect();

    let out = MultipleJson {
        count: matches.len(),
        matches: items,
    };
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

// ── Main entry ──

pub fn run(args: StatusArgs) -> Result<()> {
    let root = Path::new(".");
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);

    if !llmanspec_dir.exists() {
        return Err(anyhow!("llmanspec/ not found. Run `llman sdd init` first."));
    }

    // Validate format if explicitly provided
    if let Some(ref fmt) = args.format
        && fmt != "toon"
        && fmt != "json"
    {
        return Err(anyhow!("Invalid format '{}'. Supported: toon, json", fmt));
    }

    let format = args.resolved_format();

    match &args.target {
        None => {
            // Project-level overview
            let changes = collect_active_changes(root);
            let specs_count = list_specs(root).unwrap_or_default().len();
            match format {
                Format::Toon => print!("{}", toon_project_overview(&changes, specs_count)),
                Format::Json => json_project_overview(&changes, specs_count)?,
            }
        }
        Some(target) => {
            let resolved = resolve_target(root, target);
            match resolved {
                TargetResult::Single(ci) => match format {
                    Format::Toon => print!("{}", toon_single_change(&ci, root)),
                    Format::Json => json_single_change(&ci)?,
                },
                TargetResult::Multiple(matches) => match format {
                    Format::Toon => print!("{}", toon_multiple_matches(&matches)),
                    Format::Json => json_multiple_matches(&matches)?,
                },
                TargetResult::None => {
                    let suggestions = suggest_similar_changes(root, target);
                    return Err(anyhow!(
                        "No change matches '{}'.{}",
                        target,
                        if suggestions.is_empty() {
                            String::new()
                        } else {
                            format!(" Did you mean: {}", suggestions.join(", "))
                        }
                    ));
                }
            }
        }
    }

    Ok(())
}

fn suggest_similar_changes(root: &Path, target: &str) -> Vec<String> {
    let active = collect_active_changes(root);
    let archived = collect_archived_changes(root);
    let lower = target.to_lowercase();

    let mut names: Vec<String> = active
        .into_iter()
        .chain(archived)
        .map(|c| c.dir_name)
        .filter(|n| {
            let nl = n.to_lowercase();
            // Simple similarity: share at least 3 consecutive chars
            (0..nl.len().saturating_sub(2)).any(|i| {
                let sub = &nl[i..i + 3];
                lower.contains(sub)
            })
        })
        .collect();

    names.sort();
    names.truncate(5);
    names
}
