use crate::config::{
    CURSOR_APP, CURSOR_EXTENSION, Config, DEFAULT_EXTENSION, TARGET_CURSOR_RULES_DIR,
};
use crate::path_utils::{safe_parent_for_creation, validate_path_str};
use anyhow::{Result, anyhow};
use inquire::{Confirm, MultiSelect, Select, Text, validator::Validation};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct PromptCommand {
    config: Config,
}

impl PromptCommand {
    #[allow(dead_code)]
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: Config::new()?,
        })
    }

    #[allow(dead_code)]
    pub fn with_config_dir(config_dir: Option<&str>) -> Result<Self> {
        Ok(Self {
            config: Config::with_config_dir(config_dir)?,
        })
    }

    pub fn generate_interactive(&self) -> Result<()> {
        println!("{}", t!("interactive.title"));
        self.check_project_directory()?;

        let apps = vec![CURSOR_APP];
        let app = Select::new(&t!("interactive.select_app"), apps).prompt()?;

        let templates = select_templates(self.get_available_templates(app)?)?;
        if templates.is_empty() {
            return Ok(());
        }

        let target_dir = self.prompt_target_dir(app)?;
        for template_name in &templates {
            self.generate_rules_with_target_dir(app, template_name, false, Some(&target_dir))?;
        }

        println!("{}", t!("messages.rule_generation_success"));
        Ok(())
    }

    pub fn generate_rules(&self, app: &str, template_name: &str, force: bool) -> Result<()> {
        self.generate_rules_with_target_dir(app, template_name, force, None)
    }

    fn generate_rules_with_target_dir(
        &self,
        app: &str,
        template_name: &str,
        force: bool,
        target_dir: Option<&Path>,
    ) -> Result<()> {
        self.validate_app(app)?;

        if !force {
            self.check_project_directory()?;
        }

        let rule_name = template_name;
        let target_path = self.get_target_path(app, rule_name, target_dir)?;

        if target_path.exists() && !force {
            let overwrite = Confirm::new(&t!(
                "messages.file_exists_overwrite",
                path = target_path.display()
            ))
            .with_default(false)
            .prompt()?;

            if !overwrite {
                println!("{}", t!("messages.operation_cancelled"));
                return Ok(());
            }
        }

        let content = self.get_template_content(app, template_name)?;

        if let Some(parent) = safe_parent_for_creation(&target_path) {
            fs::create_dir_all(parent)?;
        }

        fs::write(&target_path, content)?;

        println!(
            "{}",
            t!("messages.rule_generated", path = target_path.display())
        );
        Ok(())
    }

    pub fn list_rules(&self, app: Option<&str>) -> Result<()> {
        if let Some(app) = app {
            self.validate_app(app)?;
            self.list_app_rules(app)?;
        } else {
            let apps = vec![CURSOR_APP];
            for app in apps {
                println!("\n📁 {app}:");
                self.list_app_rules(app)?;
            }
        }
        Ok(())
    }

    pub fn upsert_rule(
        &self,
        app: &str,
        name: &str,
        content: Option<&str>,
        file: Option<&str>,
    ) -> Result<()> {
        self.validate_app(app)?;
        self.config.ensure_app_dir(app)?;

        let rule_content = if let Some(content) = content {
            content.to_string()
        } else if let Some(file_path) = file {
            fs::read_to_string(file_path)?
        } else {
            return Err(anyhow!(t!("messages.content_or_file_required")));
        };

        let rule_path = self.config.rule_file_path(app, name);
        fs::write(&rule_path, rule_content)?;

        println!("{}", t!("messages.rule_saved", path = rule_path.display()));
        Ok(())
    }

    pub fn remove_rule(&self, app: &str, name: &str) -> Result<()> {
        self.validate_app(app)?;

        let rule_path = self.config.rule_file_path(app, name);

        if !rule_path.exists() {
            return Err(anyhow!(t!("errors.rule_not_found", name = name)));
        }

        let confirm = Confirm::new(&t!("messages.confirm_delete", name = name))
            .with_default(false)
            .prompt()?;

        if confirm {
            fs::remove_file(&rule_path)?;
            println!("{}", t!("messages.rule_deleted", name = name));
        } else {
            println!("{}", t!("messages.operation_cancelled"));
        }

        Ok(())
    }

    fn validate_app(&self, app: &str) -> Result<()> {
        match app {
            CURSOR_APP => Ok(()),
            _ => Err(anyhow!(t!("errors.invalid_app", app = app))),
        }
    }

    fn check_project_directory(&self) -> Result<()> {
        let current_dir = env::current_dir()?;

        if let Some(user_dir) = directories::UserDirs::new()
            && current_dir == user_dir.home_dir().to_path_buf()
        {
            return Err(anyhow!(t!("errors.home_dir_not_allowed")));
        }

        let git_dir = current_dir.join(".git");
        if !git_dir.exists() {
            return Err(anyhow!(t!("errors.not_project_directory")));
        }

        Ok(())
    }

    fn prompt_target_dir(&self, app: &str) -> Result<PathBuf> {
        let default_dir = self.resolve_target_dir(app, None)?;
        let default_display = default_dir.to_string_lossy().to_string();
        let target_dir = Text::new(&t!("interactive.input_target_dir"))
            .with_default(&default_display)
            .with_help_message(&t!("interactive.target_dir_help"))
            .with_validator(|input: &str| match validate_path_str(input) {
                Ok(()) => Ok(Validation::Valid),
                Err(message) => Ok(Validation::Invalid(message.into())),
            })
            .prompt()?;
        Ok(PathBuf::from(target_dir))
    }

    fn get_target_path(&self, app: &str, name: &str, target_dir: Option<&Path>) -> Result<PathBuf> {
        let target_dir = self.resolve_target_dir(app, target_dir)?;
        let extension = match app {
            CURSOR_APP => CURSOR_EXTENSION,
            _ => DEFAULT_EXTENSION,
        };
        Ok(target_dir.join(format!("{name}.{extension}")))
    }

    fn resolve_target_dir(&self, app: &str, target_dir: Option<&Path>) -> Result<PathBuf> {
        if let Some(target_dir) = target_dir {
            return Ok(target_dir.to_path_buf());
        }

        match app {
            CURSOR_APP => {
                let current_dir = env::current_dir()?;
                Ok(current_dir.join(TARGET_CURSOR_RULES_DIR))
            }
            _ => Err(anyhow!(t!("errors.invalid_app", app = app))),
        }
    }

    fn get_available_templates(&self, app: &str) -> Result<Vec<String>> {
        self.config.list_rules(app)
    }

    fn get_template_content(&self, app: &str, template: &str) -> Result<String> {
        let template_path = self.config.rule_file_path(app, template);

        if template_path.exists() {
            Ok(fs::read_to_string(template_path)?)
        } else {
            Err(anyhow!(t!("errors.rule_not_found", name = template)))
        }
    }

    fn list_app_rules(&self, app: &str) -> Result<()> {
        let rules = self.config.list_rules(app)?;

        if rules.is_empty() {
            println!("  {}", t!("errors.no_rules_found"));
        } else {
            for rule in rules {
                println!("  📄 {rule}");
            }
        }

        Ok(())
    }
}

fn select_templates(templates: Vec<String>) -> Result<Vec<String>> {
    if templates.is_empty() {
        println!("{}", t!("interactive.no_templates"));
        println!("{}", t!("interactive.no_templates_hint"));
        return Ok(Vec::new());
    }

    Ok(MultiSelect::new(&t!("interactive.select_template"), templates).prompt()?)
}

#[cfg(test)]
mod tests {
    use super::{PromptCommand, select_templates};
    use crate::config::{CURSOR_APP, TARGET_CURSOR_RULES_DIR};
    use std::env;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("llman_prompt_{label}_{nanos}"))
    }

    #[test]
    fn test_select_templates_empty_returns_empty() {
        let result = select_templates(Vec::new()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_target_path_with_custom_dir() {
        let temp_dir = temp_config_dir("custom");
        let command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        let target = command
            .get_target_path(CURSOR_APP, "feedback-mode", Some(Path::new("custom/dir")))
            .unwrap();
        assert_eq!(target, Path::new("custom/dir").join("feedback-mode.mdc"));
    }

    #[test]
    fn test_get_target_path_default_dir() {
        let temp_dir = temp_config_dir("default");
        let command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        let current_dir = env::current_dir().unwrap();
        let target = command
            .get_target_path(CURSOR_APP, "feedback-mode", None)
            .unwrap();
        assert_eq!(
            target,
            current_dir
                .join(TARGET_CURSOR_RULES_DIR)
                .join("feedback-mode.mdc")
        );
    }
}
