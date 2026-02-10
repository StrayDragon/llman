use crate::config_schema::{
    ConfigSchemaKind, LLMANSPEC_SCHEMA_URL, prepend_schema_header, validate_yaml_value,
};
use crate::sdd::shared::constants::LLMANSPEC_CONFIG_FILE;
use anyhow::{Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Skills output paths used by llman SDD.")]
pub struct SkillsConfig {
    #[serde(default = "default_claude_path")]
    #[schemars(
        description = "Path for generated Claude Code skills (relative to llmanspec root)."
    )]
    pub claude_path: String,
    #[serde(default = "default_codex_path")]
    #[schemars(description = "Path for generated Codex skills (relative to llmanspec root).")]
    pub codex_path: String,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            claude_path: default_claude_path(),
            codex_path: default_codex_path(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "SDD project configuration for llmanspec.")]
pub struct SddConfig {
    #[serde(default = "default_version")]
    #[schemars(description = "Configuration schema version.")]
    pub version: u32,
    #[serde(default = "default_locale")]
    #[schemars(description = "Locale used for SDD templates and skills.")]
    pub locale: String,
    #[serde(default)]
    #[schemars(description = "SDD skills output configuration.")]
    pub skills: SkillsConfig,
}

impl Default for SddConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            locale: default_locale(),
            skills: SkillsConfig::default(),
        }
    }
}

fn default_version() -> u32 {
    CONFIG_VERSION
}

fn default_locale() -> String {
    "en".to_string()
}

fn default_claude_path() -> String {
    ".claude/skills".to_string()
}

fn default_codex_path() -> String {
    ".codex/skills".to_string()
}

pub fn config_path(llmanspec_dir: &Path) -> PathBuf {
    llmanspec_dir.join(LLMANSPEC_CONFIG_FILE)
}

pub fn config_with_locale(locale: Option<&str>) -> SddConfig {
    let mut config = SddConfig::default();
    if let Some(locale) = locale {
        config.locale = normalize_locale(locale);
    }
    config
}

pub fn load_config(llmanspec_dir: &Path) -> Result<Option<SddConfig>> {
    let path = config_path(llmanspec_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .map_err(|err| anyhow!(t!("sdd.config.read_failed", error = err)))?;
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|err| anyhow!(t!("sdd.config.parse_failed", error = err)))?;
    if let Err(error) = validate_yaml_value(ConfigSchemaKind::Llmanspec, &yaml_value) {
        return Err(anyhow!(t!(
            "sdd.config.schema_invalid",
            path = path.display(),
            error = error
        )));
    }
    let config: SddConfig = serde_yaml::from_value(yaml_value)
        .map_err(|err| anyhow!(t!("sdd.config.parse_failed", error = err)))?;
    if config.version != CONFIG_VERSION {
        return Err(anyhow!(t!(
            "sdd.config.unsupported_version",
            version = config.version
        )));
    }
    Ok(Some(SddConfig {
        version: CONFIG_VERSION,
        locale: normalize_locale(&config.locale),
        skills: config.skills,
    }))
}

pub fn load_or_create_config(llmanspec_dir: &Path) -> Result<SddConfig> {
    match load_config(llmanspec_dir)? {
        Some(config) => Ok(config),
        None => {
            let config = SddConfig::default();
            write_config(llmanspec_dir, &config)?;
            Ok(config)
        }
    }
}

pub fn write_config(llmanspec_dir: &Path, config: &SddConfig) -> Result<()> {
    let path = config_path(llmanspec_dir);
    let content = serde_yaml::to_string(config)
        .map_err(|err| anyhow!(t!("sdd.config.serialize_failed", error = err)))?;
    let content = prepend_schema_header(&content, LLMANSPEC_SCHEMA_URL);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content).map_err(|err| anyhow!(t!("sdd.config.write_failed", error = err)))?;
    Ok(())
}

pub fn normalize_locale(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return default_locale();
    }
    let lower = trimmed.to_lowercase();
    if lower == "zh" || lower.starts_with("zh-hans") || lower == "zh-cn" {
        return "zh-Hans".to_string();
    }
    if lower.starts_with("en") {
        return "en".to_string();
    }
    trimmed.to_string()
}

pub fn locale_fallbacks(locale: &str) -> Vec<String> {
    let normalized = normalize_locale(locale);
    let mut seen = HashSet::new();
    let mut locales = Vec::new();

    push_unique(&mut locales, &mut seen, normalized.clone());
    if let Some((lang, _)) = normalized.split_once('-') {
        push_unique(&mut locales, &mut seen, lang.to_string());
    }
    push_unique(&mut locales, &mut seen, "en".to_string());

    locales
}

fn push_unique(locales: &mut Vec<String>, seen: &mut HashSet<String>, value: String) {
    if seen.insert(value.clone()) {
        locales.push(value);
    }
}

pub fn resolve_skill_path(root: &Path, path: &str) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn normalize_locale_defaults_to_en() {
        assert_eq!(normalize_locale(""), "en");
        assert_eq!(normalize_locale("en-US"), "en");
        assert_eq!(normalize_locale("zh"), "zh-Hans");
        assert_eq!(normalize_locale("zh-cn"), "zh-Hans");
    }

    #[test]
    fn locale_fallbacks_include_en() {
        let fallbacks = locale_fallbacks("zh-Hans");
        assert_eq!(fallbacks, vec!["zh-Hans", "zh", "en"]);
        let fallbacks = locale_fallbacks("en");
        assert_eq!(fallbacks, vec!["en"]);
    }

    #[test]
    fn resolve_skill_path_handles_relative() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let resolved = resolve_skill_path(root, ".claude/skills");
        assert_eq!(resolved, root.join(".claude/skills"));
    }

    #[test]
    fn load_config_normalizes_locale() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let path = config_path(llmanspec_dir);
        let content = "version: 1\nlocale: zh-cn\nskills:\n  claude_path: .claude/skills\n  codex_path: .codex/skills\n";
        fs::write(&path, content).expect("write config");
        let config = load_config(llmanspec_dir).expect("load").expect("config");
        assert_eq!(config.locale, "zh-Hans");
    }
}
