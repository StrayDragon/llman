use super::config::{SddConfig, load_or_create_config};
use super::fs_utils::update_file_with_markers;
use super::templates::{
    TemplateStyle, default_agents_file, managed_block_content, root_stub_content,
};
use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, LLMANSPEC_MARKERS};
use anyhow::{Result, anyhow};
use std::path::Path;

pub fn run(target: &Path, style: TemplateStyle) -> Result<()> {
    let llmanspec_path = target.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        let init_cmd = "llman sdd init";
        return Err(anyhow!(
            "No llmanspec directory found. Run '{init_cmd}' first."
        ));
    }

    let config = load_or_create_config(&llmanspec_path)?;
    update_agents_file(&llmanspec_path, target, &config, style)?;
    write_root_agents_file(target, &config, style)?;

    Ok(())
}

fn update_agents_file(
    llmanspec_path: &Path,
    target: &Path,
    config: &SddConfig,
    style: TemplateStyle,
) -> Result<()> {
    let agents_path = llmanspec_path.join("AGENTS.md");
    if agents_path.exists() {
        update_file_with_markers(
            &agents_path,
            &managed_block_content(config, target, style)?,
            LLMANSPEC_MARKERS.start,
            LLMANSPEC_MARKERS.end,
        )?;
        return Ok(());
    }

    let content = default_agents_file(config, target, style)?;
    atomic_write_with_mode(&agents_path, content.as_bytes(), None)?;
    Ok(())
}

fn write_root_agents_file(target: &Path, config: &SddConfig, style: TemplateStyle) -> Result<()> {
    let agents_path = target.join("AGENTS.md");
    let content = root_stub_content(config, target, style)?;
    update_file_with_markers(
        &agents_path,
        &content,
        LLMANSPEC_MARKERS.start,
        LLMANSPEC_MARKERS.end,
    )?;
    Ok(())
}
