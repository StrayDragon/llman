use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Metadata for codex configuration management
#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    /// Currently selected group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_group: Option<String>,
}

impl Metadata {
    /// Load metadata from file
    pub fn load() -> Result<Self> {
        let path = Self::metadata_path()?;

        if !path.exists() {
            return Ok(Self {
                current_group: None,
            });
        }

        let content = fs::read_to_string(&path).context("Failed to read metadata file")?;

        let metadata: Self = toml::from_str(&content).context("Failed to parse metadata file")?;

        Ok(metadata)
    }

    /// Save metadata to file
    pub fn save(&self) -> Result<()> {
        let path = Self::metadata_path()?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create codex directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize metadata")?;

        fs::write(&path, content).context("Failed to write metadata file")?;

        Ok(())
    }

    /// Get metadata file path
    fn metadata_path() -> Result<PathBuf> {
        let codex_dir = Self::codex_dir()?;
        Ok(codex_dir.join("metadata.toml"))
    }

    /// Get codex configuration directory
    pub fn codex_dir() -> Result<PathBuf> {
        let config_dir = if let Ok(dir) = std::env::var("LLMAN_CONFIG_DIR") {
            PathBuf::from(dir)
        } else {
            dirs::config_dir()
                .context("Failed to get config directory")?
                .join("llman")
        };
        Ok(config_dir.join("codex"))
    }

    /// Get groups directory path
    pub fn groups_dir() -> Result<PathBuf> {
        Ok(Self::codex_dir()?.join("groups"))
    }
}

/// Configuration manager for codex groups
pub struct ConfigManager;

impl ConfigManager {
    /// List all available groups
    pub fn list_groups() -> Result<Vec<String>> {
        let groups_dir = Metadata::groups_dir()?;

        if !groups_dir.exists() {
            return Ok(Vec::new());
        }

        let mut groups = Vec::new();

        for entry in fs::read_dir(&groups_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    groups.push(name.to_string());
                }
            }
        }

        groups.sort();
        Ok(groups)
    }

    /// Get path to a group's config file
    pub fn group_path(name: &str) -> Result<PathBuf> {
        let groups_dir = Metadata::groups_dir()?;
        Ok(groups_dir.join(format!("{}.toml", name)))
    }

    /// Check if a group exists
    pub fn group_exists(name: &str) -> Result<bool> {
        Ok(Self::group_path(name)?.exists())
    }

    /// Create a new group from template
    pub fn create_group(name: &str, template: &str) -> Result<()> {
        let group_path = Self::group_path(name)?;

        if group_path.exists() {
            bail!("Group '{}' already exists", name);
        }

        // Create groups directory if needed
        if let Some(parent) = group_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write template
        fs::write(&group_path, template)?;

        Ok(())
    }

    /// Import a group from existing codex config
    pub fn import_group(name: &str, source_path: &Path) -> Result<()> {
        let group_path = Self::group_path(name)?;

        if group_path.exists() {
            bail!("Group '{}' already exists", name);
        }

        // Create groups directory if needed
        if let Some(parent) = group_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Copy file
        fs::copy(source_path, &group_path)?;

        Ok(())
    }

    /// Delete a group
    pub fn delete_group(name: &str) -> Result<()> {
        let group_path = Self::group_path(name)?;

        if !group_path.exists() {
            bail!("Group '{}' does not exist", name);
        }

        fs::remove_file(&group_path)?;

        Ok(())
    }

    /// Read a group's configuration
    pub fn read_group(name: &str) -> Result<String> {
        let group_path = Self::group_path(name)?;

        if !group_path.exists() {
            bail!("Group '{}' does not exist", name);
        }

        fs::read_to_string(&group_path).context("Failed to read group configuration")
    }

    /// Switch to a group by creating symlink
    pub fn switch_group(name: &str) -> Result<()> {
        let group_path = Self::group_path(name)?;

        if !group_path.exists() {
            bail!("Group '{}' does not exist", name);
        }

        let codex_config = Self::codex_config_path()?;

        // Backup existing config if it's not a symlink
        if codex_config.exists() && !Self::is_symlink(&codex_config) {
            let backup_path = codex_config.with_extension("toml.backup");
            fs::rename(&codex_config, &backup_path)?;
            eprintln!("Backed up existing config to: {}", backup_path.display());
        }

        // Remove existing symlink/file
        if codex_config.exists() {
            fs::remove_file(&codex_config)?;
        }

        // Create parent directory if needed
        if let Some(parent) = codex_config.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create symlink
        Self::create_symlink(&group_path, &codex_config)?;

        // Update metadata
        let mut metadata = Metadata::load()?;
        metadata.current_group = Some(name.to_string());
        metadata.save()?;

        Ok(())
    }

    /// Get the path to ~/.codex/config.toml
    fn codex_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        Ok(home.join(".codex").join("config.toml"))
    }

    /// Check if a path is a symlink
    #[cfg(unix)]
    fn is_symlink(path: &Path) -> bool {
        path.symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    fn is_symlink(_path: &Path) -> bool {
        // On Windows, we use file copy instead of symlink
        false
    }

    /// Create a symlink (or copy on Windows)
    #[cfg(unix)]
    fn create_symlink(source: &Path, target: &Path) -> Result<()> {
        std::os::unix::fs::symlink(source, target).context("Failed to create symlink")?;
        Ok(())
    }

    #[cfg(windows)]
    fn create_symlink(source: &Path, target: &Path) -> Result<()> {
        // On Windows, copy the file instead of creating symlink
        fs::copy(source, target).context("Failed to copy config file")?;
        Ok(())
    }

    /// Get default templates for common providers
    pub fn get_template(provider: &str) -> &'static str {
        match provider {
            "openai" => include_str!("../../../templates/codex/openai.toml"),
            "minimax" => include_str!("../../../templates/codex/minimax.toml"),
            "rightcode" => include_str!("../../../templates/codex/rightcode.toml"),
            _ => include_str!("../../../templates/codex/custom.toml"),
        }
    }
}

/// Template provider enum
#[derive(Debug, Clone, Copy)]
pub enum TemplateProvider {
    OpenAI,
    MiniMax,
    RightCode,
    Custom,
}

impl TemplateProvider {
    pub fn all() -> Vec<Self> {
        vec![Self::OpenAI, Self::MiniMax, Self::RightCode, Self::Custom]
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::OpenAI => "OpenAI (gpt-5-codex)",
            Self::MiniMax => "MiniMax (codex-MiniMax-M2)",
            Self::RightCode => "RightCode (gpt-5.1-codex)",
            Self::Custom => "Custom",
        }
    }

    pub fn key(&self) -> &str {
        match self {
            Self::OpenAI => "openai",
            Self::MiniMax => "minimax",
            Self::RightCode => "rightcode",
            Self::Custom => "custom",
        }
    }

    pub fn template(&self) -> &'static str {
        ConfigManager::get_template(self.key())
    }
}
