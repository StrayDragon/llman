use anyhow::{Context, Result, anyhow, bail};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

pub fn load_from_path(path: &Path) -> Result<Playbook> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read playbook {}", path.display()))?;
    load_from_str(&raw, &format!("playbook {}", path.display()))
}

pub fn load_from_str(content: &str, context: &str) -> Result<Playbook> {
    let value: serde_yaml::Value =
        serde_yaml::from_str(content).with_context(|| format!("{context}: parse YAML"))?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| anyhow!("{context}: playbook MUST be a YAML mapping"))?;

    if mapping.contains_key("version") {
        bail!(
            "{context}: legacy sdd-eval playbook detected (top-level `version`). \
The playbook format has been replaced by a workflow/jobs/steps DSL. \
Re-generate a new template via `llman x sdd-eval init` and port your content to the new format."
        );
    }

    ensure_mapping_field(mapping, "variants", context)?;
    ensure_nested_mapping_field(mapping, &["workflow", "jobs"], context)?;

    let pb: Playbook =
        serde_yaml::from_value(value).with_context(|| format!("{context}: decode playbook"))?;
    pb.validate().with_context(|| format!("{context}: validate playbook"))?;
    Ok(pb)
}

fn ensure_mapping_field(
    mapping: &serde_yaml::Mapping,
    key: &str,
    context: &str,
) -> Result<()> {
    let Some(value) = mapping.get(key) else {
        return Err(anyhow!("{context}: `{}` is required", key));
    };
    if !value.is_mapping() {
        bail!("{context}: `{key}` MUST be a YAML mapping");
    }
    Ok(())
}

fn ensure_nested_mapping_field(
    mapping: &serde_yaml::Mapping,
    path: &[&str],
    context: &str,
) -> Result<()> {
    let mut cur = serde_yaml::Value::Mapping(mapping.clone());
    let mut full = String::new();
    for (idx, key) in path.iter().enumerate() {
        if !full.is_empty() {
            full.push('.');
        }
        full.push_str(key);

        let Some(next) = cur
            .as_mapping()
            .and_then(|m| m.get(*key))
            .cloned()
        else {
            if idx == path.len() - 1 {
                bail!("{context}: `{}` is required", full);
            }
            bail!("{context}: `{}` is required", full);
        };
        if !next.is_mapping() {
            bail!("{context}: `{}` MUST be a YAML mapping", full);
        }
        cur = next;
    }
    Ok(())
}

fn default_template() -> &'static str {
    // Keep this template in sync with the schema instance used by `llman self schema check`.
    r#"# yaml-language-server: $schema=https://raw.githubusercontent.com/StrayDragon/llman/main/artifacts/schema/playbooks/en/llman-sdd-eval.schema.json
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
  sdd-cc:
    style: sdd
    agent:
      kind: claude-code-acp
      preset: production
      command: claude-agent-acp

  sdd-legacy-codex:
    style: sdd-legacy
    agent:
      kind: codex-acp
      preset: openai
      command: codex-acp

workflow:
  jobs:
    eval:
      strategy:
        matrix:
          variant: [sdd-cc, sdd-legacy-codex]
      steps:
        - uses: builtin:sdd-eval/workspace.prepare
        - uses: builtin:sdd-eval/sdd.prepare
        - uses: builtin:sdd-eval/acp.sdd-loop

    report:
      needs: [eval]
      steps:
        - uses: builtin:sdd-eval/report.generate

report:
  ai_judge:
    enabled: false
    model: gpt-4.1
  human:
    enabled: true
"#
}

pub(crate) fn default_template_yaml() -> &'static str {
    default_template()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    title = "llman SDD Eval Playbook",
    description = "Playbook for `llman x sdd-eval` (workflow/jobs/steps DSL)."
)]
pub struct Playbook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Optional human-readable playbook name.")]
    pub name: Option<String>,

    #[schemars(description = "Shared evaluation task definition.")]
    pub task: TaskConfig,

    #[schemars(
        description = "Variants keyed by stable variant_id. Used by workflow matrix expansion."
    )]
    pub variants: IndexMap<String, VariantConfig>,

    #[schemars(description = "Workflow definition (jobs/steps).")]
    pub workflow: WorkflowConfig,

    #[serde(default)]
    #[schemars(description = "Default loop configuration for built-in actions.")]
    pub sdd_loop: SddLoopConfig,

    #[serde(default)]
    #[schemars(description = "Report configuration.")]
    pub report: ReportConfig,
}

impl Playbook {
    pub fn validate(&self) -> Result<()> {
        if self.variants.is_empty() {
            bail!("Playbook must define at least one variant");
        }
        if self.sdd_loop.max_iterations == 0 {
            bail!("sdd_loop.max_iterations must be > 0");
        }

        for (variant_id, variant) in &self.variants {
            if !is_safe_id(variant_id) {
                bail!(
                    "Invalid variant id `{}` (expected pattern: ^[a-zA-Z][a-zA-Z0-9_-]*$)",
                    variant_id
                );
            }
            variant
                .validate()
                .with_context(|| format!("validate variant `{}`", variant_id))?;
        }

        self.workflow.validate(&self.variants)?;
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Task", description = "Evaluation task shared by all variants.")]
pub struct TaskConfig {
    #[schemars(description = "Short human-readable task title.")]
    pub title: String,
    #[schemars(description = "Full task prompt presented to the agent.")]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    title = "Workflow",
    description = "Workflow definition containing jobs and steps."
)]
pub struct WorkflowConfig {
    #[schemars(description = "Job map keyed by stable job_id (YAML order is significant).")]
    pub jobs: IndexMap<String, WorkflowJob>,
}

impl WorkflowConfig {
    fn validate(&self, variants: &IndexMap<String, VariantConfig>) -> Result<()> {
        if self.jobs.is_empty() {
            bail!("workflow.jobs is required and must not be empty");
        }

        for (job_id, job) in &self.jobs {
            if !is_safe_id(job_id) {
                bail!(
                    "Invalid job id `{}` (expected pattern: ^[a-zA-Z][a-zA-Z0-9_-]*$)",
                    job_id
                );
            }
            job.validate(variants)
                .with_context(|| format!("validate job `{}`", job_id))?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Job", description = "Workflow job definition.")]
pub struct WorkflowJob {
    #[serde(default)]
    #[schemars(description = "Job dependencies by job_id.")]
    pub needs: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Optional job strategy (matrix).")]
    pub strategy: Option<JobStrategy>,

    #[schemars(description = "Ordered steps for the job (must be non-empty).")]
    pub steps: Vec<JobStep>,
}

impl WorkflowJob {
    fn validate(&self, variants: &IndexMap<String, VariantConfig>) -> Result<()> {
        if self.steps.is_empty() {
            bail!("steps must be non-empty");
        }
        for (idx, step) in self.steps.iter().enumerate() {
            step.validate().with_context(|| format!("validate step {}", idx + 1))?;
        }

        if let Some(strategy) = &self.strategy {
            if let Some(matrix) = &strategy.matrix {
                if matrix.variant.is_empty() {
                    bail!("strategy.matrix.variant must be non-empty when strategy.matrix is present");
                }
                for variant_id in &matrix.variant {
                    if !variants.contains_key(variant_id) {
                        bail!("strategy.matrix.variant references unknown variant id `{}`", variant_id);
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Strategy", description = "Job strategy (matrix expansion).")]
pub struct JobStrategy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Matrix definition (limited to variants in v1).")]
    pub matrix: Option<JobMatrix>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Matrix", description = "Matrix definition for job expansion.")]
pub struct JobMatrix {
    #[serde(default)]
    #[schemars(description = "Variant ids to expand (serial, ordered).")]
    pub variant: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Step", description = "Workflow step (`uses` or `run`).")]
pub struct JobStep {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Optional human-readable step name.")]
    pub name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Built-in action identifier (e.g. builtin:sdd-eval/workspace.prepare).")]
    pub uses: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Inputs for the action (v1: no keys; may be omitted or `{}`).")]
    pub with: Option<BuiltinWithV1>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Run a single allowlisted command (no shell).")]
    pub run: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Optional working directory (relative to sandbox root).")]
    pub cwd: Option<String>,
}

impl JobStep {
    fn validate(&self) -> Result<()> {
        match (self.uses.as_ref(), self.run.as_ref()) {
            (Some(_), None) => {
                if self.cwd.is_some() {
                    bail!("`cwd` is not allowed for `uses` steps");
                }
            }
            (None, Some(run)) => {
                if self.with.is_some() {
                    bail!("`with` is not allowed for `run` steps");
                }
                if run.trim().is_empty() {
                    bail!("`run` must not be empty");
                }
            }
            (Some(_), Some(_)) => bail!("step must specify exactly one of `uses` or `run`"),
            (None, None) => bail!("step must specify exactly one of `uses` or `run`"),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Builtin With", description = "Built-in action inputs (v1: empty).")]
pub struct BuiltinWithV1 {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    title = "Variant",
    description = "Variant configuration (workflow style + ACP agent preset)."
)]
pub struct VariantConfig {
    #[schemars(description = "Workflow style for the variant.")]
    pub style: WorkflowStyle,
    #[schemars(description = "ACP agent configuration for the variant.")]
    pub agent: AgentConfig,
}

impl VariantConfig {
    fn validate(&self) -> Result<()> {
        self.agent.validate().context("validate agent")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(title = "Workflow Style", description = "Workflow style selector.")]
pub enum WorkflowStyle {
    #[serde(rename = "sdd")]
    Sdd,
    #[serde(rename = "sdd-legacy")]
    SddLegacy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(title = "Agent Kind", description = "ACP agent kind.")]
pub enum AgentKind {
    #[serde(rename = "claude-code-acp")]
    ClaudeCode,
    #[serde(rename = "codex-acp")]
    Codex,
    #[serde(rename = "fake-acp")]
    Fake,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Agent", description = "ACP agent config for a variant.")]
pub struct AgentConfig {
    #[schemars(description = "ACP agent kind.")]
    pub kind: AgentKind,
    #[schemars(description = "Preset/group name from `llman x cc` or `llman x codex`.")]
    pub preset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Agent executable name/path (optional; defaults depend on kind).")]
    pub command: Option<String>,
    #[serde(default)]
    #[schemars(description = "Extra process args for the agent.")]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "SDD Loop", description = "Loop configuration for sdd-eval.")]
pub struct SddLoopConfig {
    #[serde(default = "default_max_iterations")]
    #[schemars(description = "Maximum iterations for the ACP SDD loop (must be > 0).")]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Report", description = "Report configuration for a run.")]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
#[schemars(title = "AI Judge", description = "Optional AI judge configuration.")]
pub struct AiJudgeConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Human Report", description = "Human scoring export settings.")]
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

fn is_safe_id(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
