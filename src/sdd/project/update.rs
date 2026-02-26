use super::config::{SddConfig, load_or_create_config};
use super::fs_utils::update_file_with_markers;
use super::templates::{
    TemplateStyle, default_agents_file, managed_block_content, root_stub_content,
    spec_driven_templates,
};
use crate::sdd::shared::constants::{
    LLMANSPEC_DIR_NAME, LLMANSPEC_MARKERS, SPEC_DRIVEN_TEMPLATE_DIR,
};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub fn run(target: &Path, style: TemplateStyle) -> Result<()> {
    let llmanspec_path = target.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        return Err(anyhow!(
            "No llmanspec directory found. Run 'llman sdd init' first."
        ));
    }

    let config = load_or_create_config(&llmanspec_path)?;
    update_agents_file(&llmanspec_path, target, &config, style)?;
    write_root_agents_file(target, &config, style)?;
    write_spec_driven_templates(&llmanspec_path, target, &config, style)?;

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

    fs::write(&agents_path, default_agents_file(config, target, style)?)?;
    Ok(())
}

fn write_spec_driven_templates(
    llmanspec_path: &Path,
    target: &Path,
    config: &SddConfig,
    style: TemplateStyle,
) -> Result<()> {
    let template_dir = llmanspec_path.join(SPEC_DRIVEN_TEMPLATE_DIR);
    fs::create_dir_all(&template_dir)?;
    for template in spec_driven_templates(config, target, style)? {
        let path = template_dir.join(template.name);
        fs::write(path, template.content)?;
    }
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
