use crate::path_utils::safe_parent_for_creation;
use anyhow::{Context, Result, anyhow};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use toml::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLlmanConfigs {
    pub override_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    #[serde(default = "default_wire_api")]
    pub wire_api: String,
    pub env_key: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llman_configs: Option<ProviderLlmanConfigs>,
    #[serde(default, flatten)]
    pub extra: HashMap<String, Value>,
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
pub fn provider_to_codex_table(provider: &ProviderConfig, effective_name: &str) -> Value {
    let mut table = toml::map::Map::new();

    let mut extra_keys: Vec<&String> = provider.extra.keys().collect();
    extra_keys.sort();
    for key in extra_keys {
        let Some(value) = provider.extra.get(key) else {
            continue;
        };
        if key == "env" || key == "llman_configs" {
            continue;
        }
        table.insert(key.clone(), value.clone());
    }

    table.insert("name".into(), Value::String(effective_name.to_string()));
    table.insert("base_url".into(), Value::String(provider.base_url.clone()));
    table.insert("wire_api".into(), Value::String(provider.wire_api.clone()));
    table.insert("env_key".into(), Value::String(provider.env_key.clone()));
    Value::Table(table)
}

/// Upsert the selected provider into `~/.codex/config.toml`.
/// Sets `model_provider = "<name>"` and `model_providers.<name> = { ... }`.
/// Returns true if the file was actually written (config changed), false if already up-to-date.
pub fn upsert_to_codex_config(provider_key: &str, provider: &ProviderConfig) -> Result<bool> {
    let effective_name = match provider
        .llman_configs
        .as_ref()
        .and_then(|cfg| cfg.override_name.as_deref())
    {
        Some(override_name) => {
            let override_name = override_name.trim();
            if override_name.is_empty() {
                return Err(anyhow!(
                    t!("codex.error.override_name_blank", name = provider_key)
                ));
            }
            override_name.to_string()
        }
        None => provider_key.to_string(),
    };

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

    let new_provider_table = provider_to_codex_table(provider, &effective_name);

    // Check if already up-to-date
    let current_model_provider = root
        .get("model_provider")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let current_provider_entry = root
        .get("model_providers")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get(&effective_name));

    if current_model_provider.as_deref() == Some(&effective_name)
        && current_provider_entry == Some(&new_provider_table)
    {
        return Ok(false);
    }

    // Set model_provider
    root.insert(
        "model_provider".into(),
        Value::String(effective_name.to_string()),
    );

    // Upsert model_providers.<name>
    let providers = root
        .entry("model_providers")
        .or_insert_with(|| Value::Table(toml::map::Map::new()));

    if let Some(providers_table) = providers.as_table_mut() {
        providers_table.insert(effective_name.to_string(), new_provider_table);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestProcess;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn provider_to_codex_table_includes_extra_excludes_env_and_llman_configs_and_overrides_name() {
        let provider = ProviderConfig {
            name: "b".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            wire_api: "responses".to_string(),
            env_key: "CODEX_API_KEY".to_string(),
            env: [("CODEX_API_KEY".to_string(), "sk-test".to_string())]
                .into_iter()
                .collect(),
            llman_configs: Some(ProviderLlmanConfigs {
                override_name: Some("a".to_string()),
            }),
            extra: [
                ("request_max_retries".to_string(), Value::Integer(9999)),
                ("some_flag".to_string(), Value::Boolean(true)),
            ]
            .into_iter()
            .collect(),
        };

        let table = provider_to_codex_table(&provider, "a");
        let t = table.as_table().expect("provider table");

        assert_eq!(t.get("name").and_then(|v| v.as_str()), Some("a"));
        assert_eq!(
            t.get("base_url").and_then(|v| v.as_str()),
            Some("https://api.example.com/v1")
        );
        assert_eq!(
            t.get("wire_api").and_then(|v| v.as_str()),
            Some("responses")
        );
        assert_eq!(
            t.get("env_key").and_then(|v| v.as_str()),
            Some("CODEX_API_KEY")
        );

        assert_eq!(
            t.get("request_max_retries").and_then(|v| v.as_integer()),
            Some(9999)
        );
        assert_eq!(t.get("some_flag").and_then(|v| v.as_bool()), Some(true));

        assert!(t.get("env").is_none());
        assert!(t.get("llman_configs").is_none());
    }

    #[test]
    fn config_save_roundtrips_provider_extra_fields() {
        let temp = TempDir::new().expect("temp dir");
        let config_path = temp.path().join("codex.toml");
        fs::write(
            &config_path,
            r#"
[model_providers.b]
name = "b"
base_url = "https://api.example.com/v1"
wire_api = "responses"
env_key = "CODEX_API_KEY"
request_max_retries = 9999

[model_providers.b.env]
CODEX_API_KEY = "sk-test"

[model_providers.b.llman_configs]
override_name = "a"
"#,
        )
        .expect("write config");

        let config = Config::load_from_path(&config_path).expect("load config");
        let provider = config.get_provider("b").expect("provider b");
        assert_eq!(
            provider
                .extra
                .get("request_max_retries")
                .and_then(|v| v.as_integer()),
            Some(9999)
        );
        assert_eq!(
            provider
                .llman_configs
                .as_ref()
                .and_then(|c| c.override_name.as_deref()),
            Some("a")
        );

        let saved_path = temp.path().join("codex.saved.toml");
        config.save_to_path(&saved_path).expect("save config");

        let config2 = Config::load_from_path(&saved_path).expect("load saved config");
        let provider2 = config2.get_provider("b").expect("provider b");
        assert_eq!(
            provider2
                .extra
                .get("request_max_retries")
                .and_then(|v| v.as_integer()),
            Some(9999)
        );
        assert_eq!(
            provider2
                .llman_configs
                .as_ref()
                .and_then(|c| c.override_name.as_deref()),
            Some("a")
        );
    }

    #[test]
    fn upsert_to_codex_config_uses_override_name_and_is_idempotent() {
        let temp = TempDir::new().expect("temp dir");
        let mut proc = TestProcess::new();
        proc.set_var("HOME", temp.path());

        let provider = ProviderConfig {
            name: "b".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            wire_api: "responses".to_string(),
            env_key: "CODEX_API_KEY".to_string(),
            env: HashMap::new(),
            llman_configs: Some(ProviderLlmanConfigs {
                override_name: Some("a".to_string()),
            }),
            extra: [("request_max_retries".to_string(), Value::Integer(9999))]
                .into_iter()
                .collect(),
        };

        let wrote = upsert_to_codex_config("b", &provider).expect("upsert");
        assert!(wrote);

        let codex_config_path = temp.path().join(".codex").join("config.toml");
        let content1 = fs::read_to_string(&codex_config_path).expect("read config");

        let v: Value = toml::from_str(&content1).expect("parse toml");
        let root = v.as_table().expect("root table");

        assert_eq!(
            root.get("model_provider").and_then(|v| v.as_str()),
            Some("a")
        );

        let providers = root
            .get("model_providers")
            .and_then(|v| v.as_table())
            .expect("model_providers");
        assert!(providers.get("b").is_none());

        let provider_a = providers.get("a").and_then(|v| v.as_table()).expect("a");
        assert_eq!(provider_a.get("name").and_then(|v| v.as_str()), Some("a"));
        assert_eq!(
            provider_a.get("request_max_retries").and_then(|v| v.as_integer()),
            Some(9999)
        );
        assert!(provider_a.get("env").is_none());
        assert!(provider_a.get("llman_configs").is_none());

        let wrote2 = upsert_to_codex_config("b", &provider).expect("second upsert");
        assert!(!wrote2);

        let content2 = fs::read_to_string(&codex_config_path).expect("read config 2");
        assert_eq!(content1, content2);
    }
}
