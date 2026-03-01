use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn write_template(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        bail!("Playbook already exists: {}", path.display());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create playbook dir {}", parent.display()))?;
    }
    fs::write(path, default_template())
        .with_context(|| format!("write playbook {}", path.display()))?;
    Ok(())
}

pub fn load_from_path(path: &Path) -> Result<PlaybookV1> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read playbook {}", path.display()))?;
    let pb: PlaybookV1 = serde_yaml::from_str(&raw).with_context(|| "parse playbook YAML")?;
    Ok(pb)
}

fn default_template() -> &'static str {
    r#"version: 1
name: demo

task:
  title: "Build a web backend service"
  prompt: |
    Build a small web backend service.
    Requirements:
    - Provide at least 3 REST APIs
    - Add basic tests
    - Include a short README

sdd_loop:
  max_iterations: 6

variants:
  - name: sdd-cc
    style: sdd
    agent:
      kind: claude-code-acp
      preset: production
      command: claude-agent-acp

  - name: legacy-codex
    style: sdd-legacy
    agent:
      kind: codex-acp
      preset: openai
      command: codex-acp

report:
  ai_judge:
    enabled: false
    model: gpt-4.1
  human:
    enabled: true
"#
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaybookV1 {
    pub version: u32,
    pub name: Option<String>,
    pub task: TaskConfig,
    #[serde(default)]
    pub sdd_loop: SddLoopConfig,
    pub variants: Vec<VariantConfig>,
    #[serde(default)]
    pub report: ReportConfig,
}

impl PlaybookV1 {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            bail!("Unsupported playbook version: {}", self.version);
        }
        if self.variants.is_empty() {
            bail!("Playbook must define at least one variant");
        }
        if self.sdd_loop.max_iterations == 0 {
            bail!("sdd_loop.max_iterations must be > 0");
        }

        let mut names = HashSet::new();
        for v in &self.variants {
            if v.name.trim().is_empty() {
                bail!("Variant name must not be empty");
            }
            if !names.insert(v.name.clone()) {
                bail!("Duplicate variant name: {}", v.name);
            }
            v.validate()
                .with_context(|| format!("validate variant {}", v.name))?;
        }

        self.task.validate().context("validate task")?;
        self.report.validate().context("validate report")?;
        Ok(())
    }

    pub fn display_name(&self, fallback: &str) -> String {
        self.name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback)
            .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskConfig {
    pub title: String,
    pub prompt: String,
}

impl TaskConfig {
    fn validate(&self) -> Result<()> {
        if self.title.trim().is_empty() {
            bail!("task.title must not be empty");
        }
        if self.prompt.trim().is_empty() {
            bail!("task.prompt must not be empty");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SddLoopConfig {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
}

fn default_max_iterations() -> u32 {
    6
}

impl Default for SddLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowStyle {
    Sdd,
    #[serde(rename = "sdd-legacy")]
    SddLegacy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantConfig {
    pub name: String,
    pub style: WorkflowStyle,
    pub agent: AgentConfig,
}

impl VariantConfig {
    fn validate(&self) -> Result<()> {
        self.agent.validate().context("validate agent")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentKind {
    #[serde(rename = "claude-code-acp")]
    ClaudeCode,
    #[serde(rename = "codex-acp")]
    Codex,
    #[serde(rename = "fake-acp")]
    Fake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    pub kind: AgentKind,
    /// Preset/group name from `llman x cc` or `llman x codex` (depending on kind).
    pub preset: String,
    /// Agent process executable name/path (default depends on kind).
    pub command: Option<String>,
    /// Extra process args for the agent.
    #[serde(default)]
    pub args: Vec<String>,
}

impl AgentConfig {
    fn validate(&self) -> Result<()> {
        if self.preset.trim().is_empty() {
            bail!("agent.preset must not be empty");
        }
        if let Some(cmd) = &self.command
            && cmd.trim().is_empty()
        {
            bail!("agent.command must not be empty when provided");
        }
        Ok(())
    }

    pub fn command_or_default(&self) -> Result<String> {
        if let Some(cmd) = &self.command {
            return Ok(cmd.clone());
        }
        match self.kind {
            AgentKind::ClaudeCode => Ok("claude-agent-acp".to_string()),
            AgentKind::Codex => Ok("codex-acp".to_string()),
            AgentKind::Fake => Ok("llman-fake-acp-agent".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ReportConfig {
    #[serde(default)]
    pub ai_judge: AiJudgeConfig,
    #[serde(default)]
    pub human: HumanConfig,
}

impl ReportConfig {
    fn validate(&self) -> Result<()> {
        self.ai_judge
            .validate()
            .context("validate report.ai_judge")?;
        self.human.validate().context("validate report.human")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AiJudgeConfig {
    #[serde(default)]
    pub enabled: bool,
    pub model: Option<String>,
}

impl AiJudgeConfig {
    fn validate(&self) -> Result<()> {
        if self.enabled {
            let Some(model) = self
                .model
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                bail!("report.ai_judge.model is required when enabled");
            };
            if model.is_empty() {
                return Err(anyhow!("report.ai_judge.model must not be empty"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for HumanConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
        }
    }
}

impl HumanConfig {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

pub fn playbook_file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "run".to_string())
}

pub fn normalize_playbook_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
