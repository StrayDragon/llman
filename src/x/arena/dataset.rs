use crate::path_utils::safe_parent_for_creation;
use crate::x::arena::paths::ArenaPaths;
use anyhow::{Result, anyhow};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Args, Debug, Clone)]
pub struct DatasetInitArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfigV1 {
    pub version: u32,
    pub name: String,
    pub repo_template_path: Option<PathBuf>,
    pub tasks: Vec<TaskConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: TaskKind,
    pub prompt: String,
    #[serde(default)]
    pub rubric: Option<String>,
    #[serde(default)]
    pub verify: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskKind {
    Text,
    Repo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: TaskKind,
    pub prompt: String,
    pub rubric: Option<String>,
    pub verify: Option<Vec<String>>,
}

impl From<&TaskConfig> for TaskSnapshot {
    fn from(value: &TaskConfig) -> Self {
        Self {
            id: value.id.clone(),
            kind: value.kind,
            prompt: value.prompt.clone(),
            rubric: value.rubric.clone(),
            verify: value.verify.clone(),
        }
    }
}

pub fn run_init(args: &DatasetInitArgs) -> Result<()> {
    validate_name(&args.name)?;
    let paths = ArenaPaths::resolve()?;
    paths.ensure_dirs()?;

    let path = paths.dataset_path(&args.name);
    if path.exists() && !args.force {
        return Err(anyhow!(
            "Dataset already exists: {} (use --force to overwrite)",
            path.display()
        ));
    }

    if let Some(parent) = safe_parent_for_creation(&path) {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, dataset_template(&args.name))?;
    println!("✅ Wrote dataset template: {}", path.display());
    Ok(())
}

pub fn load_by_name(name: &str) -> Result<DatasetConfigV1> {
    validate_name(name)?;
    let paths = ArenaPaths::resolve()?;
    let path = paths.dataset_path(name);
    let content = fs::read_to_string(&path)
        .map_err(|e| anyhow!("Failed to read dataset {}: {}", path.display(), e))?;
    let cfg: DatasetConfigV1 =
        serde_yaml::from_str(&content).map_err(|e| anyhow!("Invalid dataset YAML: {e}"))?;
    validate_dataset(&cfg)?;
    Ok(cfg)
}

pub fn validate_dataset(cfg: &DatasetConfigV1) -> Result<()> {
    if cfg.version != 1 {
        return Err(anyhow!(
            "Unsupported dataset version: {} (expected 1)",
            cfg.version
        ));
    }
    if cfg.name.trim().is_empty() {
        return Err(anyhow!("Dataset name is required"));
    }
    if cfg.tasks.is_empty() {
        return Err(anyhow!("Dataset tasks must be non-empty"));
    }
    for task in &cfg.tasks {
        if task.id.trim().is_empty() {
            return Err(anyhow!("Task id is required"));
        }
        if task.prompt.trim().is_empty() {
            return Err(anyhow!("Task prompt is required (task id: {})", task.id));
        }
    }
    Ok(())
}

pub fn requires_repo_template(cfg: &DatasetConfigV1) -> bool {
    cfg.tasks.iter().any(|t| t.kind == TaskKind::Repo)
}

fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(anyhow!("name is required"));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(anyhow!("invalid name: {name}"));
    }
    Ok(())
}

fn dataset_template(name: &str) -> String {
    format!(
        r#"version: 1
name: {name}

# Required when the dataset includes any `repo` tasks.
# Arena will copy this directory into a temp workspace for each generation.
repo_template_path: /path/to/repo-template

tasks:
  - id: t_text_1
    type: text
    prompt: |
      Write a concise answer to this question.
    rubric: |
      Prefer correctness and actionable steps.

  - id: t_repo_1
    type: repo
    prompt: |
      Produce a unified diff that updates the repo to satisfy the requested change.
      Output ONLY the diff (no commentary).
    verify:
      - just test
"#
    )
}
