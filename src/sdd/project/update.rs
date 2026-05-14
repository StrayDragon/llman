use super::config::{SddConfig, load_or_create_config};
use super::fs_utils::update_file_with_markers;
use super::templates::root_stub_content;
use super::update_skills::{self, UpdateSkillsArgs};
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, LLMANSPEC_MARKERS};
use anyhow::{Result, anyhow};
use std::path::Path;

pub fn run(target: &Path) -> Result<()> {
    let llmanspec_path = target.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        return Err(anyhow!(
            "No llmanspec directory found. Run 'llman sdd init' first."
        ));
    }

    let config = load_or_create_config(&llmanspec_path)?;
    write_root_agents_file(target, &config)?;
    update_skills::run_with_root(
        target,
        UpdateSkillsArgs {
            no_interactive: true,
            commands_only: false,
            skills_only: false,
        },
    )?;

    Ok(())
}

fn write_root_agents_file(target: &Path, config: &SddConfig) -> Result<()> {
    let agents_path = target.join("AGENTS.md");
    let content = root_stub_content(config, target)?;
    update_file_with_markers(
        &agents_path,
        &content,
        LLMANSPEC_MARKERS.start,
        LLMANSPEC_MARKERS.end,
    )?;
    Ok(())
}
