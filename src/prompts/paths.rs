use crate::config::{home_dir, try_home_dir};
use crate::skills::shared::git::find_git_root;
use anyhow::{Result, anyhow};
use inquire::Confirm;
use std::env;
use std::path::{Path, PathBuf};

pub fn cwd() -> Result<PathBuf> {
    Ok(env::current_dir()?)
}

pub fn ensure_not_home_dir(current_dir: &Path) -> Result<()> {
    if let Some(home_dir) = try_home_dir()
        && current_dir == home_dir
    {
        return Err(anyhow!(t!("errors.home_directory_not_allowed")));
    }
    Ok(())
}

pub fn project_root(current_dir: &Path, force: bool, interactive: bool) -> Result<Option<PathBuf>> {
    ensure_not_home_dir(current_dir)?;

    if let Some(root) = find_git_root(current_dir) {
        return Ok(Some(root));
    }

    if force {
        return Ok(Some(current_dir.to_path_buf()));
    }

    if interactive {
        let prompt = t!("interactive.project_root_force_prompt");
        let confirmed = Confirm::new(&prompt)
            .with_default(false)
            .prompt()
            .map_err(|e| anyhow!(t!("errors.interactive_prompt_error", error = e)))?;
        if confirmed {
            return Ok(Some(current_dir.to_path_buf()));
        }
        println!("{}", t!("messages.operation_cancelled"));
        return Ok(None);
    }

    Err(anyhow!(t!("errors.project_scope_requires_repo")))
}

pub fn codex_home_dir() -> Result<PathBuf> {
    if let Ok(home) = env::var("CODEX_HOME") {
        return Ok(PathBuf::from(home));
    }
    Ok(home_dir()?.join(".codex"))
}

pub fn claude_home_dir() -> Result<PathBuf> {
    if let Ok(home) = env::var("CLAUDE_HOME") {
        return Ok(PathBuf::from(home));
    }
    Ok(home_dir()?.join(".claude"))
}
