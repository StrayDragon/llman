use crate::x::codex::interactive;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use toml;

/// Configuration manager for Codex
/// Manages a single configuration file with all profiles
pub struct CodexConfigManager {
    config_file: PathBuf,
    codex_config_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexConfig {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub model_providers: std::collections::HashMap<String, ModelProvider>,
    pub profiles: std::collections::HashMap<String, CodexProfile>,
    pub features: Option<Features>,
    pub shell_environment_policy: Option<ShellEnvironmentPolicy>,
    #[serde(flatten)]
    pub sandbox_config: SandboxConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexProfile {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub model_providers: Option<std::collections::HashMap<String, ModelProvider>>,
    pub features: Option<Features>,
    pub shell_environment_policy: Option<ShellEnvironmentPolicy>,
    #[serde(flatten)]
    pub sandbox_config: SandboxConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    pub name: String,
    pub base_url: String,
    pub env_key: String,
    pub wire_api: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Features {
    pub view_image_tool: Option<bool>,
    pub web_search_request: Option<bool>,
    pub apply_patch_freeform: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellEnvironmentPolicy {
    pub inherit: Option<String>,
    pub include_only: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub set: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SandboxConfig {
    #[serde(rename = "sandbox_workspace_write")]
    pub sandbox_workspace_write: Option<SandboxWorkspaceWrite>,
    #[serde(rename = "sandbox_read_only")]
    pub sandbox_read_only: Option<SandboxReadOnly>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxWorkspaceWrite {
    pub exclude_tmpdir_env_var: Option<bool>,
    pub exclude_slash_tmp: Option<bool>,
    pub network_access: Option<bool>,
    pub writable_roots: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxReadOnly {
    pub exclude_tmpdir_env_var: Option<bool>,
    pub exclude_slash_tmp: Option<bool>,
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            model: None,
            model_provider: Some("openai".to_string()),
            approval_policy: None,
            sandbox_mode: Some("workspace-write".to_string()),
            model_providers: std::collections::HashMap::new(),
            profiles: std::collections::HashMap::new(),
            features: None,
            shell_environment_policy: None,
            sandbox_config: SandboxConfig::default(),
        }
    }
}

impl CodexConfigManager {
    /// Create a new CodexConfigManager
    pub fn new() -> Result<Self> {
        let base_config = std::env::var("LLMAN_CONFIG_DIR")
            .context("LLMAN_CONFIG_DIR environment variable not set")?;

        let config_file = PathBuf::from(base_config).join("codex.toml");
        let codex_config_path = dirs::home_dir()
            .context("Failed to get home directory")?
            .join(".codex")
            .join("config.toml");

        Ok(Self {
            config_file,
            codex_config_path,
        })
    }

    /// Initialize configuration file only if it doesn't exist
    pub fn initialize(&self) -> Result<()> {
        // If config file already exists, just use it
        if self.config_file.exists() {
            println!(
                "🔧 Using existing configuration file: {}",
                self.config_file.display()
            );
            return Ok(());
        }

        // Check if parent directory exists and has files that might conflict
        if let Some(parent) = self.config_file.parent()
            && parent.exists()
            && parent.read_dir()?.next().is_some()
        {
            // Directory exists and is not empty, but config file doesn't exist
            println!(
                "⚠️  Configuration directory '{}' exists but no config file found.",
                parent.display()
            );
            if !interactive::confirm_overwrite()? {
                println!("❌ Initialization cancelled.");
                return Ok(());
            }
        }

        // Create new configuration
        println!(
            "📝 Creating new configuration file: {}",
            self.config_file.display()
        );
        let config = CodexConfig::default();
        self.save_config(&config)?;

        // Create development template profile
        self.create_profile_from_template("dev", "development")?;

        Ok(())
    }

    /// Load configuration from file
    pub fn load_config(&self) -> Result<CodexConfig> {
        if !self.config_file.exists() {
            return Ok(CodexConfig::default());
        }

        let content =
            fs::read_to_string(&self.config_file).context("Failed to read configuration file")?;

        let config: CodexConfig =
            toml::from_str(&content).context("Failed to parse configuration file")?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save_config(&self, config: &CodexConfig) -> Result<()> {
        let content =
            toml::to_string_pretty(config).context("Failed to serialize configuration")?;

        // Create parent directory if needed
        if let Some(parent) = self.config_file.parent() {
            fs::create_dir_all(parent).context("Failed to create configuration directory")?;
        }

        fs::write(&self.config_file, content).context("Failed to write configuration file")?;

        Ok(())
    }

    /// Export configuration to ~/.codex/config.toml
    pub fn export(&self) -> Result<()> {
        let config = self.load_config()?;

        // Create .codex directory
        if let Some(parent) = self.codex_config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create .codex directory")?;
        }

        let content =
            toml::to_string_pretty(&config).context("Failed to serialize configuration")?;

        fs::write(&self.codex_config_path, content)
            .context("Failed to export configuration to ~/.codex/config.toml")?;

        println!(
            "✅ Configuration exported to: {}",
            self.codex_config_path.display()
        );
        Ok(())
    }

    /// Get the path to the configuration file (for editing)
    pub fn config_file_path(&self) -> &PathBuf {
        &self.config_file
    }

    /// List all available profiles
    pub fn list_profiles(&self) -> Result<Vec<String>> {
        let config = self
            .load_config()
            .context("Failed to load configuration for profile listing")?;
        Ok(config.profiles.keys().cloned().collect())
    }

    /// Get a specific profile
    pub fn get_profile(&self, name: &str) -> Result<Option<CodexProfile>> {
        let config = self.load_config()?;
        Ok(config.profiles.get(name).cloned())
    }

    /// Add or update a profile
    pub fn add_profile(&self, name: &str, profile: CodexProfile) -> Result<()> {
        let mut config = self.load_config()?;
        config.profiles.insert(name.to_string(), profile);
        self.save_config(&config)?;
        println!("✅ Profile '{}' added/updated", name);
        Ok(())
    }

    /// Remove a profile
    pub fn remove_profile(&self, name: &str) -> Result<()> {
        let mut config = self.load_config()?;
        if config.profiles.remove(name).is_some() {
            self.save_config(&config)?;
            println!("✅ Profile '{}' removed", name);
        } else {
            println!("⚠️  Profile '{}' not found", name);
        }
        Ok(())
    }

    /// Create profile from template
    pub fn create_profile_from_template(&self, name: &str, template: &str) -> Result<()> {
        let profile = match template {
            "development" | "dev" => {
                println!("✅ Using development template (relaxed security, enabled features)");
                self.development_template()
            }
            "production" | "prod" => {
                println!("✅ Using production template (strict security, limited features)");
                self.production_template()
            }
            _ => {
                eprintln!(
                    "⚠️  Unknown template '{}', falling back to development template",
                    template
                );
                println!("💡 Available templates: development, production");
                self.development_template()
            }
        };

        self.add_profile(name, profile)
    }

    /// Get development template
    fn development_template(&self) -> CodexProfile {
        CodexProfile {
            model: Some("gpt-4".to_string()),
            model_provider: Some("openai".to_string()),
            approval_policy: Some("never".to_string()),
            sandbox_mode: Some("workspace-write".to_string()),
            model_providers: Some({
                let mut providers = std::collections::HashMap::new();
                providers.insert(
                    "openai".to_string(),
                    ModelProvider {
                        name: "OpenAI".to_string(),
                        base_url: "https://api.openai.com/v1".to_string(),
                        env_key: "OPENAI_API_KEY".to_string(),
                        wire_api: Some("chat".to_string()),
                    },
                );
                providers
            }),
            features: Some(Features {
                view_image_tool: Some(true),
                web_search_request: Some(true),
                apply_patch_freeform: Some(false),
            }),
            shell_environment_policy: Some(ShellEnvironmentPolicy {
                inherit: Some("core".to_string()),
                include_only: Some(vec![
                    "PATH".to_string(),
                    "HOME".to_string(),
                    "LANG".to_string(),
                    "NODE_ENV".to_string(),
                    "RUST_LOG".to_string(),
                    "PYTHONPATH".to_string(),
                ]),
                exclude: None,
                set: None,
            }),
            sandbox_config: SandboxConfig {
                sandbox_workspace_write: Some(SandboxWorkspaceWrite {
                    exclude_tmpdir_env_var: Some(false),
                    exclude_slash_tmp: Some(false),
                    network_access: Some(true),
                    writable_roots: Some(vec!["/tmp".to_string()]),
                }),
                sandbox_read_only: None,
            },
        }
    }

    /// Get production template
    fn production_template(&self) -> CodexProfile {
        CodexProfile {
            model: Some("gpt-4".to_string()),
            model_provider: Some("openai".to_string()),
            approval_policy: Some("on-request".to_string()),
            sandbox_mode: Some("read-only".to_string()),
            model_providers: Some({
                let mut providers = std::collections::HashMap::new();
                providers.insert(
                    "openai".to_string(),
                    ModelProvider {
                        name: "OpenAI".to_string(),
                        base_url: "https://api.openai.com/v1".to_string(),
                        env_key: "OPENAI_API_KEY".to_string(),
                        wire_api: Some("chat".to_string()),
                    },
                );
                providers
            }),
            features: Some(Features {
                view_image_tool: Some(false),
                web_search_request: Some(false),
                apply_patch_freeform: Some(false),
            }),
            shell_environment_policy: Some(ShellEnvironmentPolicy {
                inherit: Some("core".to_string()),
                include_only: Some(vec![
                    "PATH".to_string(),
                    "HOME".to_string(),
                    "LANG".to_string(),
                ]),
                exclude: None,
                set: None,
            }),
            sandbox_config: SandboxConfig {
                sandbox_workspace_write: None,
                sandbox_read_only: Some(SandboxReadOnly {
                    exclude_tmpdir_env_var: Some(true),
                    exclude_slash_tmp: Some(true),
                }),
            },
        }
    }
}
