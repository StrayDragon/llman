use crate::path_utils::safe_parent_for_creation;
use anyhow::{Context, Result, anyhow};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use toml::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    #[serde(default = "default_wire_api")]
    pub wire_api: String,
    pub env_key: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_wire_api() -> String {
    "responses".to_string()
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub model_providers: HashMap<String, ProviderConfig>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_file_path()?;
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| t!("codex.error.load_config_failed", path = path.display()))?;

        toml::from_str(&content)
            .with_context(|| t!("codex.error.parse_config_failed", path = path.display()))
    }

    pub fn provider_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.model_providers.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.model_providers.get(name)
    }

    pub fn is_empty(&self) -> bool {
        self.model_providers.is_empty()
    }

    pub fn add_provider(&mut self, key: String, provider: ProviderConfig) {
        self.model_providers.insert(key, provider);
    }

    pub fn config_file_path() -> Result<PathBuf> {
        Ok(crate::config::resolve_config_dir(None)?.join("codex.toml"))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_file_path()?;
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = safe_parent_for_creation(path) {
            fs::create_dir_all(parent).with_context(|| {
                t!(
                    "codex.error.create_config_dir_failed",
                    path = parent.display()
                )
            })?;
        }

        let content = toml::to_string_pretty(self)
            .with_context(|| t!("codex.error.serialize_config_failed"))?;

        fs::write(path, content)
            .with_context(|| t!("codex.error.write_config_failed", path = path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)
                .with_context(|| t!("codex.error.metadata_failed"))?
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)
                .with_context(|| t!("codex.error.permissions_failed"))?;
        }

        Ok(())
    }
}

/// Build a TOML table for a provider (without the env sub-table) for upsert into codex config.
pub fn provider_to_codex_table(provider: &ProviderConfig) -> Value {
    let mut table = toml::map::Map::new();
    table.insert("name".into(), Value::String(provider.name.clone()));
    table.insert("base_url".into(), Value::String(provider.base_url.clone()));
    table.insert("wire_api".into(), Value::String(provider.wire_api.clone()));
    table.insert("env_key".into(), Value::String(provider.env_key.clone()));
    Value::Table(table)
}

/// Upsert the selected provider into `~/.codex/config.toml`.
/// Sets `model_provider = "<name>"` and `model_providers.<name> = { ... }`.
/// Returns true if the file was actually written (config changed), false if already up-to-date.
pub fn upsert_to_codex_config(provider_key: &str, provider: &ProviderConfig) -> Result<bool> {
    let codex_config_path = codex_config_path()?;

    let mut doc: Value = if codex_config_path.exists() {
        let content = fs::read_to_string(&codex_config_path)
            .with_context(|| t!("codex.error.read_codex_config_failed"))?;
        toml::from_str(&content).with_context(|| t!("codex.error.parse_codex_config_failed"))?
    } else {
        Value::Table(toml::map::Map::new())
    };

    let root = doc
        .as_table_mut()
        .ok_or_else(|| anyhow!(t!("codex.error.codex_config_not_table")))?;

    let new_provider_table = provider_to_codex_table(provider);

    // Check if already up-to-date
    let current_model_provider = root
        .get("model_provider")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let current_provider_entry = root
        .get("model_providers")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get(provider_key));

    if current_model_provider.as_deref() == Some(provider_key)
        && current_provider_entry == Some(&new_provider_table)
    {
        return Ok(false);
    }

    // Set model_provider
    root.insert(
        "model_provider".into(),
        Value::String(provider_key.to_string()),
    );

    // Upsert model_providers.<name>
    let providers = root
        .entry("model_providers")
        .or_insert_with(|| Value::Table(toml::map::Map::new()));

    if let Some(providers_table) = providers.as_table_mut() {
        providers_table.insert(provider_key.to_string(), new_provider_table);
    }

    // Write back
    if let Some(parent) = codex_config_path.parent() {
        fs::create_dir_all(parent).with_context(|| t!("codex.error.create_codex_dir_failed"))?;
    }

    let output = toml::to_string_pretty(&doc)
        .with_context(|| t!("codex.error.serialize_codex_config_failed"))?;

    fs::write(&codex_config_path, output)
        .with_context(|| t!("codex.error.write_codex_config_failed"))?;

    Ok(true)
}

/// Get the path to `~/.codex/config.toml`.
fn codex_config_path() -> Result<PathBuf> {
    let home = crate::config::home_dir().context(t!("codex.error.home_dir_failed"))?;
    Ok(home.join(".codex").join("config.toml"))
}

pub fn mask_secret(value: &str) -> String {
    if value.len() <= 8 {
        "*".repeat(value.len())
    } else {
        format!("{}...{}", &value[..4], &value[value.len() - 4..])
    }
}
