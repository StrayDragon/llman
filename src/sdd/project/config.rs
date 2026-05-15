use crate::config_schema::ConfigSchemaKind;
use crate::config_schema::{LLMANSPEC_SCHEMA_URL, prepend_schema_header, validate_yaml_value};
use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::LLMANSPEC_CONFIG_FILE;
use anyhow::{Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const EXPECTED_SCHEMA: &str = "spec-driven";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "SDD project configuration for llmanspec.")]
pub struct SddConfig {
    #[schemars(description = "Schema identifier. Must be \"spec-driven\".")]
    pub schema: String,

    #[serde(default = "default_locale")]
    #[schemars(description = "Locale used for SDD templates and skills.")]
    pub locale: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Project context (tech stack, conventions, constraints). Replaces project.md."
    )]
    pub context: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Per-artifact rules. Map of artifact_id -> string[].")]
    pub rules: Option<BTreeMap<String, Vec<String>>>,
}

impl Default for SddConfig {
    fn default() -> Self {
        Self {
            schema: EXPECTED_SCHEMA.to_string(),
            locale: default_locale(),
            context: None,
            rules: None,
        }
    }
}

fn default_locale() -> String {
    "en".to_string()
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

    // Reject old-format configs
    reject_old_format(&yaml_value, &path)?;

    if let Err(error) = validate_yaml_value(ConfigSchemaKind::Llmanspec, &yaml_value) {
        return Err(anyhow!(t!(
            "sdd.config.schema_invalid",
            path = path.display(),
            error = error
        )));
    }
    let config: SddConfig = serde_yaml::from_value(yaml_value)
        .map_err(|err| anyhow!(t!("sdd.config.parse_failed", error = err)))?;

    if config.schema.trim() != EXPECTED_SCHEMA {
        return Err(anyhow!(
            "Unsupported schema '{}'. Expected '{}'.",
            config.schema.trim(),
            EXPECTED_SCHEMA
        ));
    }

    Ok(Some(SddConfig {
        schema: EXPECTED_SCHEMA.to_string(),
        locale: normalize_locale(&config.locale),
        context: config.context,
        rules: config.rules,
    }))
}

fn reject_old_format(value: &serde_yaml::Value, path: &Path) -> Result<()> {
    let Some(mapping) = value.as_mapping() else {
        return Ok(());
    };
    let has_old_keys = mapping
        .keys()
        .any(|k| k.as_str() == Some("spec_style") || k.as_str() == Some("version"));
    if has_old_keys {
        return Err(anyhow!(
            "Old config format detected in {}. Please run `llman sdd init` to reinitialize.",
            path.display()
        ));
    }
    Ok(())
}

pub fn load_required_config(llmanspec_dir: &Path) -> Result<SddConfig> {
    load_config(llmanspec_dir)?.ok_or_else(|| {
        let path = config_path(llmanspec_dir);
        anyhow!(t!("sdd.config.missing", path = path.display()))
    })
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
    atomic_write_with_mode(&path, content.as_bytes(), None)
        .map_err(|err| anyhow!(t!("sdd.config.write_failed", error = err)))?;
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
    fn load_config_normalizes_locale() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let path = config_path(llmanspec_dir);
        let content = "schema: spec-driven\nlocale: zh-cn\n";
        fs::write(&path, content).expect("write config");
        let config = load_config(llmanspec_dir).expect("load").expect("config");
        assert_eq!(config.locale, "zh-Hans");
    }

    #[test]
    fn load_config_rejects_old_format() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let path = config_path(llmanspec_dir);
        let content = "version: 1\nspec_style: ison\nlocale: en\n";
        fs::write(&path, content).expect("write config");
        let result = load_config(llmanspec_dir);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Old config format detected"), "got: {err}");
    }

    #[test]
    fn load_config_rejects_wrong_schema() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let path = config_path(llmanspec_dir);
        let content = "schema: other\nlocale: en\n";
        fs::write(&path, content).expect("write config");
        let result = load_config(llmanspec_dir);
        assert!(result.is_err());
    }

    #[test]
    fn default_config_has_spec_driven() {
        let config = SddConfig::default();
        assert_eq!(config.schema, "spec-driven");
        assert_eq!(config.locale, "en");
        assert!(config.context.is_none());
        assert!(config.rules.is_none());
    }
}
