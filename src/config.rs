use crate::path_utils::validate_path_str;
use anyhow::{Result, anyhow};
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
const GLOBAL_CONFIG_FILE: &str = "config.yaml";

fn rule_extension_for_app(app: &str) -> &'static str {
    match app {
        CURSOR_APP => CURSOR_EXTENSION,
        CODEX_APP => CODEX_EXTENSION,
        _ => DEFAULT_EXTENSION,
    }
}

fn is_recognizable_config_root(config_dir: &Path) -> bool {
    config_dir.join(GLOBAL_CONFIG_FILE).is_file() || config_dir.join(PROMPT_DIR).is_dir()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MacosConfigDirChoice {
    Default,
    LegacyApplicationSupport,
    LegacyBundleIdApplicationSupport,
}

fn select_macos_config_dir_choice(
    default_recognizable: bool,
    legacy_application_support_recognizable: bool,
    legacy_bundle_id_application_support_recognizable: bool,
) -> MacosConfigDirChoice {
    if default_recognizable {
        return MacosConfigDirChoice::Default;
    }
    if legacy_application_support_recognizable {
        return MacosConfigDirChoice::LegacyApplicationSupport;
    }
    if legacy_bundle_id_application_support_recognizable {
        return MacosConfigDirChoice::LegacyBundleIdApplicationSupport;
    }
    MacosConfigDirChoice::Default
}

pub(crate) fn try_home_dir() -> Option<PathBuf> {
    if let Some(home) = env::var_os("HOME")
        && !home.is_empty()
    {
        return Some(PathBuf::from(home));
    }

    #[cfg(windows)]
    {
        if let Some(profile) = env::var_os("USERPROFILE")
            && !profile.is_empty()
        {
            return Some(PathBuf::from(profile));
        }

        let drive = env::var_os("HOMEDRIVE");
        let path = env::var_os("HOMEPATH");
        if let (Some(drive), Some(path)) = (drive, path)
            && !drive.is_empty()
            && !path.is_empty()
        {
            let mut combined = drive;
            combined.push(path);
            return Some(PathBuf::from(combined));
        }
    }

    None
}

pub(crate) fn home_dir() -> Result<PathBuf> {
    try_home_dir().ok_or_else(|| anyhow!(t!("errors.home_dir_missing")))
}

fn expand_tilde_path(path: &Path) -> Result<PathBuf> {
    let Ok(stripped) = path.strip_prefix("~") else {
        return Ok(path.to_path_buf());
    };

    let home = home_dir()?;
    Ok(home.join(stripped))
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
        return expand_tilde_path(path);
    }

    if let Some(env_config_dir) = env_override {
        validate_path_str(env_config_dir)
            .map_err(|e| anyhow!(t!("errors.invalid_config_dir_env", error = e)))?;
        return expand_tilde_path(Path::new(env_config_dir));
    }

    let home = home_dir()?;
    let default_dir = home.join(".config").join(APP_NAME);

    if cfg!(target_os = "macos") {
        let legacy_application_support = home
            .join("Library")
            .join("Application Support")
            .join(APP_NAME);
        let legacy_bundle_id_application_support = home
            .join("Library")
            .join("Application Support")
            .join("com.StrayDragon.llman");

        let choice = select_macos_config_dir_choice(
            is_recognizable_config_root(&default_dir),
            is_recognizable_config_root(&legacy_application_support),
            is_recognizable_config_root(&legacy_bundle_id_application_support),
        );

        match choice {
            MacosConfigDirChoice::Default => Ok(default_dir),
            MacosConfigDirChoice::LegacyApplicationSupport => {
                eprintln!(
                    "{}",
                    t!(
                        "messages.macos_legacy_config_dir_warning",
                        legacy_path = legacy_application_support.display()
                    )
                );
                Ok(legacy_application_support)
            }
            MacosConfigDirChoice::LegacyBundleIdApplicationSupport => {
                eprintln!(
                    "{}",
                    t!(
                        "messages.macos_legacy_config_dir_warning",
                        legacy_path = legacy_bundle_id_application_support.display()
                    )
                );
                Ok(legacy_bundle_id_application_support)
            }
        }
    } else {
        Ok(default_dir)
    }
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
    use crate::test_utils::TestProcess;
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
        let temp_home = TempDir::new().expect("temp home");
        let mut proc = TestProcess::new();
        proc.set_var("HOME", temp_home.path());

        let resolved = resolve_config_dir_with(None, None).unwrap();
        assert_eq!(resolved, temp_home.path().join(".config").join(APP_NAME));
    }

    #[test]
    fn test_select_macos_config_dir_choice_prefers_default_when_recognizable() {
        assert_eq!(
            select_macos_config_dir_choice(true, true, true),
            MacosConfigDirChoice::Default
        );
    }

    #[test]
    fn test_select_macos_config_dir_choice_falls_back_to_legacy_application_support() {
        assert_eq!(
            select_macos_config_dir_choice(false, true, true),
            MacosConfigDirChoice::LegacyApplicationSupport
        );
    }

    #[test]
    fn test_select_macos_config_dir_choice_falls_back_to_bundle_id_when_only_bundle_recognizable() {
        assert_eq!(
            select_macos_config_dir_choice(false, false, true),
            MacosConfigDirChoice::LegacyBundleIdApplicationSupport
        );
    }

    #[test]
    fn test_select_macos_config_dir_choice_returns_default_when_no_recognizable_roots() {
        assert_eq!(
            select_macos_config_dir_choice(false, false, false),
            MacosConfigDirChoice::Default
        );
    }

    #[test]
    fn test_is_recognizable_config_root_detects_config_yaml_or_prompt_dir() {
        let temp = TempDir::new().expect("temp dir");
        assert!(!is_recognizable_config_root(temp.path()));

        fs::write(temp.path().join(GLOBAL_CONFIG_FILE), "x").expect("write config.yaml");
        assert!(is_recognizable_config_root(temp.path()));

        let temp = TempDir::new().expect("temp dir");
        fs::create_dir_all(temp.path().join(PROMPT_DIR)).expect("create prompt dir");
        assert!(is_recognizable_config_root(temp.path()));
    }

    #[cfg(unix)]
    #[test]
    fn test_resolve_config_dir_cli_expands_tilde_to_home() {
        let temp_home = TempDir::new().expect("temp home");
        let mut proc = TestProcess::new();
        proc.set_var("HOME", temp_home.path());

        let resolved =
            resolve_config_dir_with(Some(Path::new("~/.config/llman")), None).expect("resolve");
        assert_eq!(resolved, temp_home.path().join(".config").join("llman"));
    }

    #[cfg(unix)]
    #[test]
    fn test_resolve_config_dir_env_expands_tilde_to_home() {
        let temp_home = TempDir::new().expect("temp home");
        let mut proc = TestProcess::new();
        proc.set_var("HOME", temp_home.path());

        let resolved = resolve_config_dir_with(None, Some("~/.config/llman")).expect("resolve");
        assert_eq!(resolved, temp_home.path().join(".config").join("llman"));
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
