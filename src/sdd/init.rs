use crate::sdd::config::{SddConfig, config_with_locale, write_config};
use crate::sdd::constants::{LLMANSPEC_DIR_NAME, LLMANSPEC_MARKERS, SPEC_DRIVEN_TEMPLATE_DIR};
use crate::sdd::fs_utils::update_file_with_markers;
use crate::sdd::templates::{
    default_agents_file, render_project_template, root_stub_content, spec_driven_templates,
};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub fn run(target: &Path, locale: Option<&str>) -> Result<()> {
    ensure_directory(target)?;

    let llmanspec_path = target.join(LLMANSPEC_DIR_NAME);
    if llmanspec_path.exists() {
        return Err(anyhow!("llmanspec directory already exists"));
    }

    create_structure(&llmanspec_path)?;
    let config = config_with_locale(locale);
    write_config(&llmanspec_path, &config)?;
    write_project_file(&llmanspec_path, target, &config)?;
    write_agents_file(&llmanspec_path, target, &config)?;
    write_root_agents_file(target, &config)?;
    write_spec_driven_templates(&llmanspec_path, target, &config)?;

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

fn write_project_file(llmanspec_path: &Path, target: &Path, config: &SddConfig) -> Result<()> {
    let project_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Project");
    let content = render_project_template(project_name, config, target)?;
    let path = llmanspec_path.join("project.md");
    fs::write(path, content)?;
    Ok(())
}

fn write_agents_file(llmanspec_path: &Path, target: &Path, config: &SddConfig) -> Result<()> {
    let agents_path = llmanspec_path.join("AGENTS.md");
    let content = default_agents_file(config, target)?;
    fs::write(&agents_path, content)?;
    Ok(())
}

fn write_spec_driven_templates(
    llmanspec_path: &Path,
    target: &Path,
    config: &SddConfig,
) -> Result<()> {
    let template_dir = llmanspec_path.join(SPEC_DRIVEN_TEMPLATE_DIR);
    for template in spec_driven_templates(config, target)? {
        let path = template_dir.join(template.name);
        fs::write(path, template.content)?;
    }
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
