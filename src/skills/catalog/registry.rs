use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Registry {
    pub skills: HashMap<String, SkillEntry>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SkillEntry {
    #[serde(default)]
    pub targets: HashMap<String, bool>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl Registry {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!(t!("skills.registry.read_failed", error = e)))?;
        let parsed = serde_json::from_str(&content)
            .map_err(|e| anyhow!(t!("skills.registry.parse_failed", error = e)))?;
        Ok(parsed)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| anyhow!(t!("skills.registry.write_failed", error = e)))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    pub fn ensure_skill(&mut self, skill_id: &str) -> &mut SkillEntry {
        self.skills.entry(skill_id.to_string()).or_default()
    }
}
