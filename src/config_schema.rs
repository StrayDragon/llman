use crate::config::resolve_config_dir;
use crate::sdd::LLMANSPEC_DIR_NAME;
use crate::sdd::SddConfig;
use crate::tool::config as tool_config;
use anyhow::{Result, anyhow};
use jsonschema::JSONSchema;
use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
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
pub const LLMANSPEC_SCHEMA_URL: &str = "https://raw.githubusercontent.com/StrayDragon/llman/main/artifacts/schema/configs/en/llmanspec-config.schema.json";

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
    pub tools: tool_config::ToolsConfig,
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
    pub tools: tool_config::ToolsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(
    title = "llman Skills Config",
    description = "Global skills configuration."
)]
pub struct GlobalSkillsConfig {
    #[schemars(description = "Override skills root directory. Supports ~ and env vars.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let tool_defaults = tool_config::Config::default();
        Self {
            version: tool_defaults.version,
            tools: tool_defaults.tools,
            skills: Some(GlobalSkillsConfig {
                dir: Some("$LLMAN_CONFIG_DIR/skills".to_string()),
            }),
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        let tool_defaults = tool_config::Config::default();
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

const SCHEMA_ERROR_LIMIT: usize = 5;

pub fn schema_paths() -> SchemaPaths {
    let root = PathBuf::from(SCHEMA_OUTPUT_DIR);
    SchemaPaths {
        global: root.join(GLOBAL_SCHEMA_FILE),
        project: root.join(PROJECT_SCHEMA_FILE),
        llmanspec: root.join(LLMANSPEC_SCHEMA_FILE),
        root,
    }
}

pub fn schema_header_line(schema_url: &str) -> String {
    format!("# yaml-language-server: $schema={schema_url}")
}

pub fn prepend_schema_header(content: &str, schema_url: &str) -> String {
    let header = schema_header_line(schema_url);
    if content.is_empty() {
        return format!("{header}\n");
    }
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    format!("{header}{newline}{content}")
}

pub fn apply_schema_header_to_content(content: &str, schema_url: &str) -> (String, bool) {
    let header = schema_header_line(schema_url);
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let has_trailing = content.ends_with('\n') || content.ends_with("\r\n");
    let mut lines: Vec<String> = Vec::new();
    let mut header_count = 0;
    let mut header_at_top = false;

    for (index, line) in content.lines().enumerate() {
        if line
            .trim_start()
            .starts_with("# yaml-language-server: $schema=")
        {
            header_count += 1;
            if index == 0 && line.trim() == header {
                header_at_top = true;
            }
            continue;
        }
        lines.push(line.to_string());
    }

    if header_at_top && header_count == 1 {
        return (content.to_string(), false);
    }

    let mut out_lines = Vec::with_capacity(lines.len() + 1);
    out_lines.push(header);
    out_lines.extend(lines);
    let mut updated = out_lines.join(newline);
    if has_trailing {
        updated.push_str(newline);
    }
    (updated, true)
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
    let (updated, changed) = apply_schema_header_to_content(&content, schema_url);
    if !changed {
        return Ok(ApplyResult::Unchanged);
    }
    fs::write(path, updated).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = path.display(),
            error = e
        ))
    })?;
    Ok(ApplyResult::Updated)
}

pub fn generate_schema_artifacts() -> Result<SchemaArtifacts> {
    let global = generate_schema::<GlobalConfig>();
    let project = generate_schema::<ProjectConfig>();
    let llmanspec = generate_schema::<SddConfig>();

    Ok(SchemaArtifacts {
        global: serde_json::to_string_pretty(&global)
            .map_err(|e| anyhow!(t!("self.schema.generate_failed", error = e)))?,
        project: serde_json::to_string_pretty(&project)
            .map_err(|e| anyhow!(t!("self.schema.generate_failed", error = e)))?,
        llmanspec: serde_json::to_string_pretty(&llmanspec)
            .map_err(|e| anyhow!(t!("self.schema.generate_failed", error = e)))?,
    })
}

fn generate_schema<T: JsonSchema>() -> schemars::Schema {
    let mut settings = SchemaSettings::draft07();
    settings.inline_subschemas = true;
    settings.into_generator().into_root_schema_for::<T>()
}

pub fn validate_yaml_value(
    kind: ConfigSchemaKind,
    value: &serde_yaml::Value,
) -> Result<(), String> {
    let json_value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    let schema_value = schema_value_for_kind(kind)?;
    let compiled = JSONSchema::compile(&schema_value).map_err(|e| e.to_string())?;
    if let Err(errors) = compiled.validate(&json_value) {
        return Err(format_schema_errors(errors.map(|err| err.to_string())));
    }
    Ok(())
}

pub fn format_schema_errors<I>(errors: I) -> String
where
    I: IntoIterator<Item = String>,
{
    let mut iter = errors.into_iter();
    let mut items = Vec::new();
    for _ in 0..SCHEMA_ERROR_LIMIT {
        if let Some(err) = iter.next() {
            items.push(err);
        } else {
            break;
        }
    }
    let remaining = iter.count();
    if items.is_empty() {
        return "unknown".to_string();
    }
    let mut message = items.join("; ");
    if remaining > 0 {
        message.push_str(&format!("; ... (+{remaining} more)"));
    }
    message
}

fn schema_value_for_kind(kind: ConfigSchemaKind) -> Result<serde_json::Value, String> {
    let schema = match kind {
        ConfigSchemaKind::Global => generate_schema::<GlobalConfig>(),
        ConfigSchemaKind::Project => generate_schema::<ProjectConfig>(),
        ConfigSchemaKind::Llmanspec => generate_schema::<SddConfig>(),
    };
    serde_json::to_value(&schema).map_err(|e| e.to_string())
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

    fs::write(&paths.global, artifacts.global).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = paths.global.display(),
            error = e
        ))
    })?;
    fs::write(&paths.project, artifacts.project).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = paths.project.display(),
            error = e
        ))
    })?;
    fs::write(&paths.llmanspec, artifacts.llmanspec).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = paths.llmanspec.display(),
            error = e
        ))
    })?;

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
    let content = prepend_schema_header(&yaml, GLOBAL_SCHEMA_URL);
    fs::write(&path, content).map_err(|e| {
        anyhow!(t!(
            "self.schema.write_failed",
            path = path.display(),
            error = e
        ))
    })?;
    Ok(Some(path))
}

pub fn global_config_path() -> Result<PathBuf> {
    Ok(resolve_config_dir(None)?.join("config.yaml"))
}

pub fn project_config_path() -> Result<PathBuf> {
    Ok(env::current_dir()?.join(".llman").join("config.yaml"))
}

pub fn llmanspec_config_path() -> Result<PathBuf> {
    Ok(env::current_dir()?
        .join(LLMANSPEC_DIR_NAME)
        .join("config.yaml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_schema_header_inserts_before_doc_start() {
        let content = "---\nversion: \"0.1\"\n";
        let (updated, changed) = apply_schema_header_to_content(content, GLOBAL_SCHEMA_URL);
        assert!(changed);
        assert!(updated.starts_with("# yaml-language-server: $schema="));
        assert!(updated.contains("\n---\n"));
    }

    #[test]
    fn apply_schema_header_replaces_existing() {
        let content =
            "# yaml-language-server: $schema=https://example.com/old.json\nversion: \"0.1\"\n";
        let (updated, changed) = apply_schema_header_to_content(content, GLOBAL_SCHEMA_URL);
        assert!(changed);
        assert!(updated.starts_with(&schema_header_line(GLOBAL_SCHEMA_URL)));
        assert!(!updated.contains("old.json"));
    }
}
