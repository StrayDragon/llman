use super::config::{SddConfig, load_or_create_config, write_default_config};
use super::fs_utils::update_file_with_markers;
use super::templates::root_stub_content;
use super::update_skills;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, LLMANSPEC_MARKERS};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub fn run(target: &Path, locale: Option<&str>, update: bool) -> Result<()> {
    ensure_directory(target)?;
    let llmanspec_path = target.join(LLMANSPEC_DIR_NAME);

    if update {
        if !llmanspec_path.exists() {
            return Err(anyhow!(
                "No llmanspec directory found. Run 'llman sdd init' first."
            ));
        }
    } else {
        if llmanspec_path.exists() {
            return Err(anyhow!("llmanspec directory already exists"));
        }
        create_structure(&llmanspec_path)?;
        write_default_config(&llmanspec_path, locale.unwrap_or("en"))?;
    }

    let config = load_or_create_config(&llmanspec_path)?;
    write_root_agents_file(target, &config)?;
    write_llmanspec_agents_file(&llmanspec_path)?;
    update_skills::run_with_root(target)?;

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
    let specs = llmanspec_path.join("specs");
    fs::create_dir_all(&specs)?;
    fs::write(specs.join(".gitkeep"), "")?;

    let archive = llmanspec_path.join("changes").join("archive");
    fs::create_dir_all(&archive)?;
    fs::write(archive.join(".gitkeep"), "")?;

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

fn write_llmanspec_agents_file(llmanspec_path: &Path) -> Result<()> {
    let agents_path = llmanspec_path.join("AGENTS.md");
    let content = if agents_path.exists() {
        // Preserve existing content; managed block will be updated if present
        fs::read_to_string(&agents_path)?
    } else {
        let locale = load_or_create_config(llmanspec_path)?.locale;
        let stub = match locale.as_str() {
            "zh-Hans" => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/sdd/zh-Hans/llmanspec-agents-stub.md"
            )),
            _ => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/sdd/en/llmanspec-agents-stub.md"
            )),
        };
        stub.to_string()
    };
    fs::write(&agents_path, content.as_bytes())?;
    Ok(())
}
