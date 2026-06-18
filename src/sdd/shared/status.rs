use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::{list_changes, list_specs};
use crate::sdd::shared::tasks;
use crate::sdd::spec::validation::{ChangeStage, determine_stage};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug)]
pub struct StatusArgs {
    pub json: bool,
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

pub fn run(args: StatusArgs) -> Result<()> {
    let root = Path::new(".");
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);

    if !llmanspec_dir.exists() {
        return Err(anyhow::anyhow!(
            "llmanspec/ not found. Run `llman sdd init` first."
        ));
    }

    let changes = list_changes(root).unwrap_or_default();
    let specs = list_specs(root).unwrap_or_default();

    let changes_dir = llmanspec_dir.join("changes");
    let mut draft = 0;
    let mut specified = 0;
    let mut designed = 0;
    let mut full = 0;
    let mut pending_validation = 0;

    for change in &changes {
        let change_dir = changes_dir.join(change);
        let stage = determine_stage(&change_dir);
        match stage {
            ChangeStage::Draft => draft += 1,
            ChangeStage::Specified => specified += 1,
            ChangeStage::Designed => designed += 1,
            ChangeStage::Full => {
                full += 1;
                // Check if tasks are incomplete
                let tasks_path = change_dir.join("tasks.md");
                if let Ok(Some(report)) = tasks::parse_tasks_file(&tasks_path)
                    && report.completed < report.total()
                {
                    pending_validation += 1;
                }
            }
        }
    }

    let active_changes = changes.len();

    if args.json {
        let status = StatusJson {
            active_changes,
            draft,
            specified,
            designed,
            full,
            pending_validation,
            specs: specs.len(),
        };
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("Project Status");
        println!("══════════════");
        println!("Active changes: {}", active_changes);
        if active_changes > 0 {
            println!("  Draft:     {}", draft);
            println!("  Specified: {}", specified);
            println!("  Designed:  {}", designed);
            println!("  Full:      {}", full);
        }
        println!("Pending validation: {}", pending_validation);
        println!("Specs: {}", specs.len());
    }

    Ok(())
}
