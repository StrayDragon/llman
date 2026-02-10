use crate::path_utils::validate_path_str;
use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const ENV_CONFIG_DIR: &str = "LLMAN_CONFIG_DIR";
pub const ENV_LANG: &str = "LLMAN_LANG";
pub const APP_NAME: &str = "llman";
pub const CURSOR_APP: &str = "cursor";
pub const CODEX_APP: &str = "codex";
pub const CLAUDE_CODE_APP: &str = "claude-code";
pub const CURSOR_EXTENSION: &str = "mdc";
pub const CODEX_EXTENSION: &str = "md";
pub const DEFAULT_EXTENSION: &str = "txt";
pub const PROMPT_DIR: &str = "prompt";
pub const TARGET_CURSOR_RULES_DIR: &str = ".cursor/rules";

fn rule_extension_for_app(app: &str) -> &'static str {
    match app {
        CURSOR_APP => CURSOR_EXTENSION,
        CODEX_APP => CODEX_EXTENSION,
        _ => DEFAULT_EXTENSION,
    }
}

pub fn resolve_config_dir(cli_override: Option<&Path>) -> Result<PathBuf> {
    let env_override = env::var(ENV_CONFIG_DIR).ok();
    resolve_config_dir_with(cli_override, env_override.as_deref())
}

pub fn resolve_config_dir_with(
    cli_override: Option<&Path>,
    env_override: Option<&str>,
) -> Result<PathBuf> {
    if let Some(path) = cli_override {
        validate_path_str(&path.to_string_lossy())
            .map_err(|e| anyhow!(t!("errors.invalid_config_dir", error = e)))?;
        return Ok(path.to_path_buf());
    }

    if let Some(env_config_dir) = env_override {
        validate_path_str(env_config_dir)
            .map_err(|e| anyhow!(t!("errors.invalid_config_dir_env", error = e)))?;
        return Ok(PathBuf::from(env_config_dir));
    }

    let project_dirs = ProjectDirs::from("", "", APP_NAME)
        .ok_or_else(|| anyhow!(t!("errors.not_find_config_dir")))?;
    Ok(project_dirs.config_dir().to_path_buf())
}

pub struct Config {
    config_dir: PathBuf,
    prompt_dir: PathBuf,
}

impl Config {
    #[allow(dead_code)]
    pub fn new() -> Result<Self> {
        Self::with_config_dir(None)
    }

    pub fn with_config_dir(config_dir_override: Option<&str>) -> Result<Self> {
        let config_dir = resolve_config_dir(config_dir_override.map(Path::new))?;

        let prompt_dir = config_dir.join(PROMPT_DIR);

        fs::create_dir_all(&config_dir)?;

        Ok(Self {
            config_dir,
            prompt_dir,
        })
    }

    #[allow(dead_code)]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    #[allow(dead_code)]
    pub fn prompt_dir(&self) -> &Path {
        &self.prompt_dir
    }

    pub fn app_dir(&self, app: &str) -> PathBuf {
        self.prompt_dir.join(app)
    }

    pub fn ensure_app_dir(&self, app: &str) -> Result<PathBuf> {
        let app_dir = self.app_dir(app);
        fs::create_dir_all(&app_dir)?;
        Ok(app_dir)
    }

    pub fn rule_file_path(&self, app: &str, name: &str) -> PathBuf {
        let extension = rule_extension_for_app(app);
        self.app_dir(app).join(format!("{name}.{extension}"))
    }

    pub fn list_rules(&self, app: &str) -> Result<Vec<String>> {
        let app_dir = self.app_dir(app);

        if !app_dir.exists() {
            return Ok(Vec::new());
        }

        let mut rules = Vec::new();
        let expected_extension = rule_extension_for_app(app);
        for entry in fs::read_dir(app_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && path.extension().and_then(|ext| ext.to_str()) == Some(expected_extension)
                && let Some(stem) = path.file_stem()
                && let Some(name) = stem.to_str()
            {
                rules.push(name.to_string());
            }
        }

        rules.sort();
        Ok(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_config_dir_env_override() {
        let temp = TempDir::new().expect("temp dir");
        let temp_dir = temp.path().to_path_buf();
        let resolved = resolve_config_dir_with(None, temp_dir.to_str()).unwrap();
        assert_eq!(resolved, temp_dir);
    }

    #[test]
    fn test_resolve_config_dir_cli_overrides_env() {
        let env_temp = TempDir::new().expect("temp dir");
        let cli_temp = TempDir::new().expect("temp dir");
        let env_dir = env_temp.path().to_path_buf();
        let cli_dir = cli_temp.path().to_path_buf();

        let resolved = resolve_config_dir_with(Some(cli_dir.as_path()), env_dir.to_str()).unwrap();
        assert_eq!(resolved, cli_dir);
    }

    #[test]
    fn test_resolve_config_dir_env_overrides_default() {
        let env_temp = TempDir::new().expect("temp dir");
        let env_dir = env_temp.path().to_path_buf();

        let resolved = resolve_config_dir_with(None, env_dir.to_str()).unwrap();
        assert_eq!(resolved, env_dir);
    }

    #[test]
    fn test_resolve_config_dir_default_path() {
        let resolved = resolve_config_dir_with(None, None).unwrap();
        let expected = ProjectDirs::from("", "", APP_NAME)
            .unwrap()
            .config_dir()
            .to_path_buf();

        assert_eq!(resolved, expected);
    }

    #[test]
    fn test_app_dir() {
        let temp = TempDir::new().expect("temp dir");
        let temp_dir = temp.path().to_path_buf();

        let config = Config::with_config_dir(temp_dir.to_str()).unwrap();
        let cursor_dir = config.app_dir("cursor");
        assert_eq!(cursor_dir, temp_dir.join("prompt").join("cursor"));
    }

    #[test]
    fn test_rule_file_path() {
        let temp = TempDir::new().expect("temp dir");
        let temp_dir = temp.path().to_path_buf();

        let config = Config::with_config_dir(temp_dir.to_str()).unwrap();
        let rule_path = config.rule_file_path("cursor", "test-rule");
        assert_eq!(
            rule_path,
            temp_dir.join("prompt").join("cursor").join("test-rule.mdc")
        );
    }

    #[test]
    fn test_rule_file_path_codex_uses_md() {
        let temp = TempDir::new().expect("temp dir");
        let temp_dir = temp.path().to_path_buf();

        let config = Config::with_config_dir(temp_dir.to_str()).unwrap();
        let rule_path = config.rule_file_path(CODEX_APP, "draftpr");
        assert_eq!(
            rule_path,
            temp_dir.join("prompt").join(CODEX_APP).join("draftpr.md")
        );
    }

    #[test]
    fn test_list_rules_filters_by_extension_per_app() {
        let temp = TempDir::new().expect("temp dir");
        let config = Config::with_config_dir(temp.path().to_str()).unwrap();

        let cursor_dir = config.ensure_app_dir(CURSOR_APP).unwrap();
        let codex_dir = config.ensure_app_dir(CODEX_APP).unwrap();
        let claude_dir = config.ensure_app_dir(CLAUDE_CODE_APP).unwrap();

        fs::write(cursor_dir.join("keep.mdc"), "x").expect("write");
        fs::write(cursor_dir.join("ignore.txt"), "x").expect("write");
        fs::write(cursor_dir.join("backup.mdc.bak"), "x").expect("write");

        fs::write(codex_dir.join("draft.md"), "x").expect("write");
        fs::write(codex_dir.join("draft.md.bak"), "x").expect("write");

        fs::write(claude_dir.join("mem.txt"), "x").expect("write");
        fs::write(claude_dir.join("mem.md"), "x").expect("write");

        assert_eq!(config.list_rules(CURSOR_APP).unwrap(), vec!["keep"]);
        assert_eq!(config.list_rules(CODEX_APP).unwrap(), vec!["draft"]);
        assert_eq!(config.list_rules(CLAUDE_CODE_APP).unwrap(), vec!["mem"]);
    }
}
