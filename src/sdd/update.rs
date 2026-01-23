use crate::sdd::constants::{LLMANSPEC_DIR_NAME, LLMANSPEC_MARKERS, SPEC_DRIVEN_TEMPLATE_DIR};
use crate::sdd::fs_utils::update_file_with_markers;
use crate::sdd::templates::{default_agents_file, managed_block_content, spec_driven_templates};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub fn run(target: &Path) -> Result<()> {
    let llmanspec_path = target.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        return Err(anyhow!(
            "No llmanspec directory found. Run 'llman sdd init' first."
        ));
    }

    update_agents_file(&llmanspec_path)?;
    write_spec_driven_templates(&llmanspec_path)?;

    Ok(())
}

fn update_agents_file(llmanspec_path: &Path) -> Result<()> {
    let agents_path = llmanspec_path.join("AGENTS.md");
    if agents_path.exists() {
        update_file_with_markers(
            &agents_path,
            &managed_block_content(),
            LLMANSPEC_MARKERS.start,
            LLMANSPEC_MARKERS.end,
        )?;
        return Ok(());
    }

    fs::write(&agents_path, default_agents_file())?;
    Ok(())
}

fn write_spec_driven_templates(llmanspec_path: &Path) -> Result<()> {
    let template_dir = llmanspec_path.join(SPEC_DRIVEN_TEMPLATE_DIR);
    fs::create_dir_all(&template_dir)?;
    for template in spec_driven_templates() {
        let path = template_dir.join(template.name);
        fs::write(path, template.content)?;
    }
    Ok(())
}
