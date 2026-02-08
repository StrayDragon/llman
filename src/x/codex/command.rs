use crate::arg_utils::split_shell_args;
use crate::x::codex::config::{Config, upsert_to_codex_config};
use crate::x::codex::interactive;
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use rust_i18n::t;
use std::fs;
use std::process::Command;

#[derive(Args)]
#[command(
    author,
    version,
    about,
    long_about = "Commands for managing OpenAI Codex configurations"
)]
pub struct CodexArgs {
    #[command(subcommand)]
    pub command: Option<CodexCommands>,
}

#[derive(Subcommand)]
pub enum CodexCommands {
    /// Manage Codex configuration
    #[command(alias = "a")]
    Account {
        #[command(subcommand)]
        action: Option<AccountAction>,
    },
    /// Run codex with configuration selection
    Run {
        #[arg(
            short = 'i',
            long,
            help = "Interactive mode: prompt for configuration and arguments"
        )]
        interactive: bool,

        #[arg(long = "group", help = "Configuration group name to use")]
        group: Option<String>,

        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Arguments to pass to codex (use -- to separate from run options)"
        )]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum AccountAction {
    /// Edit codex configuration file
    Edit,
    /// Import a new provider configuration interactively
    Import,
}

fn select_editor_raw() -> String {
    std::env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "vi".to_string())
}

fn parse_editor_command(raw: &str) -> Result<(String, Vec<String>)> {
    let parts = split_shell_args(raw).map_err(|e| {
        anyhow::anyhow!(t!(
            "codex.error.invalid_editor_command",
            editor = raw,
            error = e
        ))
    })?;
    match parts.split_first() {
        Some((cmd, args)) if !cmd.trim().is_empty() => Ok((cmd.clone(), args.to_vec())),
        _ => Ok(("vi".to_string(), Vec::new())),
    }
}

pub fn run(args: &CodexArgs) -> Result<()> {
    match &args.command {
        None => handle_main_command()?,
        Some(CodexCommands::Account { action }) => handle_account_command(action.as_ref())?,
        Some(CodexCommands::Run {
            interactive,
            group,
            args,
        }) => handle_run_command(*interactive, group.as_deref(), args.clone())?,
    }
    Ok(())
}

/// `llman x codex` — interactive select → upsert provider → inject env → exec codex
fn handle_main_command() -> Result<()> {
    let config = Config::load().context(t!("codex.error.load_config_failed"))?;

    if config.is_empty() {
        bail!(no_configs_message());
    }

    if let Some(selected) = interactive::select_provider(&config)? {
        activate_and_exec(&config, &selected, &[])?;
    }

    Ok(())
}

fn handle_account_command(action: Option<&AccountAction>) -> Result<()> {
    match action {
        Some(AccountAction::Edit) | None => handle_account_edit()?,
        Some(AccountAction::Import) => handle_account_import()?,
    }
    Ok(())
}

fn handle_account_edit() -> Result<()> {
    let config_path = Config::config_file_path()?;

    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context(t!(
                "codex.error.create_config_dir_failed",
                path = parent.display()
            ))?;
        }
        let template = include_str!("../../../templates/codex/default.toml");
        fs::write(&config_path, template).context(t!(
            "codex.error.write_config_failed",
            path = config_path.display()
        ))?;
        println!(
            "{}",
            t!("codex.account.config_created", path = config_path.display())
        );
    }

    let editor_raw = select_editor_raw();
    let (editor_cmd, editor_args) = parse_editor_command(&editor_raw)?;

    let status = Command::new(&editor_cmd)
        .args(editor_args)
        .arg(&config_path)
        .status()
        .context(t!("codex.error.open_editor_failed", editor = editor_raw))?;

    if !status.success() {
        bail!(t!("codex.error.editor_exit_status", status = status));
    }

    println!("{}", t!("codex.account.edited"));
    Ok(())
}

fn handle_account_import() -> Result<()> {
    if let Some((key, provider)) = interactive::prompt_import()? {
        let mut config = Config::load().context(t!("codex.error.load_config_failed"))?;

        if config.model_providers.contains_key(&key) {
            bail!(t!("codex.error.group_exists", name = key));
        }

        config.add_provider(key.clone(), provider);
        config.save()?;
        println!("{}", t!("codex.account.imported", name = key));
    }
    Ok(())
}

fn handle_run_command(
    interactive_mode: bool,
    group_name: Option<&str>,
    args: Vec<String>,
) -> Result<()> {
    let config = Config::load().context(t!("codex.error.load_config_failed"))?;

    if config.is_empty() {
        bail!(no_configs_message());
    }

    if !interactive_mode && group_name.is_none() {
        bail!(
            "{}\n{}",
            t!("codex.run.error.group_required_non_interactive"),
            t!("codex.run.error.use_i_or_group")
        );
    }

    let (selected, codex_args) = if interactive_mode {
        handle_interactive_mode(&config)?
    } else {
        (group_name.unwrap().to_string(), args)
    };

    activate_and_exec(&config, &selected, &codex_args)?;

    Ok(())
}

/// Core: upsert provider to codex config, inject env vars, exec codex.
fn activate_and_exec(config: &Config, provider_key: &str, args: &[String]) -> Result<()> {
    let provider = config
        .get_provider(provider_key)
        .ok_or_else(|| anyhow::anyhow!(t!("codex.error.group_not_found", name = provider_key)))?;

    // Upsert provider to ~/.codex/config.toml
    let wrote = upsert_to_codex_config(provider_key, provider)?;
    if wrote {
        println!("{}", t!("codex.run.provider_synced", name = provider_key));
    }

    println!("{}", t!("codex.run.using_config", name = provider_key));

    // Execute codex with injected env vars
    let mut cmd = Command::new("codex");
    for (key, value) in &provider.env {
        cmd.env(key, value);
    }
    for arg in args {
        cmd.arg(arg);
    }

    let status = cmd.status().context(t!("codex.error.execute_failed"))?;

    if !status.success() {
        bail!(t!("codex.error.failed_codex_command"));
    }

    Ok(())
}

fn no_configs_message() -> String {
    let config_path = Config::config_file_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    format!(
        "{}\n\n{}\n  {}\n  {}\n\n{}:\n  {}",
        t!("codex.main.no_configs_found"),
        t!("codex.main.suggestion"),
        t!("codex.main.command_import"),
        t!("codex.main.command_edit"),
        t!("codex.main.config_location"),
        config_path
    )
}

fn handle_interactive_mode(config: &Config) -> Result<(String, Vec<String>)> {
    let selected = interactive::select_provider(config)?
        .ok_or_else(|| anyhow::anyhow!(t!("codex.error.no_configuration_selected")))?;

    let use_args = inquire::Confirm::new(&t!("codex.run.interactive.prompt_args"))
        .with_default(false)
        .prompt()
        .context(t!("codex.error.prompt_args_failed"))?;

    let codex_args = if use_args {
        loop {
            let args_text = inquire::Text::new(&t!("codex.run.interactive.enter_args"))
                .with_help_message(&t!("codex.run.interactive.args_help"))
                .prompt()
                .context(t!("codex.error.args_input_failed"))?;

            match split_shell_args(&args_text) {
                Ok(parsed) => break parsed,
                Err(e) => {
                    eprintln!(
                        "{}",
                        t!("codex.run.interactive.args_parse_failed", error = e)
                    );
                }
            }
        }
    } else {
        Vec::new()
    };

    Ok((selected, codex_args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ENV_CONFIG_DIR;
    use crate::test_utils::ENV_MUTEX;
    use crate::x::codex::config::{ProviderConfig, provider_to_codex_table};
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn editor_parsing_supports_args_and_quotes() {
        let (cmd, args) = parse_editor_command("code --wait").expect("parse");
        assert_eq!(cmd, "code");
        assert_eq!(args, vec!["--wait"]);

        let (cmd, args) = parse_editor_command("\"/path with spaces/code\" --wait").expect("parse");
        assert_eq!(cmd, "/path with spaces/code");
        assert_eq!(args, vec!["--wait"]);

        let (cmd, args) = parse_editor_command("   ").expect("parse");
        assert_eq!(cmd, "vi");
        assert!(args.is_empty());
    }

    #[test]
    fn editor_env_prefers_visual_over_editor_and_falls_back_to_vi() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        unsafe {
            env::set_var("VISUAL", "code --wait");
            env::set_var("EDITOR", "vim");
        }
        assert_eq!(select_editor_raw(), "code --wait");

        unsafe {
            env::remove_var("VISUAL");
            env::set_var("EDITOR", "  ");
        }
        assert_eq!(select_editor_raw(), "vi");

        unsafe {
            env::remove_var("EDITOR");
        }
    }

    #[cfg(unix)]
    #[test]
    fn editor_non_zero_exit_status_is_propagated() {
        use std::os::unix::fs::PermissionsExt;

        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp = TempDir::new().expect("temp dir");
        unsafe {
            env::set_var(ENV_CONFIG_DIR, temp.path());
        }

        let config_path = temp.path().join("codex.toml");
        fs::write(&config_path, "[model_providers]\n").expect("write config");

        let editor_path = temp.path().join("fail-editor.sh");
        fs::write(&editor_path, "#!/bin/sh\nexit 42\n").expect("write editor");
        let mut perms = fs::metadata(&editor_path).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&editor_path, perms).expect("chmod");

        unsafe {
            env::remove_var("VISUAL");
            env::set_var("EDITOR", editor_path.to_string_lossy().to_string());
        }

        let err = handle_account_edit().expect_err("should error");
        assert!(err.to_string().contains("Editor exited with status"));

        unsafe {
            env::remove_var(ENV_CONFIG_DIR);
            env::remove_var("EDITOR");
        }
    }

    #[test]
    fn config_load_and_provider_access() {
        let temp = TempDir::new().expect("temp dir");
        let config_path = temp.path().join("codex.toml");

        let content = r#"
[model_providers.openai]
name = "openai"
base_url = "https://api.openai.com/v1"
wire_api = "responses"
env_key = "OPENAI_API_KEY"

[model_providers.openai.env]
OPENAI_API_KEY = "sk-test-key"

[model_providers.minimax]
name = "minimax"
base_url = "https://api.minimax.com/v1"
wire_api = "responses"
env_key = "MINIMAX_KEY"

[model_providers.minimax.env]
MINIMAX_KEY = "sk-minimax"
"#;
        fs::write(&config_path, content).expect("write config");

        let config = Config::load_from_path(&config_path).expect("load config");
        assert_eq!(config.provider_names(), vec!["minimax", "openai"]);
        assert!(!config.is_empty());

        let openai = config.get_provider("openai").expect("openai");
        assert_eq!(openai.base_url, "https://api.openai.com/v1");
        assert_eq!(openai.env.get("OPENAI_API_KEY").unwrap(), "sk-test-key");

        let minimax = config.get_provider("minimax").expect("minimax");
        assert_eq!(minimax.env_key, "MINIMAX_KEY");
        assert_eq!(minimax.env.get("MINIMAX_KEY").unwrap(), "sk-minimax");
    }

    #[test]
    fn upsert_creates_and_updates_codex_config() {
        let temp = TempDir::new().expect("temp dir");
        let codex_dir = temp.path().join(".codex");
        fs::create_dir_all(&codex_dir).expect("create .codex");
        let codex_config = codex_dir.join("config.toml");

        // Write initial codex config
        fs::write(
            &codex_config,
            r#"model = "o3-pro"
model_provider = "openai"

[model_providers.openai]
name = "openai"
base_url = "https://api.openai.com/v1"
wire_api = "responses"
env_key = "OPENAI_API_KEY"
"#,
        )
        .expect("write codex config");

        let provider = ProviderConfig {
            name: "minimax".into(),
            base_url: "https://api.minimax.com/v1".into(),
            wire_api: "responses".into(),
            env_key: "MINIMAX_KEY".into(),
            env: [("MINIMAX_KEY".into(), "sk-test".into())]
                .into_iter()
                .collect(),
        };

        // We can't easily test upsert_to_codex_config because it uses dirs::home_dir,
        // but we can test the building blocks
        let table = provider_to_codex_table(&provider);
        assert!(table.is_table());
        let t = table.as_table().unwrap();
        assert_eq!(t.get("name").unwrap().as_str().unwrap(), "minimax");
        assert_eq!(
            t.get("base_url").unwrap().as_str().unwrap(),
            "https://api.minimax.com/v1"
        );
        // env should NOT be in the codex table
        assert!(t.get("env").is_none());
    }
}
