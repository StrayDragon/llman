use crate::arg_utils::split_shell_args;
use crate::editor::{parse_editor_command, select_editor_raw};
use crate::path_utils::safe_parent_for_creation;
use crate::x::claude_code::config::{Config, ConfigGroup};
use crate::x::claude_code::env_injection::{
    EnvSyntax, env_syntax_for_current_platform, render_env_injection_lines,
};
use crate::x::claude_code::interactive;
use crate::x::claude_code::security::{SecurityChecker, SecurityWarning};
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use rust_i18n::t;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::process::Command;

#[derive(Args)]
#[command(
    author,
    version,
    about,
    long_about = "Commands for managing Claude Code configurations and API settings"
)]
pub struct ClaudeCodeArgs {
    #[command(subcommand)]
    pub command: Option<ClaudeCodeCommands>,
}

#[derive(Subcommand)]
pub enum ClaudeCodeCommands {
    /// Account management commands for handling multiple API configurations
    #[command(alias = "a")]
    Account {
        #[command(subcommand)]
        action: Option<AccountAction>,
    },
    /// Run claude with configuration selection
    #[command(about = "Run claude with configuration")]
    Run {
        /// Interactive mode: prompt for configuration and arguments
        #[arg(
            short = 'i',
            long,
            help = "Interactive mode: prompt for configuration and arguments"
        )]
        interactive: bool,

        /// Configuration group name to use (required in non-interactive mode)
        #[arg(long = "group", help = "Configuration group name to use")]
        group: Option<String>,

        /// Arguments to pass to claude command (use -- to separate)
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Arguments to pass to claude (use -- to separate from run options)"
        )]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum AccountAction {
    /// Edit claude-code configuration file
    Edit,
    /// Display all configured API configuration groups
    ///
    /// This command lists all configuration groups that have been created,
    /// showing their names and the environment variables they contain.
    /// Sensitive values like API keys are partially masked for security.
    #[command(about = "List all configuration groups")]
    List,
    /// Import a new configuration group from JSON format
    ///
    /// Interactive mode allows you to paste JSON configuration in two formats:
    ///   - Direct key-value pairs: {"KEY": "value", ...}
    ///   - Wrapped in env object: {"env": {"KEY": "value", ...}}
    ///
    /// The command includes automatic JSON fixing for common syntax errors.
    #[command(about = "Import a configuration group from JSON")]
    Import {
        /// Force overwrite existing configuration group
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        force: bool,
    },
    /// Switch to a specific configuration group and execute a command
    ///
    /// Selects the named configuration group and optionally executes a command
    /// with that configuration active. The command will use the environment
    /// variables from the selected group.
    ///
    /// Examples:
    ///   llman x claude-code account use minimax --version
    ///   llman x cc account use production -- claude code
    #[command(about = "Use/select a configuration group")]
    Use {
        #[arg(help = "Name of the configuration group to use")]
        name: String,
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Arguments to pass to claude"
        )]
        args: Vec<String>,
    },
    /// Emit shell-consumable env injection statements for a configuration group
    ///
    /// Examples:
    ///   bash/zsh:  eval "$(llman x claude-code account env my-group)"
    ///   bash/zsh:  source <(llman x claude-code account env my-group)
    ///   PowerShell: llman x claude-code account env my-group | Out-String | Invoke-Expression
    #[command(about = "Emit env injection statements for a group")]
    Env {
        #[arg(help = "Name of the configuration group")]
        name: String,
    },
}

pub fn run(args: &ClaudeCodeArgs) -> Result<()> {
    match &args.command {
        Some(ClaudeCodeCommands::Account { action }) => {
            handle_account_command(action.as_ref())?;
        }
        Some(ClaudeCodeCommands::Run {
            interactive,
            group,
            args,
        }) => {
            handle_run_command(*interactive, group.as_deref(), args.clone())?;
        }
        None => {
            handle_main_command()?;
        }
    }

    Ok(())
}

fn handle_main_command() -> Result<()> {
    let config = Config::load().context(t!("claude_code.error.load_config_failed"))?;

    if config.is_empty() {
        bail!(no_configs_message());
    }

    if let Some(selected_group) = interactive::select_config_group(&config)? {
        let group = config.get_group(&selected_group).ok_or_else(|| {
            anyhow::anyhow!(format!(
                "{}\n{}",
                t!("claude_code.account.group_not_found", name = selected_group),
                t!("claude_code.account.use_list_command")
            ))
        })?;

        // Perform security check before executing claude
        let security_checker = SecurityChecker::from_config(&config)?;
        if let Ok(warnings) = security_checker.check_claude_settings() {
            print_security_warnings(&warnings);
        }

        // Execute claude command with all environment variables set
        let mut cmd = Command::new("claude");
        inject_env_vars(&mut cmd, group);

        let status = cmd
            .status()
            .context(t!("claude_code.error.execute_failed"))?;

        if !status.success() {
            bail!(t!("claude_code.error.failed_claude_command"));
        }
    }

    Ok(())
}

fn handle_account_command(action: Option<&AccountAction>) -> Result<()> {
    if matches!(action, Some(AccountAction::Edit)) {
        handle_account_edit()?;
        return Ok(());
    }

    let mut config = Config::load().context(t!("claude_code.error.load_config_failed"))?;

    match action {
        Some(cli_action) => execute_account_action(&mut config, cli_action)?,
        None => handle_list_groups(&config),
    }

    Ok(())
}

fn execute_account_action(config: &mut Config, action: &AccountAction) -> Result<()> {
    match action {
        AccountAction::Edit => unreachable!("Edit is handled before config load"),
        AccountAction::List => handle_list_groups(config),
        AccountAction::Import { force } => handle_import_group(config, *force)?,
        AccountAction::Use { name, args } => handle_use_group(config, name, args.clone())?,
        AccountAction::Env { name } => handle_env_group(config, name)?,
    }
    Ok(())
}

fn handle_account_edit() -> Result<()> {
    let config_path = Config::config_file_path()?;
    let editor_raw = select_editor_raw();
    handle_account_edit_with(&config_path, &editor_raw)
}

fn handle_account_edit_with(config_path: &Path, editor_raw: &str) -> Result<()> {
    if let Some(parent) = safe_parent_for_creation(config_path) {
        fs::create_dir_all(parent).context(t!(
            "claude_code.config.create_dir_failed",
            path = parent.display()
        ))?;
    }

    let created = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(config_path)
    {
        Ok(mut file) => {
            let template = include_str!("../../../templates/claude-code/default.toml");
            file.write_all(template.as_bytes()).context(t!(
                "claude_code.config.write_failed",
                path = config_path.display()
            ))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(config_path)
                    .context(t!("claude_code.config.metadata_failed"))?
                    .permissions();
                perms.set_mode(0o600);
                fs::set_permissions(config_path, perms)
                    .context(t!("claude_code.config.permissions_failed"))?;
            }

            println!(
                "{}",
                t!(
                    "claude_code.account.config_created",
                    path = config_path.display()
                )
            );
            true
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => false,
        Err(e) => {
            return Err(e).context(t!(
                "claude_code.config.write_failed",
                path = config_path.display()
            ));
        }
    };

    if !created {
        println!(
            "{}",
            t!("claude_code.account.editing", path = config_path.display())
        );
    }

    let (editor_cmd, editor_args) = parse_editor_command(editor_raw).map_err(|e| {
        anyhow::anyhow!(t!(
            "claude_code.error.invalid_editor_command",
            editor = editor_raw,
            error = e
        ))
    })?;

    let status = Command::new(&editor_cmd)
        .args(editor_args)
        .arg(config_path)
        .status()
        .context(t!(
            "claude_code.error.open_editor_failed",
            editor = editor_raw
        ))?;

    if !status.success() {
        bail!(t!("claude_code.error.editor_exit_status", status = status));
    }

    println!("{}", t!("claude_code.account.edited"));
    Ok(())
}

fn handle_import_group(config: &mut Config, force: bool) -> Result<()> {
    if let Some((name, group)) = interactive::prompt_import_config()? {
        // Check if group already exists
        if config.groups.contains_key(&name) {
            if !force {
                bail!(
                    "{}\n{}",
                    t!("claude_code.account.group_exists", name = name),
                    t!("claude_code.account.use_different_name_or_force")
                );
            } else {
                println!(
                    "{}",
                    t!("claude_code.account.overwriting_group", name = name)
                );
            }
        }

        config.add_group(name.clone(), group);
        config
            .save()
            .with_context(|| t!("claude_code.error.save_after_import_failed"))?;
        println!("{}", t!("claude_code.account.import_success", name = name));
    } else {
        println!("{}", t!("claude_code.interactive.import_cancelled"));
    }

    Ok(())
}

fn handle_list_groups(config: &Config) {
    interactive::display_config_list(config);
}

fn handle_use_group(config: &Config, name: &str, args: Vec<String>) -> Result<()> {
    if let Some(group) = config.get_group(name) {
        // Perform security check before executing claude
        let security_checker = SecurityChecker::from_config(config)?;
        if let Ok(warnings) = security_checker.check_claude_settings() {
            print_security_warnings(&warnings);
        }

        // Execute claude command with all environment variables set
        let mut cmd = Command::new("claude");
        inject_env_vars(&mut cmd, group);

        // Add any additional arguments
        for arg in args {
            cmd.arg(arg);
        }

        let status = cmd
            .status()
            .context(t!("claude_code.error.execute_failed"))?;

        if !status.success() {
            bail!(t!("claude_code.error.failed_claude_command"));
        }
    } else {
        bail!(
            "{}\n{}",
            t!("claude_code.account.group_not_found", name = name),
            t!("claude_code.account.use_list_command")
        );
    }
    Ok(())
}

fn handle_env_group(config: &Config, name: &str) -> Result<()> {
    if config.is_empty() {
        bail!(no_configs_message());
    }

    let group = config.get_group(name).ok_or_else(|| {
        anyhow::anyhow!(format!(
            "{}\n{}",
            t!("claude_code.account.group_not_found", name = name),
            t!("claude_code.account.use_list_command")
        ))
    })?;

    let syntax = env_syntax_for_current_platform();
    let lines = render_env_injection_lines(group, syntax)?;

    match syntax {
        EnvSyntax::PosixExport => {
            println!("# Bash/Zsh: source <(llman x claude-code account env {name}) && ...")
        }
        EnvSyntax::PowerShell => println!(
            "# PowerShell: llman x claude-code account env {name} | Out-String | Invoke-Expression"
        ),
    }

    for line in lines {
        println!("{line}");
    }

    Ok(())
}

fn handle_run_command(
    interactive: bool,
    group_name: Option<&str>,
    args: Vec<String>,
) -> Result<()> {
    let config = Config::load().context(t!("claude_code.error.load_config_failed"))?;

    if config.is_empty() {
        bail!(no_configs_message());
    }

    // 验证参数组合
    if !interactive && group_name.is_none() {
        bail!(
            "{}\n{}",
            t!("claude_code.run.error.group_required_non_interactive"),
            t!("claude_code.run.error.use_i_or_group")
        );
    }

    let (selected_group, claude_args) = if interactive {
        // 交互模式：询问配置和参数
        handle_interactive_mode(&config)?
    } else {
        // 非交互模式：使用指定的配置
        let group = group_name.unwrap().to_string();
        (group, args)
    };

    // 执行 claude 命令
    if let Some(env_vars) = config.get_group(&selected_group) {
        println!(
            "{}",
            t!("claude_code.run.using_config", name = selected_group)
        );

        // Perform security check before executing claude
        let security_checker = SecurityChecker::from_config(&config)?;
        if let Ok(warnings) = security_checker.check_claude_settings() {
            print_security_warnings(&warnings);
        }

        let mut cmd = Command::new("claude");
        inject_env_vars(&mut cmd, env_vars);

        // 添加传递的参数
        for arg in claude_args {
            cmd.arg(arg);
        }

        let status = cmd
            .status()
            .context(t!("claude_code.error.execute_failed"))?;

        if !status.success() {
            bail!(t!("claude_code.error.failed_claude_command"));
        }
    } else {
        bail!(
            "{}\n{}",
            t!("claude_code.account.group_not_found", name = selected_group),
            t!("claude_code.account.use_list_command")
        );
    }

    Ok(())
}

fn no_configs_message() -> String {
    let config_path = Config::config_file_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| t!("claude_code.main.unknown_path").to_string());

    format!(
        "{}\n\n{}\n  {}\n  {}\n\n{}:\n  {}",
        t!("claude_code.main.no_configs_found"),
        t!("claude_code.main.suggestion_import"),
        t!("claude_code.main.command_import"),
        t!("claude_code.main.command_edit"),
        t!("claude_code.main.alternative_config"),
        config_path
    )
}

/// 处理交互模式：选择配置和输入参数
fn handle_interactive_mode(config: &Config) -> Result<(String, Vec<String>)> {
    // 选择配置组
    let selected_group = interactive::select_config_group(config)?
        .ok_or_else(|| anyhow::anyhow!(t!("claude_code.error.no_configuration_selected")))?;

    // 询问是否需要传递参数给 claude
    let use_args = inquire::Confirm::new(&t!("claude_code.run.interactive.prompt_args"))
        .with_default(false)
        .prompt()
        .context(t!("claude_code.error.prompt_args_failed"))?;

    let claude_args = if use_args {
        loop {
            let args_text = inquire::Text::new(&t!("claude_code.run.interactive.enter_args"))
                .with_help_message(&t!("claude_code.run.interactive.args_help"))
                .prompt()
                .context(t!("claude_code.error.args_input_failed"))?;

            match split_shell_args(&args_text) {
                Ok(args) => break args,
                Err(e) => {
                    eprintln!(
                        "{}",
                        t!("claude_code.run.interactive.args_parse_failed", error = e)
                    );
                    continue;
                }
            }
        }
    } else {
        Vec::new()
    };

    Ok((selected_group, claude_args))
}

/// Print security warnings to stderr
fn print_security_warnings(warnings: &[SecurityWarning]) {
    if warnings.is_empty() {
        return;
    }

    eprintln!(
        "\n🔒 {}",
        t!(
            "claude_code.security.warning_header",
            count = warnings.len()
        )
    );
    eprintln!(
        "{}",
        t!("claude_code.security.warning_separator_char").repeat(60)
    );

    for warning in warnings {
        eprintln!(
            "\n{} [{}] {}",
            warning.severity.display_symbol(),
            warning.severity.display_name_localized(),
            t!("claude_code.security.warning_item_title")
        );
        eprintln!(
            "  📍 {} {}",
            t!("claude_code.security.label_location"),
            warning.config_path
        );
        eprintln!(
            "  ⚙️  {} {}",
            t!("claude_code.security.label_setting"),
            warning.config_item
        );
        eprintln!(
            "  🎯 {} {}",
            t!("claude_code.security.label_pattern"),
            warning.matched_pattern
        );
        eprintln!(
            "  📝 {} {}",
            t!("claude_code.security.label_description"),
            warning.description
        );
        eprintln!(
            "  💡 {} {}",
            t!("claude_code.security.label_recommendation"),
            warning.recommendation
        );
    }

    eprintln!("\n⚠️ {}", t!("claude_code.security.footer_line1"));
    eprintln!("  {}", t!("claude_code.security.footer_line2"));
    eprintln!();
}

/// Inject environment variables from a config group into a Command
fn inject_env_vars(cmd: &mut Command, group: &ConfigGroup) {
    for (key, value) in group {
        cmd.env(key, value);
    }
}
