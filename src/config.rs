use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const ENV_CONFIG_DIR: &str = "LLMAN_CONFIG_DIR";
pub const ENV_LANG: &str = "LLMAN_LANG";
pub const APP_NAME: &str = "llman";
pub const CURSOR_APP: &str = "cursor";
pub const CURSOR_EXTENSION: &str = "mdc";
pub const DEFAULT_EXTENSION: &str = "txt";
pub const PROMPT_DIR: &str = "prompt";
pub const TARGET_CURSOR_RULES_DIR: &str = ".cursor/rules";

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
        let config_dir = if let Some(custom_dir) = config_dir_override {
            PathBuf::from(custom_dir)
        } else if let Ok(custom_dir) = env::var(ENV_CONFIG_DIR) {
            PathBuf::from(custom_dir)
        } else {
            let project_dirs = ProjectDirs::from("", "", APP_NAME)
                .ok_or_else(|| anyhow!(t!("errors.not_find_config_dir")))?;
            project_dirs.config_dir().to_path_buf()
        };

        let prompt_dir = config_dir.join(PROMPT_DIR);

        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&prompt_dir)?;

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
        let extension = match app {
            CURSOR_APP => CURSOR_EXTENSION,
            _ => DEFAULT_EXTENSION,
        };
        self.app_dir(app).join(format!("{name}.{extension}"))
    }

    pub fn list_rules(&self, app: &str) -> Result<Vec<String>> {
        let app_dir = self.app_dir(app);

        if !app_dir.exists() {
            return Ok(Vec::new());
        }

        let mut rules = Vec::new();
        for entry in fs::read_dir(app_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && let Some(stem) = path.file_stem()
                && let Some(name) = stem.to_str() {
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
    use std::env;
    use std::sync::Mutex;

    // 使用互斥锁确保测试不会并发运行，避免环境变量冲突
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_config_with_env_var() {
        let _guard = TEST_MUTEX.lock().unwrap();

        let temp_dir = env::temp_dir().join("llman_test");
        unsafe {
            env::set_var(ENV_CONFIG_DIR, &temp_dir);
        }

        let config = Config::new().unwrap();
        assert_eq!(config.config_dir, temp_dir);
        assert_eq!(config.prompt_dir, temp_dir.join("prompt"));

        unsafe {
            env::remove_var(ENV_CONFIG_DIR);
        }
    }

    #[test]
    fn test_app_dir() {
        let _guard = TEST_MUTEX.lock().unwrap();

        let temp_dir = env::temp_dir().join("llman_test_app");
        unsafe {
            env::set_var(ENV_CONFIG_DIR, &temp_dir);
        }

        let config = Config::new().unwrap();
        let cursor_dir = config.app_dir("cursor");
        assert_eq!(cursor_dir, temp_dir.join("prompt").join("cursor"));

        unsafe {
            env::remove_var(ENV_CONFIG_DIR);
        }
    }

    #[test]
    fn test_rule_file_path() {
        let _guard = TEST_MUTEX.lock().unwrap();

        let temp_dir = env::temp_dir().join("llman_test_rule");
        unsafe {
            env::set_var(ENV_CONFIG_DIR, &temp_dir);
        }

        let config = Config::new().unwrap();
        let rule_path = config.rule_file_path("cursor", "test-rule");
        assert_eq!(
            rule_path,
            temp_dir.join("prompt").join("cursor").join("test-rule.mdc")
        );

        unsafe {
            env::remove_var(ENV_CONFIG_DIR);
        }
    }
}
