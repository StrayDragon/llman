use crate::config::{CLAUDE_CODE_APP, CODEX_APP, Config as LlmanConfig};
use crate::path_utils::safe_parent_for_creation;
use crate::x::arena::paths::ArenaPaths;
use anyhow::{Result, anyhow};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Args, Debug, Clone)]
pub struct ContestInitArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestConfigV1 {
    pub version: u32,
    pub name: String,
    pub app: String,
    pub models: Vec<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub top_k: Option<i32>,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: i32,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_retries")]
    pub retries: u32,
    #[serde(default)]
    pub structured_output: bool,
    #[serde(default = "default_repair_retries")]
    pub repair_retries: u32,
    #[serde(default)]
    pub verify: Vec<String>,
    pub prompts: Vec<PromptVariantConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariantConfig {
    pub id: String,
    pub prompt_name: String,
}

fn default_temperature() -> f32 {
    0.0
}

fn default_max_output_tokens() -> i32 {
    2000
}

fn default_timeout_secs() -> u64 {
    180
}

fn default_retries() -> u32 {
    2
}

fn default_repair_retries() -> u32 {
    0
}

pub fn run_init(args: &ContestInitArgs) -> Result<()> {
    validate_name(&args.name)?;
    let paths = ArenaPaths::resolve()?;
    paths.ensure_dirs()?;

    let path = paths.contest_path(&args.name);
    if path.exists() && !args.force {
        return Err(anyhow!(
            "Contest already exists: {} (use --force to overwrite)",
            path.display()
        ));
    }

    if let Some(parent) = safe_parent_for_creation(&path) {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, contest_template(&args.name))?;
    println!("✅ Wrote contest template: {}", path.display());
    Ok(())
}

pub fn load_by_name(name: &str) -> Result<ContestConfigV1> {
    validate_name(name)?;
    let paths = ArenaPaths::resolve()?;
    let path = paths.contest_path(name);
    let content = fs::read_to_string(&path)
        .map_err(|e| anyhow!("Failed to read contest {}: {}", path.display(), e))?;
    let cfg: ContestConfigV1 =
        toml::from_str(&content).map_err(|e| anyhow!("Invalid contest TOML: {e}"))?;
    validate_contest(&cfg)?;
    Ok(cfg)
}

pub fn validate_contest(cfg: &ContestConfigV1) -> Result<()> {
    if cfg.version != 1 {
        return Err(anyhow!(
            "Unsupported contest version: {} (expected 1)",
            cfg.version
        ));
    }
    if cfg.name.trim().is_empty() {
        return Err(anyhow!("Contest name is required"));
    }
    if cfg.app != CODEX_APP && cfg.app != CLAUDE_CODE_APP {
        return Err(anyhow!(
            "Invalid contest app: {} (expected {} or {})",
            cfg.app,
            CODEX_APP,
            CLAUDE_CODE_APP
        ));
    }
    if cfg.models.is_empty() {
        return Err(anyhow!("Contest models must be non-empty"));
    }
    if cfg.prompts.is_empty() {
        return Err(anyhow!("Contest prompts must be non-empty"));
    }
    for p in &cfg.prompts {
        if p.id.trim().is_empty() {
            return Err(anyhow!("Prompt id is required"));
        }
        if p.prompt_name.trim().is_empty() {
            return Err(anyhow!("Prompt name is required"));
        }
    }
    if let Some(top_p) = cfg.top_p
        && !(0.0..=1.0).contains(&top_p)
    {
        return Err(anyhow!("Invalid contest top_p: {top_p} (expected 0..=1)"));
    }
    if let Some(top_k) = cfg.top_k
        && top_k <= 0
    {
        return Err(anyhow!("Invalid contest top_k: {top_k} (expected > 0)"));
    }
    if cfg.max_output_tokens <= 0 {
        return Err(anyhow!(
            "Invalid contest max_output_tokens: {} (expected > 0)",
            cfg.max_output_tokens
        ));
    }
    if cfg.timeout_secs == 0 {
        return Err(anyhow!("Invalid contest timeout_secs: 0 (expected > 0)"));
    }
    // Ensure referenced prompts exist (best-effort validation up front).
    let llman = LlmanConfig::new()?;
    for p in &cfg.prompts {
        let path = llman.rule_file_path(&cfg.app, &p.prompt_name);
        if !path.exists() {
            return Err(anyhow!(
                "Prompt not found for app={}: {} (expected file: {})",
                cfg.app,
                p.prompt_name,
                path.display()
            ));
        }
    }
    Ok(())
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

fn contest_template(name: &str) -> String {
    format!(
        r#"version = 1
name = "{name}"

# Which prompt store to use: "codex" or "claude-code"
app = "{CODEX_APP}"

# Models to participate (copy/paste from: `llman x arena models pick`)
models = ["gpt-4o-mini"]

temperature = 0.0
top_p = 1.0
max_output_tokens = 2000
timeout_secs = 180
retries = 2

# Prefer structured output (JSON schema) for more reliable parsing.
# (Some OpenAI-compatible endpoints may not support this.)
structured_output = false

# If output is invalid (e.g. JSON parse / diff apply failure), retry with error feedback.
repair_retries = 0

# Default verification commands for `repo` tasks (task-level verify can override)
verify = ["just test"]

[[prompts]]
id = "p1"
prompt_name = "draftpr"

[[prompts]]
id = "p2"
prompt_name = "strict"
"#
    )
}
