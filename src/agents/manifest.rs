use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const AGENT_MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentManifestV1 {
    pub version: u32,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub includes: Vec<String>,
    #[serde(default)]
    pub skills: Vec<AgentSkillMetaV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSkillMetaV1 {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl AgentManifestV1 {
    pub fn new(id: impl Into<String>, includes: Vec<String>) -> Self {
        let id = id.into();
        let includes = includes
            .into_iter()
            .filter(|skill_id| skill_id != &id)
            .collect::<Vec<_>>();
        Self {
            version: AGENT_MANIFEST_VERSION,
            id,
            description: None,
            includes,
            skills: Vec::new(),
        }
    }

    pub fn normalize(&mut self) {
        self.includes.retain(|skill_id| skill_id != &self.id);
        self.includes.sort();
        self.includes.dedup();
        self.skills.sort_by(|a, b| a.id.cmp(&b.id));
        self.skills
            .dedup_by(|a, b| a.id == b.id && a.path == b.path);
    }

    pub fn write_to_path(&self, path: &Path) -> Result<()> {
        let raw = toml::to_string_pretty(self).context("serialize agent.toml")?;
        fs::write(path, raw).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}

pub fn load_agent_manifest_v1(path: &Path) -> Result<AgentManifestV1> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut parsed: AgentManifestV1 =
        toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    if parsed.version != AGENT_MANIFEST_VERSION {
        return Err(anyhow!(
            "Unsupported agent manifest version {} in {} (expected {})",
            parsed.version,
            path.display(),
            AGENT_MANIFEST_VERSION
        ));
    }
    if parsed.id.trim().is_empty() {
        return Err(anyhow!("Agent manifest id is empty in {}", path.display()));
    }
    parsed.normalize();
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn new_manifest_filters_self_from_includes() {
        let manifest = AgentManifestV1::new(
            "foo",
            vec!["foo".to_string(), "bar".to_string(), "bar".to_string()],
        );
        assert_eq!(
            manifest.includes,
            vec!["bar".to_string(), "bar".to_string()]
        );
    }

    #[test]
    fn normalize_sorts_and_dedupes_includes() {
        let mut manifest = AgentManifestV1 {
            version: 1,
            id: "foo".to_string(),
            description: None,
            includes: vec![
                "b".to_string(),
                "foo".to_string(),
                "a".to_string(),
                "b".to_string(),
            ],
            skills: vec![],
        };
        manifest.normalize();
        assert_eq!(manifest.includes, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn roundtrip_preserves_includes_and_skills_meta() {
        let temp = TempDir::new().expect("temp dir");
        let path: PathBuf = temp.path().join("agent.toml");
        let mut manifest = AgentManifestV1 {
            version: 1,
            id: "foo".to_string(),
            description: Some("desc".to_string()),
            includes: vec!["bar".to_string()],
            skills: vec![AgentSkillMetaV1 {
                id: "bar".to_string(),
                path: Some("/tmp/bar".to_string()),
            }],
        };
        manifest.normalize();
        manifest.write_to_path(&path).expect("write");

        let loaded = load_agent_manifest_v1(&path).expect("load");
        assert_eq!(loaded.id, "foo");
        assert_eq!(loaded.includes, vec!["bar".to_string()]);
        assert_eq!(loaded.skills.len(), 1);
        assert_eq!(loaded.skills[0].id, "bar");
        assert_eq!(loaded.skills[0].path.as_deref(), Some("/tmp/bar"));
    }
}
