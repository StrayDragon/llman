use crate::config::resolve_config_dir;
use crate::fs_utils::{atomic_write_new_with_mode, atomic_write_with_mode};
use crate::schema_utils;
use crate::sdd::project::config::{SddConfig, llmanspec_schema};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::tool::config as tool_config;
use anyhow::{Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const SCHEMA_OUTPUT_DIR: &str = "artifacts/schema/configs/en";
pub const GLOBAL_SCHEMA_FILE: &str = "llman-config.schema.json";
pub const PROJECT_SCHEMA_FILE: &str = "llman-project-config.schema.json";
pub const LLMANSPEC_SCHEMA_FILE: &str = "llmanspec-config.schema.json";

pub const GLOBAL_SCHEMA_URL: &str = "https://raw.githubusercontent.com/StrayDragon/llman/main/artifacts/schema/configs/en/llman-config.schema.json";
pub const PROJECT_SCHEMA_URL: &str = "https://raw.githubusercontent.com/StrayDragon/llman/main/artifacts/schema/configs/en/llman-project-config.schema.json";
// LLMANSPEC_SCHEMA_URL lives with its schema owner: `sdd::project::config`.

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(
    title = "llman Global Config",
    description = "Global configuration for llman."
)]
pub struct GlobalConfig {
    #[schemars(description = "Configuration version for tool settings.")]
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Global-only settings for llman.")]
    pub skills: Option<GlobalSkillsConfig>,
    #[schemars(description = "Tool configuration.")]
    pub tools: tool_config::ToolsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(
    title = "llman Project Config",
    description = "Project-level configuration for llman. This is a subset of the global config."
)]
pub struct ProjectConfig {
    #[schemars(description = "Configuration version for tool settings.")]
    pub version: String,
    #[schemars(description = "Tool configuration.")]
    pub tools: tool_config::ToolsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
#[schemars(
    title = "llman Skills Config",
    description = "Global skills configuration. Sources are declared only via skills.repo[].",
    deny_unknown_fields
)]
pub struct GlobalSkillsConfig {
    #[schemars(
        description = "Skills repository sources. Each entry declares a local path (and optional display name). Missing or non-directory paths are skipped with a warning at startup."
    )]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repo: Vec<GlobalSkillsRepoEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(
    title = "llman Skills Repo Source",
    description = "A single skills repository source under skills.repo[]."
)]
pub struct GlobalSkillsRepoEntry {
    #[schemars(
        description = "Optional human-readable label used in the TUI. When omitted, the positional index is used as the stable id."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[schemars(
        description = "Local filesystem path to the skills repository root. Supports ~ and env vars."
    )]
    pub path: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let tool_defaults = tool_config::ToolConfig::default();
        Self {
            version: tool_defaults.version,
            tools: tool_defaults.tools,
            skills: Some(GlobalSkillsConfig {
                repo: vec![GlobalSkillsRepoEntry {
                    name: Some("default".to_string()),
                    path: "$LLMAN_CONFIG_DIR/skills".to_string(),
                }],
            }),
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        let tool_defaults = tool_config::ToolConfig::default();
        Self {
            version: tool_defaults.version,
            tools: tool_defaults.tools,
        }
    }
}

pub struct SchemaPaths {
    pub root: PathBuf,
    pub global: PathBuf,
    pub project: PathBuf,
    pub llmanspec: PathBuf,
}

pub struct SchemaArtifacts {
    pub global: String,
    pub project: String,
    pub llmanspec: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ConfigSchemaKind {
    Global,
    Project,
    Llmanspec,
}

pub enum ApplyResult {
    Updated,
    Unchanged,
    Missing,
}

pub fn schema_paths() -> SchemaPaths {
    let root = PathBuf::from(SCHEMA_OUTPUT_DIR);
    SchemaPaths {
        global: root.join(GLOBAL_SCHEMA_FILE),
        project: root.join(PROJECT_SCHEMA_FILE),
        llmanspec: root.join(LLMANSPEC_SCHEMA_FILE),
        root,
    }
}

pub fn apply_schema_header(path: &Path, schema_url: &str) -> Result<ApplyResult> {
    if !path.exists() {
        return Ok(ApplyResult::Missing);
    }
    let content = fs::read_to_string(path).map_err(|e| {
        anyhow!(t!(
            "self.schema.read_failed",
            path = path.display(),
            error = e
        ))
    })?;
    let (updated, changed) = schema_utils::apply_schema_header_to_content(&content, schema_url);
    if !changed {
        return Ok(ApplyResult::Unchanged);
    }
    atomic_write_with_mode(path, updated.as_bytes(), None).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = path.display(),
            error = e
        ))
    })?;
    Ok(ApplyResult::Updated)
}

pub fn generate_schema_artifacts() -> Result<SchemaArtifacts> {
    let global = schema_utils::generate_schema::<GlobalConfig>();
    let project = schema_utils::generate_schema::<ProjectConfig>();
    let llmanspec = llmanspec_schema();

    Ok(SchemaArtifacts {
        global: serde_json::to_string_pretty(&global)
            .map_err(|e| anyhow!(t!("self.schema.generate_failed", error = e)))?,
        project: serde_json::to_string_pretty(&project)
            .map_err(|e| anyhow!(t!("self.schema.generate_failed", error = e)))?,
        llmanspec: serde_json::to_string_pretty(&llmanspec)
            .map_err(|e| anyhow!(t!("self.schema.generate_failed", error = e)))?,
    })
}

pub fn validate_yaml_value(
    kind: ConfigSchemaKind,
    value: &serde_yaml::Value,
) -> Result<(), String> {
    match kind {
        ConfigSchemaKind::Global => {
            schema_utils::validate_yaml_value_against::<GlobalConfig>(value)
        }
        ConfigSchemaKind::Project => {
            schema_utils::validate_yaml_value_against::<ProjectConfig>(value)
        }
        ConfigSchemaKind::Llmanspec => {
            schema_utils::validate_yaml_value_against::<SddConfig>(value)
        }
    }
}

pub fn write_schema_files() -> Result<SchemaPaths> {
    let artifacts = generate_schema_artifacts()?;
    let paths = schema_paths();
    fs::create_dir_all(&paths.root).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = paths.root.display(),
            error = e
        ))
    })?;

    atomic_write_with_mode(&paths.global, artifacts.global.as_bytes(), None).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = paths.global.display(),
            error = e
        ))
    })?;
    atomic_write_with_mode(&paths.project, artifacts.project.as_bytes(), None).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = paths.project.display(),
            error = e
        ))
    })?;
    atomic_write_with_mode(&paths.llmanspec, artifacts.llmanspec.as_bytes(), None).map_err(
        |e| {
            anyhow!(t!(
                "self.schema.write_failed",
                path = paths.llmanspec.display(),
                error = e
            ))
        },
    )?;

    Ok(paths)
}

pub fn ensure_global_sample_config(config_dir: &Path) -> Result<Option<PathBuf>> {
    let path = config_dir.join("config.yaml");
    if path.exists() {
        return Ok(None);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            anyhow!(t!(
                "self.schema.write_failed",
                path = parent.display(),
                error = e
            ))
        })?;
    }

    let config = GlobalConfig::default();
    let yaml = serde_yaml::to_string(&config)
        .map_err(|e| anyhow!(t!("self.schema.generate_failed", error = e)))?;
    let content = schema_utils::prepend_schema_header(&yaml, GLOBAL_SCHEMA_URL);
    let created = atomic_write_new_with_mode(&path, content.as_bytes(), None).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = path.display(),
            error = e
        ))
    })?;
    if created { Ok(Some(path)) } else { Ok(None) }
}

pub fn global_config_path() -> Result<PathBuf> {
    Ok(resolve_config_dir(None)?.join("config.yaml"))
}

fn find_config_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if is_config_root(&current) {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn is_config_root(path: &Path) -> bool {
    has_root_marker(path, ".git")
        || has_root_marker(path, ".llman")
        || has_root_marker(path, LLMANSPEC_DIR_NAME)
}

fn has_root_marker(root: &Path, name: &str) -> bool {
    let candidate = root.join(name);
    fs::symlink_metadata(&candidate)
        .map(|meta| meta.is_dir() || meta.is_file())
        .unwrap_or(false)
}

pub fn project_config_path() -> Result<PathBuf> {
    let cwd = env::current_dir()?;
    Ok(project_config_path_from(&cwd))
}

pub fn llmanspec_config_path() -> Result<PathBuf> {
    let cwd = env::current_dir()?;
    Ok(llmanspec_config_path_from(&cwd))
}

fn project_config_path_from(cwd: &Path) -> PathBuf {
    let root = find_config_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    root.join(".llman").join("config.yaml")
}

fn llmanspec_config_path_from(cwd: &Path) -> PathBuf {
    let root = find_config_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    root.join(LLMANSPEC_DIR_NAME).join("config.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn project_and_llmanspec_paths_discover_root_from_subdir() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).expect("create nested");
        fs::create_dir_all(root.join(".git")).expect("create git dir");

        assert_eq!(
            project_config_path_from(&nested),
            root.join(".llman").join("config.yaml")
        );
        assert_eq!(
            llmanspec_config_path_from(&nested),
            root.join(LLMANSPEC_DIR_NAME).join("config.yaml")
        );
    }

    #[test]
    fn skills_config_schema_round_trips_repo() {
        let multi = GlobalSkillsConfig {
            repo: vec![
                GlobalSkillsRepoEntry {
                    name: Some("Team".to_string()),
                    path: "/team/skills".to_string(),
                },
                GlobalSkillsRepoEntry {
                    name: None,
                    path: "/personal/skills".to_string(),
                },
            ],
        };
        let yaml = serde_yaml::to_string(&multi).expect("serialize");
        let back: GlobalSkillsConfig = serde_yaml::from_str(&yaml).expect("deserialize");
        assert_eq!(back, multi);
        assert_eq!(back.repo.len(), 2);
    }

    #[test]
    fn skills_config_rejects_legacy_dir_field() {
        let err = serde_yaml::from_str::<GlobalSkillsConfig>("dir: /old/skills\n")
            .expect_err("legacy dir must be rejected");
        assert!(
            err.to_string().contains("dir") || err.to_string().contains("unknown"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn default_global_config_emits_default_repo() {
        let default = GlobalConfig::default();
        let skills = default.skills.expect("default skills section");
        assert_eq!(skills.repo.len(), 1);
        assert_eq!(skills.repo[0].name.as_deref(), Some("default"));
        assert_eq!(skills.repo[0].path, "$LLMAN_CONFIG_DIR/skills");
    }
}
