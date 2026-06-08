use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::{extract_archived_change_id, list_changes};
use crate::sdd::shared::tasks::{self, TaskStatus};
use anyhow::Result;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct OrphansArgs {
    pub json: bool,
    pub compact_json: bool,
}

struct OrphanItem {
    text: String,
    legacy: bool,
}

pub fn run(args: OrphansArgs) -> Result<()> {
    let root = Path::new(".");
    run_with_root(root, args)
}

fn run_with_root(root: &Path, args: OrphansArgs) -> Result<()> {
    let active_ids: HashSet<String> = list_changes(root)?.into_iter().collect();
    let archived_ids = collect_archived_ids(root);
    let all_known: HashSet<&str> = active_ids
        .iter()
        .chain(archived_ids.iter())
        .map(|s| s.as_str())
        .collect();

    let mut orphans: BTreeMap<String, Vec<OrphanItem>> = BTreeMap::new();

    scan_dir_for_orphans(
        &root.join(LLMANSPEC_DIR_NAME).join("changes"),
        &all_known,
        &mut orphans,
        false,
    );
    scan_dir_for_orphans(
        &root
            .join(LLMANSPEC_DIR_NAME)
            .join("changes")
            .join("archive"),
        &all_known,
        &mut orphans,
        true,
    );

    let total: usize = orphans.values().map(|v| v.len()).sum();

    if args.json {
        print_json(&orphans, total, args.compact_json)?;
    } else {
        print_text(&orphans, total);
    }

    Ok(())
}

fn scan_dir_for_orphans(
    dir: &Path,
    all_known: &HashSet<&str>,
    orphans: &mut BTreeMap<String, Vec<OrphanItem>>,
    is_archive: bool,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if dir_name.starts_with('.') || (!is_archive && dir_name == "archive") {
            continue;
        }

        let change_id = if is_archive {
            match extract_archived_change_id(&dir_name) {
                Some(id) => id,
                None => continue,
            }
        } else {
            dir_name.clone()
        };

        let tasks_path = entry.path().join("tasks.md");
        let report = match tasks::parse_tasks_file(&tasks_path) {
            Ok(Some(r)) => r,
            _ => continue,
        };

        for item in &report.items {
            match &item.status {
                TaskStatus::Deferred { target } if !all_known.contains(target.as_str()) => {
                    orphans
                        .entry(change_id.clone())
                        .or_default()
                        .push(OrphanItem {
                            text: item.text.clone(),
                            legacy: false,
                        });
                }
                TaskStatus::LegacyDefer { .. } => {
                    orphans
                        .entry(change_id.clone())
                        .or_default()
                        .push(OrphanItem {
                            text: item.text.clone(),
                            legacy: true,
                        });
                }
                _ => {}
            }
        }
    }
}

fn collect_archived_ids(root: &Path) -> HashSet<String> {
    let archive_dir = root
        .join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join("archive");
    let entries = match fs::read_dir(archive_dir) {
        Ok(e) => e,
        Err(_) => return HashSet::new(),
    };
    let mut ids = HashSet::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(id) = extract_archived_change_id(&name) {
            ids.insert(id);
        }
    }
    ids
}

fn print_text(orphans: &BTreeMap<String, Vec<OrphanItem>>, total: usize) {
    if total == 0 {
        println!("{}", t!("sdd.orphans.no_orphans"));
        return;
    }

    println!(
        "{}",
        t!("sdd.orphans.header", count = total, changes = orphans.len())
    );
    println!();

    for (change_id, items) in orphans {
        println!("{}", t!("sdd.orphans.change_header", change = change_id));
        for item in items {
            if item.legacy {
                println!("{}", t!("sdd.orphans.legacy_item", task = item.text));
            } else {
                println!("{}", t!("sdd.orphans.item", task = item.text));
            }
        }
        println!();
    }

    println!("{}", t!("sdd.orphans.suggestion"));
}

fn print_json(
    orphans: &BTreeMap<String, Vec<OrphanItem>>,
    total: usize,
    compact: bool,
) -> Result<()> {
    let items: serde_json::Value = orphans
        .iter()
        .map(|(change_id, items)| {
            let tasks: Vec<serde_json::Value> = items
                .iter()
                .map(|item| {
                    serde_json::json!({
                        "text": item.text,
                        "legacy": item.legacy,
                    })
                })
                .collect();
            (change_id.clone(), serde_json::Value::Array(tasks))
        })
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    let output = serde_json::json!({
        "totalOrphans": total,
        "changes": orphans.len(),
        "items": items,
    });

    if compact {
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&output)?);
    }

    Ok(())
}
