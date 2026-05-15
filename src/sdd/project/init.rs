use super::config::{SddConfig, config_with_locale, write_default_config};
use super::fs_utils::update_file_with_markers;
use super::templates::root_stub_content;
use super::update_skills::{self, UpdateSkillsArgs};
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, LLMANSPEC_MARKERS};
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
    write_default_config(&llmanspec_path, &config.locale)?;
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
