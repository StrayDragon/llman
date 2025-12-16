use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use rust_i18n::t;

/// Configuration status for detection and migration
#[derive(Debug, Clone)]
pub enum ConfigStatus {
    /// Symlink is active and functioning properly
    SymlinkActive,
    /// Imported existing configuration as 'default'
    Imported,
    /// Created new default configuration
    Created,
    /// Migrated from regular file to symlink system
    Migrated,
}

/// Core manager for OpenAI Codex configuration using symlinks
pub struct CodexManager {
    codex_dir: PathBuf,
    configs_dir: PathBuf,
    active_config: PathBuf,
}

impl CodexManager {
    /// Creates a new CodexManager instance
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir()
            .context("Cannot find home directory")?;

        let codex_dir = home.join(".codex");
        let configs_dir = codex_dir.join("configs");
        let active_config = codex_dir.join("config.toml");

        Ok(Self {
            codex_dir,
            configs_dir,
            active_config,
        })
    }

    /// Get the path to the active config file
    pub fn active_config_path(&self) -> &Path {
        &self.active_config
    }

    /// Get the path to the configs directory
    pub fn configs_dir(&self) -> &Path {
        &self.configs_dir
    }

    /// Initialize or detect existing configuration state
    pub fn init_or_detect(&self) -> Result<ConfigStatus> {
        // Create directory structure
        std::fs::create_dir_all(&self.configs_dir)
            .context("Failed to create configs directory")?;

        // Detect existing configuration state
        if !self.active_config.exists() {
            // No main config file exists
            return if self.has_existing_codex_config() {
                // Found existing Codex config, import it
                self.import_existing_config()
            } else {
                // Create default configuration
                self.create_default_setup()
            };
        }

        // Check if it's a symlink
        match std::fs::read_link(&self.active_config) {
            Ok(_target) => {
                // Already a symlink, normal state
                Ok(ConfigStatus::SymlinkActive)
            }
            Err(_) => {
                // It's a regular file, need to migrate to symlink system
                self.migrate_to_symlink()
            }
        }
    }

    /// Check if there's an existing Codex configuration
    fn has_existing_codex_config(&self) -> bool {
        self.active_config.exists()
    }

    /// Import existing Codex configuration
    fn import_existing_config(&self) -> Result<ConfigStatus> {
        println!("🔄 {} ", t!("codex.config.import.found_existing"));

        // Save existing config as default
        let default_config = self.configs_dir.join("default.toml");
        std::fs::copy(&self.active_config, &default_config)
            .context("Failed to copy existing config to default")?;

        // Create symlink
        self.create_symlink(&default_config)?;

        // Add llman metadata
        self.enhance_config_with_llman_metadata(&default_config, "imported")?;

        println!("✅ {}", t!("codex.config.import.imported_as_default"));
        println!("📁 {}", t!("codex.config.import.default_location",
            path = default_config.display()));

        Ok(ConfigStatus::Imported)
    }

    /// Create default setup for new installations
    fn create_default_setup(&self) -> Result<ConfigStatus> {
        println!("🚀 {} ", t!("codex.config.setup.creating_default"));

        let default_config = self.configs_dir.join("default.toml");
        self.create_default_config(&default_config)?;
        self.create_symlink(&default_config)?;

        println!("✅ {}", t!("codex.config.setup.default_created"));
        Ok(ConfigStatus::Created)
    }

    /// Migrate existing regular file to symlink system
    fn migrate_to_symlink(&self) -> Result<ConfigStatus> {
        println!("🔄 {} ", t!("codex.config.migrate.detected_regular_file"));

        // Backup existing config
        let backup_path = self.active_config.with_extension("toml.llman.backup");
        std::fs::copy(&self.active_config, &backup_path)
            .context("Failed to backup existing config")?;

        // Save as default config
        let default_config = self.configs_dir.join("default.toml");
        std::fs::copy(&self.active_config, &default_config)
            .context("Failed to copy existing config to default")?;

        // Create symlink
        self.create_symlink(&default_config)?;

        // Add llman metadata
        self.enhance_config_with_llman_metadata(&default_config, "migrated")?;

        println!("✅ {}", t!("codex.config.migrate.migrated_success"));
        println!("💾 {}", t!("codex.config.migrate.backup_location",
            path = backup_path.display()));

        Ok(ConfigStatus::Migrated)
    }

    /// List all available configurations
    pub fn list_configs(&self) -> Result<Vec<String>> {
        let mut configs = Vec::new();

        if self.configs_dir.exists() {
            for entry in std::fs::read_dir(&self.configs_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        configs.push(name.to_string());
                    }
                }
            }
        }

        configs.sort();
        Ok(configs)
    }

    /// Create a new configuration
    pub fn create_config(&self, name: &str, template: Option<&str>) -> Result<PathBuf> {
        let config_path = self.configs_dir.join(format!("{}.toml", name));

        if config_path.exists() {
            anyhow::bail!("{}", t!("codex.config.error.config_exists", name = name));
        }

        let content = if let Some(template_name) = template {
            self.get_template(template_name)?
        } else {
            self.get_default_template()?
        };

        std::fs::write(&config_path, content)
            .context("Failed to write config file")?;

        // Add llman metadata
        self.enhance_config_with_llman_metadata(&config_path, template.unwrap_or("custom"))?;

        Ok(config_path)
    }

    /// Switch to a specific configuration
    pub fn use_config(&self, name: &str) -> Result<()> {
        let config_path = self.configs_dir.join(format!("{}.toml", name));

        if !config_path.exists() {
            anyhow::bail!("{}", t!("codex.config.error.config_not_found", name = name));
        }

        // Don't do anything if it's already the active config
        if let Ok(current) = self.get_current_config() {
            if let Some(current_name) = current {
                if current_name == name {
                    println!("ℹ️️ {}", t!("codex.config.use.already_active", name = name));
                    return Ok(());
                }
            }
        }

        self.create_symlink(&config_path)?;
        println!("✅ {}", t!("codex.config.use.switched", name = name));
        Ok(())
    }

    /// Get the currently active configuration name
    pub fn get_current_config(&self) -> Result<Option<String>> {
        if !self.active_config.exists() {
            return Ok(None);
        }

        match std::fs::read_link(&self.active_config) {
            Ok(target) => {
                Ok(target
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string()))
            }
            Err(_) => {
                // It's not a symlink, check if it's a regular file in configs/
                if self.active_config.exists() {
                    // Try to get the name from the path if it's in configs dir
                    if let (Some(parent), Some(filename)) = (self.active_config.parent(), self.active_config.file_stem()) {
                        if let Some(expected_parent) = self.configs_dir.parent() {
                            if parent == expected_parent {
                                Ok(filename.to_str().map(|s| s.to_string()))
                            } else {
                                Ok(None)
                            }
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Delete a configuration
    pub fn delete_config(&self, name: &str) -> Result<()> {
        let config_path = self.configs_dir.join(format!("{}.toml", name));

        if !config_path.exists() {
            anyhow::bail!("{}", t!("codex.config.error.config_not_found", name = name));
        }

        // Check if it's the currently active config
        if let Some(current) = self.get_current_config()? {
            if current == name {
                anyhow::bail!("{}", t!("codex.config.error.cannot_delete_active", name = name));
            }
        }

        std::fs::remove_file(&config_path)
            .context("Failed to delete config file")?;

        println!("🗑️  {}", t!("codex.config.delete.deleted", name = name));
        Ok(())
    }

    /// Create symlink from active config to target config
    fn create_symlink(&self, target: &Path) -> Result<()> {
        // Remove existing link or file
        if self.active_config.exists() {
            std::fs::remove_file(&self.active_config)
                .context("Failed to remove existing config file")?;
        }

        // Create new symlink using platform-specific approach
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, &self.active_config)
                .context("Failed to create symlink")?;
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, &self.active_config)
                .context("Failed to create symlink")?;
        }

        Ok(())
    }

    /// Create default configuration file
    fn create_default_config(&self, path: &Path) -> Result<()> {
        let template = self.get_default_template()?;
        std::fs::write(path, template)
            .context("Failed to write default config")?;
        Ok(())
    }

    /// Get default configuration template
    fn get_default_template(&self) -> Result<String> {
        Ok(format!(r#"# Default OpenAI Codex Configuration
# For full documentation, see: https://developers.openai.com/codex

# Model settings
model = "gpt-4o"
model_provider = "openai"

# Approval policy: untrusted, on-failure, on-request, never
approval_policy = "on-request"

# Sandbox mode: read-only, workspace-write, danger-full-access
sandbox_mode = "workspace-write"

# OpenAI Provider
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"

# llman specific configuration
[llman]
# This configuration is managed by llman
auto_created = true
template = "default"
version = "1.0"

[llman.profiles]
# This section is reserved for llman-specific metadata

# Optional: Add custom profiles
[profiles.development]
model = "gpt-4o"
approval_policy = "on-request"

[profiles.production]
model = "gpt-4o"
approval_policy = "never"

# Optional: Enable features
[features]
# streamable_shell = true
# web_search_request = true
"#))
    }

    /// Get template by name
    fn get_template(&self, template_name: &str) -> Result<String> {
        match template_name {
            "openai" => Ok(format!(r#"# OpenAI Configuration
model = "gpt-4o"
model_provider = "openai"
approval_policy = "on-request"

[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"
"#)),
            "ollama" => Ok(format!(r#"# Ollama Configuration
model = "llama3"
model_provider = "ollama"
approval_policy = "never"

[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"
wire_api = "chat"
"#)),
            "minimal" => Ok(format!(r#"# Minimal Configuration
model = "gpt-4o"
model_provider = "openai"

[model_providers.openai]
env_key = "OPENAI_API_KEY"
"#)),
            _ => anyhow::bail!("{}", t!("codex.config.error.unknown_template", template = template_name)),
        }
    }

    /// Enhance configuration with llman metadata
    fn enhance_config_with_llman_metadata(&self, config_path: &Path, template: &str) -> Result<()> {
        let mut content = std::fs::read_to_string(config_path)
            .context("Failed to read config file for enhancement")?;

        let timestamp = Utc::now().to_rfc3339();

        // Add or update llman section
        let llman_section = format!(
            r#"
# llman specific configuration
[llman]
# This configuration is managed by llman
auto_created = true
template = "{}"
created_at = "{}"
version = "1.0"

[llman.profiles]
# This section is reserved for llman-specific metadata"#,
            template,
            timestamp
        );

        // Use regex to find and replace the llman section, or append if not found
        let llman_regex = Regex::new(r"(?s)\[llman\].*?(?=\n\[|\n#|$)")
            .context("Failed to create regex for llman section")?;

        if llman_regex.is_match(&content) {
            // Update existing llman section
            content = llman_regex
                .replace(&content, llman_section.trim())
                .to_string();
        } else {
            // Append llman section
            content.push_str("\n");
            content.push_str(&llman_section);
        }

        std::fs::write(config_path, content)
            .context("Failed to write enhanced config file")?;

        Ok(())
    }
}