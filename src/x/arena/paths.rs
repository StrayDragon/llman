use crate::config::resolve_config_dir;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

const ARENA_DIR: &str = "arena";

#[derive(Debug, Clone)]
pub struct ArenaPaths {
    pub contests_dir: PathBuf,
    pub datasets_dir: PathBuf,
    pub runs_dir: PathBuf,
}

impl ArenaPaths {
    pub fn resolve() -> Result<Self> {
        let config_dir = resolve_config_dir(None)?;
        Ok(Self::new(&config_dir))
    }

    pub fn new(config_dir: &Path) -> Self {
        let root = config_dir.join(ARENA_DIR);
        let contests_dir = root.join("contests");
        let datasets_dir = root.join("datasets");
        let runs_dir = root.join("runs");
        Self {
            contests_dir,
            datasets_dir,
            runs_dir,
        }
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.contests_dir)?;
        fs::create_dir_all(&self.datasets_dir)?;
        fs::create_dir_all(&self.runs_dir)?;
        Ok(())
    }

    pub fn contest_path(&self, name: &str) -> PathBuf {
        self.contests_dir.join(format!("{name}.toml"))
    }

    pub fn dataset_path(&self, name: &str) -> PathBuf {
        self.datasets_dir.join(format!("{name}.yaml"))
    }

    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.runs_dir.join(run_id)
    }
}
