use crate::sdd::constants::{LLMANSPEC_DIR_NAME, SPEC_DRIVEN_TEMPLATE_DIR};
use crate::sdd::templates::{default_agents_file, render_project_template, spec_driven_templates};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub fn run(target: &Path) -> Result<()> {
    ensure_directory(target)?;

    let llmanspec_path = target.join(LLMANSPEC_DIR_NAME);
    if llmanspec_path.exists() {
        return Err(anyhow!("llmanspec directory already exists"));
    }

    create_structure(&llmanspec_path)?;
    write_project_file(&llmanspec_path, target)?;
    write_agents_file(&llmanspec_path)?;
    write_spec_driven_templates(&llmanspec_path)?;

    Ok(())
}

fn ensure_directory(target: &Path) -> Result<()> {
    if target.exists() {
        if !target.is_dir() {
            return Err(anyhow!("Target path is not a directory"));
        }
        return Ok(());
    }
    fs::create_dir_all(target)?;
    Ok(())
}

fn create_structure(llmanspec_path: &Path) -> Result<()> {
    fs::create_dir_all(llmanspec_path.join("specs"))?;
    fs::create_dir_all(llmanspec_path.join("changes").join("archive"))?;
    fs::create_dir_all(llmanspec_path.join(SPEC_DRIVEN_TEMPLATE_DIR))?;
    Ok(())
}

fn write_project_file(llmanspec_path: &Path, target: &Path) -> Result<()> {
    let project_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Project");
    let content = render_project_template(project_name);
    let path = llmanspec_path.join("project.md");
    fs::write(path, content)?;
    Ok(())
}

fn write_agents_file(llmanspec_path: &Path) -> Result<()> {
    let agents_path = llmanspec_path.join("AGENTS.md");
    let content = default_agents_file();
    fs::write(&agents_path, content)?;
    Ok(())
}

fn write_spec_driven_templates(llmanspec_path: &Path) -> Result<()> {
    let template_dir = llmanspec_path.join(SPEC_DRIVEN_TEMPLATE_DIR);
    for template in spec_driven_templates() {
        let path = template_dir.join(template.name);
        fs::write(path, template.content)?;
    }
    Ok(())
}
