use crate::config::{CURSOR_APP, Config, TARGET_CURSOR_RULES_DIR};
use crate::error::{LlmanError, Result};
use inquire::{Confirm, MultiSelect, Select};
use std::env;
use std::fs;

pub struct PromptCommand {
    config: Config,
}

impl PromptCommand {
    #[allow(dead_code)]
    pub fn new() -> Result<Self> {
        Self::with_config_dir(None)
    }

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

        let templates = self.get_available_templates(app)?;
        if templates.is_empty() {
            println!("{}", t!("interactive.no_templates"));
        }

        let templates = if !templates.is_empty() {
            Some(MultiSelect::new(&t!("interactive.select_template"), templates).prompt()?)
        } else {
            None
        };

        for template_name in templates.as_deref().unwrap() {
            self.generate_rules(app, template_name, false)?;
        }

        println!("{}", t!("messages.rule_generation_success"));
        Ok(())
    }

    pub fn generate_rules(&self, app: &str, template_name: &str, force: bool) -> Result<()> {
        self.validate_app(app)?;

        if !force {
            self.check_project_directory()?;
        }

        let rule_name = template_name;
        let target_path = self.get_target_path(app, rule_name)?;

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

        if let Some(parent) = target_path.parent() {
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
                println!("\n📁 {}:", app);
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
            return Err(LlmanError::Config {
                message: t!("messages.content_or_file_required").to_string(),
            });
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
            return Err(LlmanError::RuleNotFound {
                name: name.to_string(),
            });
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
            _ => Err(LlmanError::InvalidApp {
                app: app.to_string(),
            }),
        }
    }

    fn check_project_directory(&self) -> Result<()> {
        let current_dir = env::current_dir()?;

        if let Some(user_dir) = directories::UserDirs::new() {
            if current_dir == user_dir.home_dir().to_path_buf() {
                return Err(LlmanError::HomeDirectoryNotAllowed);
            }
        }

        let git_dir = current_dir.join(".git");
        if !git_dir.exists() {
            return Err(LlmanError::NotProjectDirectory);
        }

        Ok(())
    }

    fn get_target_path(&self, app: &str, name: &str) -> Result<std::path::PathBuf> {
        match app {
            CURSOR_APP => {
                let current_dir = env::current_dir()?;
                Ok(current_dir.join(TARGET_CURSOR_RULES_DIR).join(format!("{}.mdc", name)))
            }
            _ => Err(LlmanError::InvalidApp {
                app: app.to_string(),
            }),
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
            // 如果模板文件不存在，返回默认内容
            Ok(self.get_default_content(app, template))
        }
    }

    fn get_default_content(&self, app: &str, template: &str) -> String {
        match app {
            CURSOR_APP => format!(
                "# {} 规则文件\n\n这是一个由 llman 生成的 {} 规则文件。\n请在此处添加您的规则内容。\n",
                template, app
            ),
            _ => format!("# {} 规则文件\n\n", template),
        }
    }

    fn list_app_rules(&self, app: &str) -> Result<()> {
        let rules = self.config.list_rules(app)?;

        if rules.is_empty() {
            println!("  (无规则文件)");
        } else {
            for rule in rules {
                println!("  📄 {}", rule);
            }
        }

        Ok(())
    }
}
