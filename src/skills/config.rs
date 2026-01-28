use crate::config::resolve_config_dir;
use crate::skills::types::{ConfigEntry, SkillsConfig, SkillsPaths, TargetMode};
use anyhow::{Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SKILLS_DIR: &str = "skills";
const CONFIG_FILE: &str = "config.toml";
const REGISTRY_FILE: &str = "registry.json";
const STORE_DIR: &str = "store";

#[derive(Deserialize, Debug)]
struct TomlConfig {
    version: Option<u32>,
    #[serde(default)]
    source: Vec<TomlEntry>,
    #[serde(default)]
    target: Vec<TomlEntry>,
}

#[derive(Deserialize, Debug)]
struct TomlEntry {
    id: String,
    agent: String,
    scope: String,
    path: String,
    mode: Option<String>,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool {
    true
}

impl SkillsPaths {
    pub fn resolve() -> Result<Self> {
        let config_dir = resolve_config_dir(None)?;
        let root = config_dir.join(SKILLS_DIR);
        Ok(Self {
            root: root.clone(),
            store_dir: root.join(STORE_DIR),
            registry_path: root.join(REGISTRY_FILE),
            config_path: root.join(CONFIG_FILE),
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.root)?;
        fs::create_dir_all(&self.store_dir)?;
        Ok(())
    }
}

pub fn load_config(paths: &SkillsPaths, repo_root: Option<PathBuf>) -> Result<SkillsConfig> {
    let base = if paths.config_path.exists() {
        let content = fs::read_to_string(&paths.config_path)
            .map_err(|e| anyhow!(t!("skills.config.read_failed", error = e)))?;
        let parsed: TomlConfig = toml::from_str(&content)
            .map_err(|e| anyhow!(t!("skills.config.parse_failed", error = e)))?;
        let version = parsed.version.unwrap_or(1);
        if version != 1 {
            return Err(anyhow!(t!(
                "skills.config.unsupported_version",
                version = version
            )));
        }
        let sources = resolve_source_entries(parsed.source)?;
        let targets = resolve_target_entries(parsed.target)?;
        SkillsConfig {
            sources,
            targets,
            repo_root: None,
        }
    } else {
        SkillsConfig {
            sources: default_sources()?,
            targets: default_targets()?,
            repo_root: None,
        }
    };

    Ok(merge_repo_scope(base, repo_root))
}

fn resolve_source_entries(entries: Vec<TomlEntry>) -> Result<Vec<ConfigEntry>> {
    let mut resolved = Vec::new();
    for entry in entries {
        let path = expand_path(&entry.path)?;
        resolved.push(ConfigEntry {
            id: entry.id,
            agent: entry.agent,
            scope: entry.scope,
            path,
            enabled: entry.enabled,
            mode: TargetMode::Link,
        });
    }
    Ok(resolved)
}

fn resolve_target_entries(entries: Vec<TomlEntry>) -> Result<Vec<ConfigEntry>> {
    let mut resolved = Vec::new();
    for entry in entries {
        let path = expand_path(&entry.path)?;
        let mode = parse_target_mode(entry.mode.as_deref())?;
        resolved.push(ConfigEntry {
            id: entry.id,
            agent: entry.agent,
            scope: entry.scope,
            path,
            enabled: entry.enabled,
            mode,
        });
    }
    Ok(resolved)
}

fn parse_target_mode(raw: Option<&str>) -> Result<TargetMode> {
    match raw.unwrap_or("link") {
        "link" => Ok(TargetMode::Link),
        "copy" => Ok(TargetMode::Copy),
        "skip" => Ok(TargetMode::Skip),
        other => Err(anyhow!(t!(
            "skills.config.invalid_target_mode",
            mode = other
        ))),
    }
}

fn default_sources() -> Result<Vec<ConfigEntry>> {
    Ok(vec![
        ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: default_claude_user_dir()?,
            enabled: true,
            mode: TargetMode::Link,
        },
        ConfigEntry {
            id: "codex_user".to_string(),
            agent: "codex".to_string(),
            scope: "user".to_string(),
            path: default_codex_user_dir()?,
            enabled: true,
            mode: TargetMode::Link,
        },
        ConfigEntry {
            id: "agent_global".to_string(),
            agent: "agent".to_string(),
            scope: "global".to_string(),
            path: default_agent_global_dir()?,
            enabled: true,
            mode: TargetMode::Link,
        },
    ])
}

fn default_targets() -> Result<Vec<ConfigEntry>> {
    Ok(vec![
        ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: default_claude_user_dir()?,
            enabled: true,
            mode: TargetMode::Link,
        },
        ConfigEntry {
            id: "codex_user".to_string(),
            agent: "codex".to_string(),
            scope: "user".to_string(),
            path: default_codex_user_dir()?,
            enabled: true,
            mode: TargetMode::Link,
        },
        ConfigEntry {
            id: "agent_global".to_string(),
            agent: "agent".to_string(),
            scope: "global".to_string(),
            path: default_agent_global_dir()?,
            enabled: true,
            mode: TargetMode::Link,
        },
    ])
}

fn merge_repo_scope(mut base: SkillsConfig, repo_root: Option<PathBuf>) -> SkillsConfig {
    if let Some(root) = repo_root.clone() {
        let repo_sources = repo_entries(&root);
        append_unique(&mut base.sources, repo_sources);
        let repo_targets = repo_entries(&root);
        append_unique(&mut base.targets, repo_targets);
        base.repo_root = Some(root);
    }
    base
}

fn repo_entries(root: &Path) -> Vec<ConfigEntry> {
    vec![
        ConfigEntry {
            id: "claude_repo".to_string(),
            agent: "claude".to_string(),
            scope: "repo".to_string(),
            path: root.join(".claude").join("skills"),
            enabled: true,
            mode: TargetMode::Link,
        },
        ConfigEntry {
            id: "codex_repo".to_string(),
            agent: "codex".to_string(),
            scope: "repo".to_string(),
            path: root.join(".codex").join("skills"),
            enabled: true,
            mode: TargetMode::Link,
        },
    ]
}

fn append_unique(into: &mut Vec<ConfigEntry>, extras: Vec<ConfigEntry>) {
    for entry in extras {
        if into.iter().any(|existing| existing.id == entry.id) {
            continue;
        }
        into.push(entry);
    }
}

fn default_claude_user_dir() -> Result<PathBuf> {
    if let Ok(home) = env::var("CLAUDE_HOME") {
        return Ok(PathBuf::from(home).join("skills"));
    }
    expand_path("~/.claude/skills")
}

fn default_codex_user_dir() -> Result<PathBuf> {
    if let Ok(home) = env::var("CODEX_HOME") {
        return Ok(PathBuf::from(home).join("skills"));
    }
    expand_path("~/.codex/skills")
}

fn default_agent_global_dir() -> Result<PathBuf> {
    expand_path("~/.skills")
}

fn expand_path(raw: &str) -> Result<PathBuf> {
    let expanded = expand_env_vars(raw);
    let path = expand_tilde(&expanded)?;
    Ok(PathBuf::from(path))
}

fn expand_tilde(path: &str) -> Result<String> {
    if path == "~" || path.starts_with("~/") {
        let home = dirs::home_dir().ok_or_else(|| anyhow!(t!("skills.config.home_missing")))?;
        if path == "~" {
            return Ok(home.to_string_lossy().to_string());
        }
        let trimmed = path.trim_start_matches("~/");
        return Ok(home.join(trimmed).to_string_lossy().to_string());
    }
    Ok(path.to_string())
}

fn expand_env_vars(input: &str) -> String {
    let re = Regex::new(r"\$([A-Za-z0-9_]+)|\$\{([^}]+)\}").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let key = caps.get(1).or_else(|| caps.get(2)).unwrap().as_str();
        env::var(key).unwrap_or_else(|_| caps.get(0).unwrap().as_str().to_string())
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::ENV_MUTEX;
    use tempfile::TempDir;

    #[test]
    fn test_expand_env_vars() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            env::set_var("SKILLS_TEST_PATH", "/tmp/skills-test");
        }
        let expanded = expand_env_vars("$SKILLS_TEST_PATH/dir");
        assert_eq!(expanded, "/tmp/skills-test/dir");
        unsafe {
            env::remove_var("SKILLS_TEST_PATH");
        }
    }

    #[test]
    fn test_load_default_config() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("temp dir");
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let home_root = temp.path().join("home");
        fs::create_dir_all(&home_root).expect("home root");
        unsafe {
            env::set_var("LLMAN_CONFIG_DIR", &config_dir);
            env::set_var("HOME", &home_root);
            env::set_var("CLAUDE_HOME", home_root.join("claude"));
            env::set_var("CODEX_HOME", home_root.join("codex"));
        }
        let paths = SkillsPaths::resolve().expect("paths");
        let config = load_config(&paths, None).expect("config");
        assert!(!config.sources.is_empty());
        assert!(!config.targets.is_empty());
        unsafe {
            env::remove_var("LLMAN_CONFIG_DIR");
            env::remove_var("HOME");
            env::remove_var("CLAUDE_HOME");
            env::remove_var("CODEX_HOME");
        }
    }

    #[test]
    fn test_repo_auto_discovery() {
        let temp = TempDir::new().expect("temp dir");
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let skills_root = config_dir.join("skills");
        let paths = SkillsPaths {
            root: skills_root.clone(),
            store_dir: skills_root.join("store"),
            registry_path: skills_root.join("registry.json"),
            config_path: skills_root.join("config.toml"),
        };
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&repo_root).expect("create repo root");
        let config = load_config(&paths, Some(repo_root)).expect("config");
        assert!(config.sources.iter().any(|entry| entry.id == "claude_repo"));
        assert!(config.targets.iter().any(|entry| entry.id == "codex_repo"));
    }
}
