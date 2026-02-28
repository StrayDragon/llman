use crate::config::resolve_config_dir;
use crate::config_schema::{ConfigSchemaKind, validate_yaml_value};
use crate::path_utils::validate_path_str;
use crate::skills::catalog::types::{ConfigEntry, SkillsConfig, SkillsPaths, TargetMode};
use crate::skills::shared::git::find_git_root;
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

    let config_dir = resolve_config_dir(None)?;
    resolve_skills_root_with(None, None, &config_dir)
}

fn resolve_skills_root_with(
    cli_override: Option<&Path>,
    env_skills_dir: Option<&str>,
    config_dir: &Path,
) -> Result<PathBuf> {
    if let Some(path) = cli_override {
        return resolve_skills_root_from_cli(path);
    }

    if let Some(env_skills_dir) = env_skills_dir {
        return resolve_skills_root_from_env(env_skills_dir);
    }

    let global_config = config_dir.join(LLMAN_CONFIG_FILE);
    if let Some(config_dir) = load_skills_root_from_config_path(&global_config)? {
        return Ok(config_dir);
    }

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
        let mode = parse_target_mode(entry.mode.as_deref())?;
        let path = expand_path(&entry.path)?;
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
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let home_dir = crate::config::try_home_dir();
    let claude_home = env::var("CLAUDE_HOME").ok().map(PathBuf::from);
    let codex_home = env::var("CODEX_HOME").ok().map(PathBuf::from);
    default_targets_with(
        &cwd,
        home_dir.as_deref(),
        claude_home.as_deref(),
        codex_home.as_deref(),
    )
}

fn default_targets_with(
    cwd: &Path,
    home_dir: Option<&Path>,
    claude_home: Option<&Path>,
    codex_home: Option<&Path>,
) -> Result<Vec<ConfigEntry>> {
    let (claude_project_path, claude_project_mode) = default_repo_scope_dir(cwd, ".claude/skills");
    let (codex_repo_path, codex_repo_mode) = default_repo_scope_dir(cwd, ".agents/skills");

    Ok(vec![
        ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: default_claude_user_dir_with(claude_home, home_dir)?,
            enabled: true,
            mode: TargetMode::Link,
        },
        ConfigEntry {
            id: "claude_project".to_string(),
            agent: "claude".to_string(),
            scope: "project".to_string(),
            path: claude_project_path,
            enabled: true,
            mode: claude_project_mode,
        },
        ConfigEntry {
            id: "codex_user".to_string(),
            agent: "codex".to_string(),
            scope: "user".to_string(),
            path: default_codex_user_dir_with(codex_home, home_dir)?,
            enabled: true,
            mode: TargetMode::Link,
        },
        ConfigEntry {
            id: "codex_repo".to_string(),
            agent: "codex".to_string(),
            scope: "repo".to_string(),
            path: codex_repo_path,
            enabled: true,
            mode: codex_repo_mode,
        },
        ConfigEntry {
            id: "agent_global".to_string(),
            agent: "agent".to_string(),
            scope: "global".to_string(),
            path: default_agent_global_dir_with(home_dir)?,
            enabled: true,
            mode: TargetMode::Link,
        },
    ])
}

fn default_claude_user_dir_with(
    claude_home: Option<&Path>,
    home_dir: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(home) = claude_home {
        return Ok(home.join("skills"));
    }
    expand_path_with("~/.claude/skills", home_dir, |_key| None)
}

fn default_codex_user_dir_with(
    codex_home: Option<&Path>,
    home_dir: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(home) = codex_home {
        let preferred = home.join(".agents").join("skills");
        if preferred.exists() {
            return Ok(preferred);
        }
        let legacy = home.join("skills");
        if legacy.exists() {
            return Ok(legacy);
        }
        return Ok(preferred);
    }

    let preferred = expand_path_with("~/.agents/skills", home_dir, |_key| None)?;
    if preferred.exists() {
        return Ok(preferred);
    }
    let legacy = expand_path_with("~/.codex/skills", home_dir, |_key| None)?;
    if legacy.exists() {
        return Ok(legacy);
    }
    Ok(preferred)
}

fn default_agent_global_dir_with(home_dir: Option<&Path>) -> Result<PathBuf> {
    expand_path_with("~/.skills", home_dir, |_key| None)
}

fn default_repo_scope_dir(cwd: &Path, relative: &str) -> (PathBuf, TargetMode) {
    if let Some(repo_root) = find_git_root(cwd) {
        return (repo_root.join(relative), TargetMode::Link);
    }
    (cwd.join(relative), TargetMode::Skip)
}

fn expand_path(raw: &str) -> Result<PathBuf> {
    expand_path_with(raw, None, |key| env::var(key).ok())
}

fn expand_path_with<F>(raw: &str, home_dir: Option<&Path>, env_lookup: F) -> Result<PathBuf>
where
    F: Fn(&str) -> Option<String>,
{
    let expanded = expand_env_vars_with(raw, env_lookup);
    let path = expand_tilde_with(&expanded, home_dir)?;
    Ok(PathBuf::from(path))
}

fn expand_tilde_with(path: &str, home_dir: Option<&Path>) -> Result<String> {
    if path == "~" || path.starts_with("~/") {
        let home = home_dir
            .map(Path::to_path_buf)
            .or_else(crate::config::try_home_dir)
            .ok_or_else(|| anyhow!(t!("skills.config.home_missing")))?;
        if path == "~" {
            return Ok(home.to_string_lossy().to_string());
        }
        let trimmed = path.trim_start_matches("~/");
        return Ok(home.join(trimmed).to_string_lossy().to_string());
    }
    Ok(path.to_string())
}

fn expand_env_vars_with<F>(input: &str, lookup: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let re = Regex::new(r"\$([A-Za-z0-9_]+)|\$\{([^}]+)\}").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let key = caps.get(1).or_else(|| caps.get(2)).unwrap().as_str();
        lookup(key).unwrap_or_else(|| caps.get(0).unwrap().as_str().to_string())
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_expand_env_vars() {
        let vars: HashMap<&str, &str> = HashMap::from([("SKILLS_TEST_PATH", "/tmp/skills-test")]);
        let expanded = expand_env_vars_with("$SKILLS_TEST_PATH/dir", |key| {
            vars.get(key).map(|value| (*value).to_string())
        });
        assert_eq!(expanded, "/tmp/skills-test/dir");
    }

    #[test]
    fn test_load_default_config() {
        let temp = TempDir::new().expect("temp dir");
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let root = resolve_skills_root_with(None, None, &config_dir).expect("paths");
        let paths = SkillsPaths {
            root: root.clone(),
            config_path: root.join(CONFIG_FILE),
        };
        assert_eq!(paths.root, config_dir.join("skills"));
        let config = load_config(&paths).expect("config");
        assert!(!config.targets.is_empty());
    }

    #[test]
    fn test_resolve_skills_root_cli_overrides_env_and_config() {
        let temp = TempDir::new().expect("temp dir");
        let cli_root = temp.path().join("cli-root");
        let env_root = temp.path().join("env-root");
        let config_dir = temp.path().join("config");

        let resolved = resolve_skills_root_with(
            Some(cli_root.as_path()),
            Some(env_root.to_str().unwrap()),
            &config_dir,
        )
        .expect("paths");
        assert_eq!(resolved, cli_root);
    }

    #[test]
    fn test_resolve_skills_root_env_overrides_config() {
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
        let resolved =
            resolve_skills_root_with(None, Some(env_root.to_str().unwrap()), &config_dir)
                .expect("paths");
        assert_eq!(resolved, env_root);
    }

    #[test]
    fn test_resolve_skills_root_local_config_ignored() {
        let temp = TempDir::new().expect("temp dir");
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
        let resolved = resolve_skills_root_with(None, None, &config_dir).expect("paths");
        assert_eq!(resolved, global_root);
    }

    #[test]
    fn test_resolve_skills_root_global_config_fallback() {
        let temp = TempDir::new().expect("temp dir");
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
        let resolved = resolve_skills_root_with(None, None, &config_dir).expect("paths");
        assert_eq!(resolved, global_root);
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
            config_path: skills_root.join("config.toml"),
        };
        let err = load_config(&paths).expect_err("should reject copy mode");
        assert!(err.to_string().contains("Unsupported target mode"));
    }

    #[test]
    fn test_default_targets_include_repo_scopes_inside_git_repo() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path().join("repo");
        let nested = repo_root.join("nested");
        fs::create_dir_all(repo_root.join(".git")).expect("create .git");
        fs::create_dir_all(&nested).expect("create nested");

        let home_root = temp.path().join("home");
        fs::create_dir_all(&home_root).expect("create home");
        let targets =
            default_targets_with(&nested, Some(&home_root), None, None).expect("default targets");

        let claude_project = targets
            .iter()
            .find(|target| target.id == "claude_project")
            .expect("claude_project target");
        assert_eq!(claude_project.mode, TargetMode::Link);
        assert_eq!(claude_project.path, repo_root.join(".claude/skills"));

        let codex_repo = targets
            .iter()
            .find(|target| target.id == "codex_repo")
            .expect("codex_repo target");
        assert_eq!(codex_repo.mode, TargetMode::Link);
        assert_eq!(codex_repo.path, repo_root.join(".agents/skills"));
    }

    #[test]
    fn test_default_targets_mark_repo_scopes_read_only_outside_git_repo() {
        let temp = TempDir::new().expect("temp dir");
        let cwd = temp.path().join("work");
        fs::create_dir_all(&cwd).expect("create cwd");

        let home_root = temp.path().join("home");
        fs::create_dir_all(&home_root).expect("create home");
        let targets = default_targets_with(&cwd, Some(&home_root), None, None).expect("default");

        let claude_project = targets
            .iter()
            .find(|target| target.id == "claude_project")
            .expect("claude_project target");
        assert_eq!(claude_project.mode, TargetMode::Skip);

        let codex_repo = targets
            .iter()
            .find(|target| target.id == "codex_repo")
            .expect("codex_repo target");
        assert_eq!(codex_repo.mode, TargetMode::Skip);
    }

    #[test]
    fn test_codex_user_prefers_agents_skills_path() {
        let temp = TempDir::new().expect("temp dir");
        let home_root = temp.path().join("home");
        fs::create_dir_all(home_root.join(".agents/skills")).expect("create agents skills");
        fs::create_dir_all(home_root.join(".codex/skills")).expect("create codex skills");

        let path = default_codex_user_dir_with(None, Some(&home_root)).expect("codex user dir");
        assert_eq!(path, home_root.join(".agents/skills"));
    }

    #[test]
    fn test_codex_user_falls_back_to_codex_skills_when_agents_missing() {
        let temp = TempDir::new().expect("temp dir");
        let home_root = temp.path().join("home");
        fs::create_dir_all(home_root.join(".codex/skills")).expect("create codex skills");

        let path = default_codex_user_dir_with(None, Some(&home_root)).expect("codex user dir");
        assert_eq!(path, home_root.join(".codex/skills"));
    }
}
