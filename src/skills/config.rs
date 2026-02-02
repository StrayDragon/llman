use crate::config::resolve_config_dir;
use crate::config_schema::{ConfigSchemaKind, validate_yaml_value};
use crate::path_utils::validate_path_str;
use crate::skills::types::{ConfigEntry, SkillsConfig, SkillsPaths, TargetMode};
use anyhow::{Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ENV_SKILLS_DIR: &str = "LLMAN_SKILLS_DIR";
const LLMAN_CONFIG_FILE: &str = "config.yaml";
const SKILLS_DIR: &str = "skills";
const CONFIG_FILE: &str = "config.toml";
const REGISTRY_FILE: &str = "registry.json";

#[derive(Deserialize, Debug)]
struct TomlConfig {
    version: Option<u32>,
    #[serde(default)]
    target: Vec<TomlEntry>,
    #[serde(default)]
    source: Vec<TomlEntry>,
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

#[derive(Deserialize, Debug)]
struct LlmanConfig {
    skills: Option<LlmanSkillsConfig>,
}

#[derive(Deserialize, Debug)]
struct LlmanSkillsConfig {
    dir: Option<String>,
}

fn default_true() -> bool {
    true
}

impl SkillsPaths {
    pub fn resolve() -> Result<Self> {
        Self::resolve_with_override(None)
    }

    pub fn resolve_with_override(cli_override: Option<&Path>) -> Result<Self> {
        let root = resolve_skills_root(cli_override)?;
        Ok(Self {
            root: root.clone(),
            registry_path: root.join(REGISTRY_FILE),
            config_path: root.join(CONFIG_FILE),
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.root)?;
        Ok(())
    }
}

fn resolve_skills_root(cli_override: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = cli_override {
        return resolve_skills_root_from_cli(path);
    }

    if let Ok(env_skills_dir) = env::var(ENV_SKILLS_DIR) {
        return resolve_skills_root_from_env(&env_skills_dir);
    }

    if let Some(config_dir) = load_skills_root_from_llman_config()? {
        return Ok(config_dir);
    }

    let config_dir = resolve_config_dir(None)?;
    Ok(config_dir.join(SKILLS_DIR))
}

fn resolve_skills_root_from_cli(path: &Path) -> Result<PathBuf> {
    let raw = path.to_string_lossy();
    validate_path_str(&raw)
        .map_err(|e| anyhow!(t!("skills.config.skills_dir_invalid_cli", error = e)))?;
    expand_path(&raw)
}

fn resolve_skills_root_from_env(raw: &str) -> Result<PathBuf> {
    validate_path_str(raw)
        .map_err(|e| anyhow!(t!("skills.config.skills_dir_invalid_env", error = e)))?;
    expand_path(raw)
}

fn resolve_skills_root_from_config(raw: &str) -> Result<PathBuf> {
    validate_path_str(raw)
        .map_err(|e| anyhow!(t!("skills.config.skills_dir_invalid_config", error = e)))?;
    expand_path(raw)
}

fn load_skills_root_from_llman_config() -> Result<Option<PathBuf>> {
    let global_config = resolve_config_dir(None)?.join(LLMAN_CONFIG_FILE);
    load_skills_root_from_config_path(&global_config)
}

fn load_skills_root_from_config_path(path: &Path) -> Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .map_err(|e| anyhow!(t!("skills.config.llman_read_failed", error = e)))?;
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| anyhow!(t!("skills.config.llman_parse_failed", error = e)))?;
    if let Err(error) = validate_yaml_value(ConfigSchemaKind::Global, &yaml_value) {
        return Err(anyhow!(t!(
            "skills.config.llman_schema_invalid",
            path = path.display(),
            error = error
        )));
    }
    let parsed: LlmanConfig = serde_yaml::from_value(yaml_value)
        .map_err(|e| anyhow!(t!("skills.config.llman_parse_failed", error = e)))?;
    if let Some(skills) = parsed.skills
        && let Some(dir) = skills.dir
    {
        return Ok(Some(resolve_skills_root_from_config(&dir)?));
    }

    Ok(None)
}

pub fn load_config(paths: &SkillsPaths) -> Result<SkillsConfig> {
    if paths.config_path.exists() {
        let content = fs::read_to_string(&paths.config_path)
            .map_err(|e| anyhow!(t!("skills.config.read_failed", error = e)))?;
        let parsed: TomlConfig = toml::from_str(&content)
            .map_err(|e| anyhow!(t!("skills.config.parse_failed", error = e)))?;
        let version = parsed.version.unwrap_or(2);
        if version != 2 {
            return Err(anyhow!(t!(
                "skills.config.unsupported_version",
                version = version
            )));
        }
        if !parsed.source.is_empty() {
            return Err(anyhow!(t!("skills.config.sources_removed")));
        }
        let targets = resolve_target_entries(parsed.target)?;
        Ok(SkillsConfig { targets })
    } else {
        Ok(SkillsConfig {
            targets: default_targets()?,
        })
    }
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
        "skip" => Ok(TargetMode::Skip),
        other => Err(anyhow!(t!(
            "skills.config.invalid_target_mode",
            mode = other
        ))),
    }
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

    struct CwdGuard {
        original: PathBuf,
    }

    impl CwdGuard {
        fn new() -> Self {
            Self {
                original: env::current_dir().expect("current dir"),
            }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original);
        }
    }

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
        assert_eq!(paths.root, config_dir.join("skills"));
        let config = load_config(&paths).expect("config");
        assert!(!config.targets.is_empty());
        unsafe {
            env::remove_var("LLMAN_CONFIG_DIR");
            env::remove_var("HOME");
            env::remove_var("CLAUDE_HOME");
            env::remove_var("CODEX_HOME");
        }
    }

    #[test]
    fn test_resolve_skills_root_cli_overrides_env_and_config() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("temp dir");
        let cli_root = temp.path().join("cli-root");
        let env_root = temp.path().join("env-root");
        unsafe {
            env::set_var(ENV_SKILLS_DIR, &env_root);
        }

        let paths = SkillsPaths::resolve_with_override(Some(cli_root.as_path())).expect("paths");
        assert_eq!(paths.root, cli_root);

        unsafe {
            env::remove_var(ENV_SKILLS_DIR);
        }
    }

    #[test]
    fn test_resolve_skills_root_env_overrides_config() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("temp dir");
        let env_root = temp.path().join("env-root");
        let global_root = temp.path().join("global-root");
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&config_dir).expect("create config dir");

        fs::write(
            config_dir.join("config.yaml"),
            format!(
                "version: \"0.1\"\ntools: {{}}\nskills:\n  dir: {}\n",
                global_root.display()
            ),
        )
        .expect("write global config");

        unsafe {
            env::set_var("LLMAN_CONFIG_DIR", &config_dir);
        }

        unsafe {
            env::set_var(ENV_SKILLS_DIR, &env_root);
        }

        let paths = SkillsPaths::resolve().expect("paths");
        assert_eq!(paths.root, env_root);

        unsafe {
            env::remove_var(ENV_SKILLS_DIR);
            env::remove_var("LLMAN_CONFIG_DIR");
        }
    }

    #[test]
    fn test_resolve_skills_root_local_config_ignored() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("temp dir");
        let local_root = temp.path().join("local-root");
        let global_root = temp.path().join("global-root");
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&config_dir).expect("create config dir");

        let _cwd_guard = CwdGuard::new();
        env::set_current_dir(temp.path()).expect("set cwd");
        fs::create_dir_all(temp.path().join(".llman")).expect("create .llman");
        fs::write(
            temp.path().join(".llman").join("config.yaml"),
            format!(
                "version: \"0.1\"\ntools: {{}}\nskills:\n  dir: {}\n",
                local_root.display()
            ),
        )
        .expect("write local config");

        fs::write(
            config_dir.join("config.yaml"),
            format!(
                "version: \"0.1\"\ntools: {{}}\nskills:\n  dir: {}\n",
                global_root.display()
            ),
        )
        .expect("write global config");

        unsafe {
            env::set_var("LLMAN_CONFIG_DIR", &config_dir);
        }

        let paths = SkillsPaths::resolve().expect("paths");
        assert_eq!(paths.root, global_root);

        unsafe {
            env::remove_var("LLMAN_CONFIG_DIR");
        }
    }

    #[test]
    fn test_resolve_skills_root_global_config_fallback() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("temp dir");
        let global_root = temp.path().join("global-root");
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&config_dir).expect("create config dir");

        let _cwd_guard = CwdGuard::new();
        env::set_current_dir(temp.path()).expect("set cwd");
        fs::write(
            config_dir.join("config.yaml"),
            format!(
                "version: \"0.1\"\ntools: {{}}\nskills:\n  dir: {}\n",
                global_root.display()
            ),
        )
        .expect("write global config");

        unsafe {
            env::set_var("LLMAN_CONFIG_DIR", &config_dir);
        }

        let paths = SkillsPaths::resolve().expect("paths");
        assert_eq!(paths.root, global_root);

        unsafe {
            env::remove_var("LLMAN_CONFIG_DIR");
        }
    }

    #[test]
    fn test_rejects_v1_config() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        fs::create_dir_all(&skills_root).expect("create skills root");
        fs::write(
            skills_root.join("config.toml"),
            "version = 1\n\n[[target]]\nid = \"claude\"\nagent = \"claude\"\nscope = \"user\"\npath = \"~/.claude/skills\"\n",
        )
        .expect("write config");
        let paths = SkillsPaths {
            root: skills_root.clone(),
            registry_path: skills_root.join("registry.json"),
            config_path: skills_root.join("config.toml"),
        };
        let err = load_config(&paths).expect_err("should reject v1");
        assert!(
            err.to_string()
                .contains("Unsupported skills config version")
        );
    }

    #[test]
    fn test_rejects_sources_in_v2_config() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        fs::create_dir_all(&skills_root).expect("create skills root");
        fs::write(
            skills_root.join("config.toml"),
            "version = 2\n\n[[source]]\nid = \"claude_user\"\nagent = \"claude\"\nscope = \"user\"\npath = \"~/.claude/skills\"\n\n[[target]]\nid = \"claude_user\"\nagent = \"claude\"\nscope = \"user\"\npath = \"~/.claude/skills\"\n",
        )
        .expect("write config");
        let paths = SkillsPaths {
            root: skills_root.clone(),
            registry_path: skills_root.join("registry.json"),
            config_path: skills_root.join("config.toml"),
        };
        let err = load_config(&paths).expect_err("should reject sources");
        assert!(err.to_string().contains("[[source]]"));
    }

    #[test]
    fn test_rejects_copy_target_mode() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        fs::create_dir_all(&skills_root).expect("create skills root");
        fs::write(
            skills_root.join("config.toml"),
            "version = 2\n\n[[target]]\nid = \"claude_user\"\nagent = \"claude\"\nscope = \"user\"\npath = \"~/.claude/skills\"\nmode = \"copy\"\n",
        )
        .expect("write config");
        let paths = SkillsPaths {
            root: skills_root.clone(),
            registry_path: skills_root.join("registry.json"),
            config_path: skills_root.join("config.toml"),
        };
        let err = load_config(&paths).expect_err("should reject copy mode");
        assert!(err.to_string().contains("Unsupported target mode"));
    }
}
