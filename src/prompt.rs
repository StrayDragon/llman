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
    cwd_override: Option<PathBuf>,
    codex_home_override: Option<PathBuf>,
    claude_home_override: Option<PathBuf>,
}

impl PromptCommand {
    #[allow(dead_code)]
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: Config::new()?,
            cwd_override: None,
            codex_home_override: None,
            claude_home_override: None,
        })
    }

    #[allow(dead_code)]
    pub fn with_config_dir(config_dir: Option<&str>) -> Result<Self> {
        Ok(Self {
            config: Config::with_config_dir(config_dir)?,
            cwd_override: None,
            codex_home_override: None,
            claude_home_override: None,
        })
    }

    pub fn generate_interactive(&self) -> Result<()> {
        println!("{}", t!("interactive.title"));

        let interactive = is_interactive();

        let apps = vec![CURSOR_APP, CODEX_APP, CLAUDE_CODE_APP];
        let app = Select::new(&t!("interactive.select_app"), apps).prompt()?;

        let templates = select_templates(self.get_available_templates(app)?)?;
        if templates.is_empty() {
            return Ok(());
        }

        if app == CURSOR_APP {
            if self.project_root(false, interactive)?.is_none() {
                return Ok(());
            }
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
                if self.project_root(force, interactive)?.is_none() {
                    return Ok(());
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

    pub fn remove_rule(&self, app: &str, name: &str, yes: bool, interactive: bool) -> Result<()> {
        self.validate_app(app)?;

        let rule_path = self.config.rule_file_path(app, name);

        if !rule_path.exists() {
            return Err(anyhow!(t!("errors.rule_not_found", name = name)));
        }

        if yes {
            fs::remove_file(&rule_path)?;
            println!("{}", t!("messages.rule_deleted", name = name));
            return Ok(());
        }

        if !interactive {
            return Err(anyhow!(t!(
                "errors.non_interactive_delete_requires_yes",
                name = name
            )));
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
                let cwd = self.cwd()?;
                let root = find_git_root(&cwd).unwrap_or(cwd);
                Ok(root.join(TARGET_CURSOR_RULES_DIR))
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
    fn cwd(&self) -> Result<PathBuf> {
        if let Some(cwd) = self.cwd_override.as_ref() {
            return Ok(cwd.clone());
        }
        Ok(env::current_dir()?)
    }

    fn ensure_not_home_dir(&self) -> Result<()> {
        let current_dir = self.cwd()?;
        if let Some(user_dir) = directories::UserDirs::new()
            && current_dir == user_dir.home_dir().to_path_buf()
        {
            return Err(anyhow!(t!("errors.home_directory_not_allowed")));
        }
        Ok(())
    }

    fn project_root(&self, force: bool, interactive: bool) -> Result<Option<PathBuf>> {
        self.ensure_not_home_dir()?;
        let cwd = self.cwd()?;
        if let Some(root) = find_git_root(&cwd) {
            return Ok(Some(root));
        }
        if force {
            return Ok(Some(cwd));
        }
        if interactive {
            let prompt = t!("interactive.project_root_force_prompt");
            let confirmed = Confirm::new(&prompt)
                .with_default(false)
                .prompt()
                .map_err(|e| anyhow!(t!("errors.interactive_prompt_error", error = e)))?;
            if confirmed {
                return Ok(Some(cwd));
            }
            println!("{}", t!("messages.operation_cancelled"));
            return Ok(None);
        }
        Err(anyhow!(t!("errors.project_scope_requires_repo")))
    }

    fn codex_home_dir(&self) -> Result<PathBuf> {
        if let Some(home) = self.codex_home_override.as_ref() {
            return Ok(home.clone());
        }
        if let Ok(home) = env::var("CODEX_HOME") {
            return Ok(PathBuf::from(home));
        }
        let home = dirs::home_dir().ok_or_else(|| anyhow!(t!("errors.home_dir_missing")))?;
        Ok(home.join(".codex"))
    }

    fn claude_home_dir(&self) -> Result<PathBuf> {
        if let Some(home) = self.claude_home_override.as_ref() {
            return Ok(home.clone());
        }
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
        interactive: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut targets = Vec::new();
        if scope == PromptScope::User || scope == PromptScope::All {
            let dir = self.codex_home_dir()?.join("prompts");
            targets.push(dir.join(format!("{name}.md")));
        }
        if scope == PromptScope::Project || scope == PromptScope::All {
            let Some(root) = self.project_root(force, interactive)? else {
                return Ok(Vec::new());
            };
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
        let targets = self.codex_prompt_targets(name, scope, force, interactive)?;
        if targets.is_empty() {
            return Ok(());
        }
        for target_path in targets {
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

    fn claude_memory_targets(
        &self,
        scope: PromptScope,
        force: bool,
        interactive: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut targets = Vec::new();
        if scope == PromptScope::User || scope == PromptScope::All {
            targets.push(self.claude_home_dir()?.join(CLAUDE_MEMORY_FILE));
        }
        if scope == PromptScope::Project || scope == PromptScope::All {
            let Some(root) = self.project_root(force, interactive)? else {
                return Ok(Vec::new());
            };
            targets.push(root.join(CLAUDE_MEMORY_FILE));
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
        let targets = self.claude_memory_targets(scope, force, interactive)?;
        if targets.is_empty() {
            return Ok(());
        }
        for path in targets {
            if path.exists() {
                let existing = fs::read_to_string(&path).map_err(|e| {
                    anyhow!(t!(
                        "errors.file_read_failed",
                        path = path.display(),
                        error = e
                    ))
                })?;
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
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;

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
        let temp_cwd = TempDir::new().expect("temp dir");
        let current_dir = temp_cwd.path().to_path_buf();
        let mut command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        command.cwd_override = Some(current_dir.clone());
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
        let temp = TempDir::new().expect("temp dir");
        let repo = temp.path().join("repo");
        fs::create_dir_all(repo.join(".git")).expect("create git dir");
        fs::write(repo.join("CLAUDE.md"), "user content").expect("write");

        let temp_dir = temp_config_dir("claude_non_interactive");
        let mut command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        command.cwd_override = Some(repo);

        let result = command.write_claude_memory_files(
            super::PromptScope::Project,
            false,
            false,
            "llman body",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_claude_inject_updates_managed_block_non_interactive() {
        let temp = TempDir::new().expect("temp dir");
        let repo = temp.path().join("repo");
        fs::create_dir_all(repo.join(".git")).expect("create git dir");

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
        let mut command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        command.cwd_override = Some(repo);
        command
            .write_claude_memory_files(super::PromptScope::Project, false, false, "new body")
            .expect("update");

        let updated = fs::read_to_string(&file).expect("read");
        assert!(updated.contains(super::LLMAN_PROMPTS_MARKER_START));
        assert!(updated.contains(super::LLMAN_PROMPTS_MARKER_END));
        assert!(updated.contains("new body"));
        assert!(updated.contains("user content"));
    }

    #[test]
    fn test_codex_user_targets_respect_codex_home() {
        let temp = TempDir::new().expect("temp dir");
        let codex_home = temp.path().join("codexhome");
        fs::create_dir_all(&codex_home).expect("create codex home");

        let temp_dir = temp_config_dir("codex_home");
        let mut command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        command.codex_home_override = Some(codex_home.clone());
        let targets = command
            .codex_prompt_targets("draftpr", super::PromptScope::User, false, false)
            .unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], codex_home.join("prompts").join("draftpr.md"));
    }

    #[test]
    fn test_remove_rule_requires_yes_in_non_interactive() {
        let temp_dir = temp_config_dir("rm_non_interactive");
        let command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();

        command
            .config
            .ensure_app_dir(CURSOR_APP)
            .expect("ensure app dir");
        let rule_path = command.config.rule_file_path(CURSOR_APP, "demo");
        fs::write(&rule_path, "content").expect("write rule");

        let err = command
            .remove_rule(CURSOR_APP, "demo", false, false)
            .expect_err("should require --yes");
        assert!(err.to_string().contains("--yes"));

        command
            .remove_rule(CURSOR_APP, "demo", true, false)
            .expect("remove with yes");
        assert!(!rule_path.exists());
    }

    #[test]
    fn test_claude_inject_fails_if_existing_file_is_not_utf8() {
        let temp = TempDir::new().expect("temp dir");
        let repo = temp.path().join("repo");
        fs::create_dir_all(repo.join(".git")).expect("create git dir");

        let file = repo.join("CLAUDE.md");
        let original_bytes = vec![0xFF, 0xFE, 0xFD];
        fs::write(&file, &original_bytes).expect("write invalid utf8");

        let temp_dir = temp_config_dir("claude_invalid_utf8");
        let mut command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        command.cwd_override = Some(repo);

        let err = command
            .write_claude_memory_files(super::PromptScope::Project, true, false, "body")
            .expect_err("should fail to read");
        assert!(err.to_string().contains("Failed to read"));

        let after = fs::read(&file).expect("read bytes");
        assert_eq!(after, original_bytes);
    }

    #[test]
    fn test_project_scope_requires_force_when_git_root_missing_non_interactive() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("no_repo");
        fs::create_dir_all(&root).expect("create dir");

        let temp_dir = temp_config_dir("codex_no_repo");
        let mut command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        command.cwd_override = Some(root.clone());

        let err = command
            .write_codex_prompt_files("draftpr", super::PromptScope::Project, false, false, "x")
            .expect_err("should require force");
        assert!(err.to_string().contains("--force"));
        assert!(!root.join(".codex/prompts/draftpr.md").exists());

        command
            .write_codex_prompt_files("draftpr", super::PromptScope::Project, true, false, "x")
            .expect("force writes");
        assert!(root.join(".codex/prompts/draftpr.md").exists());
    }

    #[test]
    fn test_project_scope_writes_to_repo_root_from_subdir() {
        let temp = TempDir::new().expect("temp dir");
        let repo = temp.path().join("repo");
        let nested = repo.join("a").join("b");
        fs::create_dir_all(&nested).expect("create nested dirs");
        fs::create_dir_all(repo.join(".git")).expect("create git dir");

        let temp_dir = temp_config_dir("codex_repo_root");
        let mut command = PromptCommand::with_config_dir(Some(temp_dir.to_str().unwrap())).unwrap();
        command.cwd_override = Some(nested.clone());

        command
            .write_codex_prompt_files(
                "draftpr",
                super::PromptScope::Project,
                false,
                false,
                "content",
            )
            .expect("write");

        assert!(repo.join(".codex/prompts/draftpr.md").exists());
        assert!(!nested.join(".codex/prompts/draftpr.md").exists());
    }
}
