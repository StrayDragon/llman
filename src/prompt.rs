use crate::config::{
    CLAUDE_CODE_APP, CODEX_APP, CURSOR_APP, CURSOR_EXTENSION, Config, DEFAULT_EXTENSION,
    TARGET_CURSOR_RULES_DIR,
};
use crate::path_utils::{safe_parent_for_creation, validate_path_str};
use crate::sdd::project::fs_utils::update_file_with_markers;
use crate::skills::cli::interactive::is_interactive;
use crate::skills::shared::git::find_git_root;
use anyhow::{Result, anyhow};
use inquire::{Confirm, MultiSelect, Select, Text, validator::Validation};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptScope {
    User,
    Project,
    All,
}

const LLMAN_PROMPTS_MARKER_START: &str = "<!-- LLMAN-PROMPTS:START -->";
const LLMAN_PROMPTS_MARKER_END: &str = "<!-- LLMAN-PROMPTS:END -->";
const CLAUDE_MEMORY_FILE: &str = "CLAUDE.md";

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

        let apps = vec![CURSOR_APP, CODEX_APP, CLAUDE_CODE_APP];
        let app = Select::new(&t!("interactive.select_app"), apps).prompt()?;

        let templates = select_templates(self.get_available_templates(app)?)?;
        if templates.is_empty() {
            return Ok(());
        }

        if app == CURSOR_APP {
            self.check_project_directory()?;
            let target_dir = self.prompt_target_dir(app)?;
            for template_name in &templates {
                self.generate_rules_with_target_dir(
                    app,
                    template_name,
                    template_name,
                    PromptScope::Project,
                    false,
                    Some(&target_dir),
                )?;
            }
        } else if app == CODEX_APP {
            let scope = self.prompt_scope(app)?;
            for template_name in &templates {
                self.generate_rules_with_target_dir(
                    app,
                    template_name,
                    template_name,
                    scope,
                    false,
                    None,
                )?;
            }
        } else if app == CLAUDE_CODE_APP {
            let scope = self.prompt_scope(app)?;
            self.inject_claude_templates(scope, &templates, false)?;
        }

        println!("{}", t!("messages.rule_generation_success"));
        Ok(())
    }

    pub fn generate_rules(
        &self,
        app: &str,
        template_name: &str,
        name: Option<&str>,
        scope: PromptScope,
        force: bool,
    ) -> Result<()> {
        let output_name = name.unwrap_or(template_name);
        self.generate_rules_with_target_dir(app, template_name, output_name, scope, force, None)
    }

    fn generate_rules_with_target_dir(
        &self,
        app: &str,
        template_name: &str,
        output_name: &str,
        scope: PromptScope,
        force: bool,
        target_dir: Option<&Path>,
    ) -> Result<()> {
        self.validate_app(app)?;

        let interactive = is_interactive();

        let content = self.get_template_content(app, template_name)?;

        match app {
            CURSOR_APP => {
                if !force {
                    self.check_project_directory()?;
                }
                let target_path = self.get_target_path(app, output_name, target_dir)?;
                if target_path.exists() && !force {
                    let overwrite = confirm_overwrite(&target_path, interactive)?;
                    if !overwrite {
                        println!("{}", t!("messages.operation_cancelled"));
                        return Ok(());
                    }
                }
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
            CODEX_APP => {
                self.write_codex_prompt_files(output_name, scope, force, interactive, &content)
            }
            CLAUDE_CODE_APP => {
                self.write_claude_memory_files(scope, force, interactive, &content)?;
                Ok(())
            }
            _ => Err(anyhow!(t!("errors.invalid_app", app = app))),
        }
    }

    fn inject_claude_templates(
        &self,
        scope: PromptScope,
        templates: &[String],
        force: bool,
    ) -> Result<()> {
        let mut parts = Vec::new();
        for name in templates {
            let content = self.get_template_content(CLAUDE_CODE_APP, name)?;
            parts.push(format!(
                "## llman prompts: {name}\n\n{}",
                content.trim_end()
            ));
        }
        let combined = parts.join("\n\n");
        self.write_claude_memory_files(scope, force, is_interactive(), &combined)
    }

    fn prompt_scope(&self, _app: &str) -> Result<PromptScope> {
        let options = vec!["project", "user", "all"];
        let picked = Select::new(&t!("prompt.scope.select"), options).prompt()?;
        Ok(match picked {
            "user" => PromptScope::User,
            "all" => PromptScope::All,
            _ => PromptScope::Project,
        })
    }

    pub fn list_rules(&self, app: Option<&str>) -> Result<()> {
        if let Some(app) = app {
            self.validate_app(app)?;
            self.list_app_rules(app)?;
        } else {
            let apps = vec![CURSOR_APP, CODEX_APP, CLAUDE_CODE_APP];
            for app in apps {
                println!("\n{}", t!("prompt.list.app_header", app = app));
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
            CODEX_APP => Ok(()),
            CLAUDE_CODE_APP => Ok(()),
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
                println!("  {}", t!("prompt.list.rule_item", name = rule));
            }
        }

        Ok(())
    }
}

fn confirm_overwrite(path: &Path, interactive: bool) -> Result<bool> {
    if !interactive {
        return Err(anyhow!(t!(
            "errors.non_interactive_overwrite_requires_force",
            path = path.display()
        )));
    }
    let overwrite = Confirm::new(&t!("messages.file_exists_overwrite", path = path.display()))
        .with_default(false)
        .prompt()
        .map_err(|e| anyhow!(t!("errors.interactive_prompt_error", error = e)))?;
    Ok(overwrite)
}

fn confirm_inject(path: &Path, interactive: bool) -> Result<bool> {
    if !interactive {
        return Err(anyhow!(t!(
            "errors.non_interactive_inject_requires_force",
            path = path.display()
        )));
    }
    let confirmed = Confirm::new(&t!("messages.file_exists_inject", path = path.display()))
        .with_default(false)
        .prompt()
        .map_err(|e| anyhow!(t!("errors.interactive_prompt_error", error = e)))?;
    Ok(confirmed)
}

fn has_llman_prompts_markers(content: &str) -> bool {
    content
        .lines()
        .any(|line| line.trim() == LLMAN_PROMPTS_MARKER_START)
        && content
            .lines()
            .any(|line| line.trim() == LLMAN_PROMPTS_MARKER_END)
}

impl PromptCommand {
    fn ensure_not_home_dir(&self) -> Result<()> {
        let current_dir = env::current_dir()?;
        if let Some(user_dir) = directories::UserDirs::new()
            && current_dir == user_dir.home_dir().to_path_buf()
        {
            return Err(anyhow!(t!("errors.home_directory_not_allowed")));
        }
        Ok(())
    }

    fn project_root(&self, force: bool) -> Result<PathBuf> {
        self.ensure_not_home_dir()?;
        let cwd = env::current_dir()?;
        if let Some(root) = find_git_root(&cwd) {
            return Ok(root);
        }
        if force {
            return Ok(cwd);
        }
        Err(anyhow!(t!("errors.project_scope_requires_repo")))
    }

    fn codex_home_dir(&self) -> Result<PathBuf> {
        if let Ok(home) = env::var("CODEX_HOME") {
            return Ok(PathBuf::from(home));
        }
        let home = dirs::home_dir().ok_or_else(|| anyhow!(t!("errors.home_dir_missing")))?;
        Ok(home.join(".codex"))
    }

    fn claude_home_dir(&self) -> Result<PathBuf> {
        if let Ok(home) = env::var("CLAUDE_HOME") {
            return Ok(PathBuf::from(home));
        }
        let home = dirs::home_dir().ok_or_else(|| anyhow!(t!("errors.home_dir_missing")))?;
        Ok(home.join(".claude"))
    }

    fn codex_prompt_targets(
        &self,
        name: &str,
        scope: PromptScope,
        force: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut targets = Vec::new();
        if scope == PromptScope::User || scope == PromptScope::All {
            let dir = self.codex_home_dir()?.join("prompts");
            targets.push(dir.join(format!("{name}.md")));
        }
        if scope == PromptScope::Project || scope == PromptScope::All {
            let root = self.project_root(force)?;
            let dir = root.join(".codex").join("prompts");
            targets.push(dir.join(format!("{name}.md")));
        }
        Ok(targets)
    }

    fn write_codex_prompt_files(
        &self,
        name: &str,
        scope: PromptScope,
        force: bool,
        interactive: bool,
        content: &str,
    ) -> Result<()> {
        for target_path in self.codex_prompt_targets(name, scope, force)? {
            if target_path.exists() && !force {
                let overwrite = confirm_overwrite(&target_path, interactive)?;
                if !overwrite {
                    println!("{}", t!("messages.operation_cancelled"));
                    continue;
                }
            }
            if let Some(parent) = safe_parent_for_creation(&target_path) {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target_path, content)?;
            println!(
                "{}",
                t!("messages.rule_generated", path = target_path.display())
            );
        }
        Ok(())
    }

    fn claude_memory_targets(&self, scope: PromptScope, force: bool) -> Result<Vec<PathBuf>> {
        let mut targets = Vec::new();
        if scope == PromptScope::User || scope == PromptScope::All {
            targets.push(self.claude_home_dir()?.join(CLAUDE_MEMORY_FILE));
        }
        if scope == PromptScope::Project || scope == PromptScope::All {
            targets.push(self.project_root(force)?.join(CLAUDE_MEMORY_FILE));
        }
        Ok(targets)
    }

    fn write_claude_memory_files(
        &self,
        scope: PromptScope,
        force: bool,
        interactive: bool,
        body: &str,
    ) -> Result<()> {
        for path in self.claude_memory_targets(scope, force)? {
            if path.exists() {
                let existing = fs::read_to_string(&path).unwrap_or_default();
                let needs_confirm =
                    !existing.trim().is_empty() && !has_llman_prompts_markers(&existing);
                if needs_confirm && !force {
                    let confirmed = confirm_inject(&path, interactive)?;
                    if !confirmed {
                        println!("{}", t!("messages.operation_cancelled"));
                        continue;
                    }
                }
            }
            update_file_with_markers(
                &path,
                body.trim_end(),
                LLMAN_PROMPTS_MARKER_START,
                LLMAN_PROMPTS_MARKER_END,
            )?;
            println!("{}", t!("messages.rule_generated", path = path.display()));
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
    use crate::config::{CLAUDE_CODE_APP, CODEX_APP, CURSOR_APP, TARGET_CURSOR_RULES_DIR};
    use crate::test_utils::ENV_MUTEX;
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
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

    #[test]
    fn test_validate_app_supports_codex_and_claude_code() {
        let temp_dir = temp_config_dir("apps");
        let command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        command.validate_app(CODEX_APP).unwrap();
        command.validate_app(CLAUDE_CODE_APP).unwrap();
    }

    #[test]
    fn test_claude_inject_requires_force_in_non_interactive_when_unmanaged() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let cwd_guard = CwdGuard::new();

        let temp = TempDir::new().expect("temp dir");
        let repo = temp.path().join("repo");
        fs::create_dir_all(repo.join(".git")).expect("create git dir");
        fs::write(repo.join("CLAUDE.md"), "user content").expect("write");
        env::set_current_dir(&repo).expect("chdir");

        let temp_dir = temp_config_dir("claude_non_interactive");
        let command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();

        let result = command.write_claude_memory_files(
            super::PromptScope::Project,
            false,
            false,
            "llman body",
        );
        assert!(result.is_err());

        drop(cwd_guard);
    }

    #[test]
    fn test_claude_inject_updates_managed_block_non_interactive() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let cwd_guard = CwdGuard::new();

        let temp = TempDir::new().expect("temp dir");
        let repo = temp.path().join("repo");
        fs::create_dir_all(repo.join(".git")).expect("create git dir");
        env::set_current_dir(&repo).expect("chdir");

        let file = repo.join("CLAUDE.md");
        fs::write(
            &file,
            format!(
                "{}\nold body\n{}\n\nuser content\n",
                super::LLMAN_PROMPTS_MARKER_START,
                super::LLMAN_PROMPTS_MARKER_END
            ),
        )
        .expect("write");

        let temp_dir = temp_config_dir("claude_managed_update");
        let command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        command
            .write_claude_memory_files(super::PromptScope::Project, false, false, "new body")
            .expect("update");

        let updated = fs::read_to_string(&file).expect("read");
        assert!(updated.contains(super::LLMAN_PROMPTS_MARKER_START));
        assert!(updated.contains(super::LLMAN_PROMPTS_MARKER_END));
        assert!(updated.contains("new body"));
        assert!(updated.contains("user content"));

        drop(cwd_guard);
    }

    #[test]
    fn test_codex_user_targets_respect_codex_home() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp = TempDir::new().expect("temp dir");
        let codex_home = temp.path().join("codexhome");
        fs::create_dir_all(&codex_home).expect("create codex home");

        unsafe {
            env::set_var("CODEX_HOME", &codex_home);
        }

        let temp_dir = temp_config_dir("codex_home");
        let command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        let targets = command
            .codex_prompt_targets("draftpr", super::PromptScope::User, false)
            .unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], codex_home.join("prompts").join("draftpr.md"));

        unsafe {
            env::remove_var("CODEX_HOME");
        }
    }
}
