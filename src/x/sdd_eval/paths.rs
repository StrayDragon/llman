use crate::skills::shared::git::find_git_root;
use anyhow::{Result, anyhow};
use std::env;
use std::path::{Path, PathBuf};

pub fn project_root_from_cwd() -> Result<PathBuf> {
    let cwd = env::current_dir()?;

    if let Some(home) = crate::config::try_home_dir()
        && cwd == home
    {
        return Err(anyhow!("Refusing to run in home directory."));
    }

    Ok(find_git_root(&cwd).unwrap_or(cwd))
}

pub fn eval_root(project_root: &Path) -> PathBuf {
    project_root.join(".llman").join("sdd-eval")
}

pub fn playbooks_dir(project_root: &Path) -> PathBuf {
    eval_root(project_root).join("playbooks")
}

pub fn playbook_path(project_root: &Path, name: &str) -> PathBuf {
    playbooks_dir(project_root).join(format!("{name}.yaml"))
}

pub fn runs_dir(project_root: &Path) -> PathBuf {
    eval_root(project_root).join("runs")
}

pub fn run_dir(project_root: &Path, run_id: &str) -> PathBuf {
    runs_dir(project_root).join(run_id)
}
