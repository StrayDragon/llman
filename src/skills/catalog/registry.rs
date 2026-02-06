use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "registry.json".into());
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp_name = format!(".{file_name}.tmp.{nanos}");
        let tmp_path: PathBuf = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.join(tmp_name),
            _ => PathBuf::from(tmp_name),
        };

        fs::write(&tmp_path, content)
            .map_err(|e| anyhow!(t!("skills.registry.write_failed", error = e)))?;

        #[cfg(windows)]
        {
            // Best-effort replace semantics. This is not fully atomic on Windows,
            // but avoids partial writes corrupting the registry file.
            if path.exists() {
                fs::remove_file(path)
                    .map_err(|e| anyhow!(t!("skills.registry.write_failed", error = e)))?;
            }
        }

        fs::rename(&tmp_path, path)
            .map_err(|e| anyhow!(t!("skills.registry.write_failed", error = e)))?;
        Ok(())
    }

    pub fn ensure_skill(&mut self, skill_id: &str) -> &mut SkillEntry {
        self.skills.entry(skill_id.to_string()).or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_writes_valid_json() {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join("registry.json");
        let mut registry = Registry::default();
        registry.ensure_skill("alpha");
        registry.save(&path).expect("save");

        let content = fs::read_to_string(&path).expect("read");
        let parsed: Registry = serde_json::from_str(&content).expect("parse json");
        assert!(parsed.skills.contains_key("alpha"));
    }

    #[cfg(unix)]
    #[test]
    fn save_does_not_corrupt_existing_file_on_write_failure() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().expect("temp dir");
        let dir = temp.path().join("dir");
        fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("registry.json");

        let mut registry = Registry::default();
        registry.ensure_skill("alpha");
        registry.save(&path).expect("save");

        let original = fs::read_to_string(&path).expect("read");
        serde_json::from_str::<Registry>(&original).expect("parse original");

        // Make directory non-writable so the temp write fails.
        let mut perms = fs::metadata(&dir).expect("meta").permissions();
        perms.set_mode(0o500);
        fs::set_permissions(&dir, perms).expect("set perms");

        registry.ensure_skill("beta");
        let _err = registry.save(&path).expect_err("save should fail");

        // Restore permissions so temp dir cleanup works.
        let mut restore = fs::metadata(&dir).expect("meta").permissions();
        restore.set_mode(0o700);
        fs::set_permissions(&dir, restore).expect("restore perms");

        let after = fs::read_to_string(&path).expect("read");
        assert_eq!(after, original);
        serde_json::from_str::<Registry>(&after).expect("parse after");
    }
}
