use crate::path_utils::safe_parent_for_creation;
use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use llm_json::{RepairOptions, loads};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Simplified ConfigGroup: 直接映射为环境变量
pub type ConfigGroup = HashMap<String, String>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub groups: HashMap<String, ConfigGroup>,
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
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        // Try to parse as new format first
        if let Ok(config) = Self::parse_new_format(&content) {
            return Ok(config);
        }

        // Try to parse as old format and migrate
        Self::parse_and_migrate_old_format(&content)
            .with_context(|| "Failed to parse TOML config file (both old and new formats)")
    }

    fn parse_new_format(content: &str) -> Result<Config> {
        toml::from_str(content).with_context(|| "Failed to parse new format config")
    }

    fn parse_and_migrate_old_format(content: &str) -> Result<Config> {
        #[derive(Debug, Deserialize)]
        struct OldConfigGroup {
            api_host: String,
            api_key: String,
        }

        #[derive(Debug, Deserialize)]
        struct OldConfig {
            groups: HashMap<String, OldConfigGroup>,
        }

        let old_config: OldConfig =
            toml::from_str(content).with_context(|| "Failed to parse old format config")?;

        // Migrate to new format
        let mut new_config = Config::default();
        for (name, old_group) in old_config.groups {
            let mut env_vars = HashMap::new();
            env_vars.insert("ANTHROPIC_BASE_URL".to_string(), old_group.api_host);
            env_vars.insert("ANTHROPIC_AUTH_TOKEN".to_string(), old_group.api_key);

            new_config.add_group(name, env_vars);
        }

        Ok(new_config)
    }

    pub fn add_group(&mut self, name: String, group: ConfigGroup) {
        self.groups.insert(name, group);
    }

    pub fn get_group(&self, name: &str) -> Option<&ConfigGroup> {
        self.groups.get(name)
    }

    pub fn group_names(&self) -> Vec<String> {
        self.groups.keys().cloned().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn config_file_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("com", "StrayDragon", "llman")
            .ok_or_else(|| anyhow!("Could not find project directory"))?;
        Ok(project_dirs.config_dir().join("claude-code.toml"))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_file_path()?;
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &PathBuf) -> Result<()> {
        // Create config directory if it doesn't exist
        if let Some(parent) = safe_parent_for_creation(path) {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content =
            toml::to_string_pretty(self).with_context(|| "Failed to serialize config to TOML")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        // Set file permissions to 0600 (read/write for owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)
                .with_context(|| "Failed to get file metadata")?
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms).with_context(|| "Failed to set file permissions")?;
        }

        Ok(())
    }
}

// Utility function for display formatting
pub fn get_display_vars(group: &ConfigGroup) -> Vec<(String, String)> {
    let mut vars: Vec<_> = group.iter().collect();
    vars.sort_by(|a, b| a.0.cmp(b.0));
    vars.into_iter()
        .map(|(k, v)| {
            (
                k.clone(),
                if k.contains("KEY") || k.contains("TOKEN") || k.contains("SECRET") {
                    mask_secret(v)
                } else {
                    v.clone()
                },
            )
        })
        .collect()
}

pub fn mask_secret(value: &str) -> String {
    if value.len() <= 8 {
        "*".repeat(value.len())
    } else {
        format!("{}...{}", &value[..4], &value[value.len() - 4..])
    }
}

// Parse JSON configuration and convert to ConfigGroup
pub fn parse_json_config(json_str: &str) -> Result<ConfigGroup> {
    // First try standard JSON parsing
    let json_value = match serde_json::from_str::<Value>(json_str) {
        Ok(value) => value,
        Err(_) => {
            // If standard parsing fails, try to repair the JSON
            println!("⚠️  JSON format appears to be malformed, attempting to repair...");
            match loads(json_str, &RepairOptions::default()) {
                Ok(repaired_value) => {
                    println!("✅ Successfully repaired JSON format");
                    repaired_value
                }
                Err(e) => {
                    anyhow::bail!(
                        "Failed to parse JSON string even after repair attempts: {}",
                        e
                    );
                }
            }
        }
    };

    let env_vars = match json_value {
        Value::Object(mut map) => {
            // Handle both {"env": {...}} and direct {...} formats
            if let Some(env_obj) = map.remove("env") {
                match env_obj {
                    Value::Object(env_map) => convert_env_map(env_map),
                    _ => anyhow::bail!("env field must be an object"),
                }
            } else {
                convert_env_map(map)
            }
        }
        _ => anyhow::bail!("JSON must be an object"),
    };

    Ok(env_vars)
}

fn convert_env_map(env_map: serde_json::Map<String, Value>) -> ConfigGroup {
    let mut config_group = HashMap::new();

    for (key, value) in env_map {
        let value_str = match value {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => String::new(),
            _ => value.to_string(), // For complex objects, convert to string representation
        };
        config_group.insert(key, value_str);
    }

    config_group
}
