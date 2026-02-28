use crate::path_utils::safe_parent_for_creation;
use anyhow::{Context, Result};
use llm_json::{RepairOptions, loads};
use rust_i18n::t;
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
    pub security: Option<SecurityConfig>,
}

/// Security configuration for Claude Code settings checking
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Dangerous command patterns that should trigger warnings
    pub dangerous_patterns: Option<Vec<String>>,
    /// Claude Code settings files to check (in precedence order)
    pub claude_settings_files: Option<Vec<String>>,
    /// Whether security checks are enabled
    pub enabled: Option<bool>,
}

impl SecurityConfig {
    /// Get whether security checks are enabled (default: true)
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    /// Get dangerous patterns (default patterns if none specified)
    pub fn get_dangerous_patterns(&self) -> Vec<String> {
        self.dangerous_patterns.clone().unwrap_or_else(|| {
            vec![
                "rm -rf".to_string(),
                "sudo rm".to_string(),
                "dd if=".to_string(),
                "mkfs".to_string(),
                "format".to_string(),
                "chmod 777".to_string(),
                "chown root".to_string(),
                ">:|".to_string(),
                "curl | sh".to_string(),
                "wget | sh".to_string(),
                "eval $(".to_string(),
                "exec $(".to_string(),
                "system(".to_string(),
                "__import__('os').system".to_string(),
                "subprocess.call".to_string(),
                "powershell -c".to_string(),
                "cmd /c".to_string(),
                "registry".to_string(),
                "reg add".to_string(),
                "net user".to_string(),
                "crontab".to_string(),
                "systemctl".to_string(),
                "service".to_string(),
                "iptables".to_string(),
                "ufw".to_string(),
                "firewall".to_string(),
            ]
        })
    }

    /// Get Claude Code settings files to check (default files if none specified)
    pub fn get_claude_settings_files(&self) -> Vec<String> {
        self.claude_settings_files.clone().unwrap_or_else(|| {
            vec![
                ".claude/settings.local.json".to_string(),
                ".claude/settings.json".to_string(),
                "~/.claude/settings.json".to_string(),
            ]
        })
    }
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
            .with_context(|| t!("claude_code.config.read_failed", path = path.display()))?;

        // Try to parse as new format first
        if let Ok(config) = Self::parse_new_format(&content) {
            return Ok(config);
        }

        // Try to parse as old format and migrate
        Self::parse_and_migrate_old_format(&content)
            .with_context(|| t!("claude_code.config.parse_all_failed"))
    }

    fn parse_new_format(content: &str) -> Result<Config> {
        toml::from_str(content).with_context(|| t!("claude_code.config.parse_new_failed"))
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
            toml::from_str(content).with_context(|| t!("claude_code.config.parse_old_failed"))?;

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
        let mut names: Vec<String> = self.groups.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn config_file_path() -> Result<PathBuf> {
        Ok(crate::config::resolve_config_dir(None)?.join("claude-code.toml"))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_file_path()?;
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &PathBuf) -> Result<()> {
        // Create config directory if it doesn't exist
        if let Some(parent) = safe_parent_for_creation(path) {
            fs::create_dir_all(parent).with_context(|| {
                t!(
                    "claude_code.config.create_dir_failed",
                    path = parent.display()
                )
            })?;
        }

        let content = toml::to_string_pretty(self)
            .with_context(|| t!("claude_code.config.serialize_failed"))?;

        fs::write(path, content)
            .with_context(|| t!("claude_code.config.write_failed", path = path.display()))?;

        // Set file permissions to 0600 (read/write for owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)
                .with_context(|| t!("claude_code.config.metadata_failed"))?
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)
                .with_context(|| t!("claude_code.config.permissions_failed"))?;
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
            eprintln!("{}", t!("claude_code.config.json_repairing"));
            match loads(json_str, &RepairOptions::default()) {
                Ok(repaired_value) => {
                    println!("{}", t!("claude_code.config.json_repaired"));
                    repaired_value
                }
                Err(e) => {
                    anyhow::bail!("{}", t!("claude_code.config.json_repair_failed", error = e));
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
                    _ => anyhow::bail!(t!("claude_code.config.json_env_invalid")),
                }
            } else {
                convert_env_map(map)
            }
        }
        _ => anyhow::bail!(t!("claude_code.config.json_must_object")),
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
