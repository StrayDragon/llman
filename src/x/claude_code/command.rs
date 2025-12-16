use crate::x::claude_code::config::Config;
use crate::x::claude_code::interactive;
use crate::x::claude_code::security::SecurityChecker;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use rust_i18n::t;
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
    let config = Config::load().context("Failed to load configuration")?;

    if config.is_empty() {
        println!("{}", t!("claude_code.main.no_configs_found"));
        println!();
        println!("{}", t!("claude_code.main.suggestion_import"));
        println!("  {}", t!("claude_code.main.command_import"));
        println!();
        println!("{}:", t!("claude_code.main.alternative_config"));
        println!(
            "  {}",
            Config::config_file_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        );
        return Ok(());
    }

    if let Some(selected_group) = interactive::select_config_group(&config)?
        && let Some(group) = config.get_group(&selected_group)
    {
        // Perform security check before executing claude
        let security_checker = SecurityChecker::from_config(&config)?;
        if let Ok(warnings) = security_checker.check_claude_settings()
            && !warnings.is_empty()
        {
            eprintln!("\n🔒 {} Security Warnings Detected:", warnings.len());
            eprintln!("{}", "═".repeat(60));

            for warning in warnings {
                eprintln!(
                    "\n{} [{}] Dangerous Permission Detected",
                    warning.severity.display_symbol(),
                    warning.severity.display_name()
                );
                eprintln!("  📍 Location: {}", warning.config_path);
                eprintln!("  ⚙️  Setting: {}", warning.config_item);
                eprintln!("  🎯 Pattern: {}", warning.matched_pattern);
                eprintln!("  📝 Description: {}", warning.description);
                eprintln!("  💡 Recommendation: {}", warning.recommendation);
            }

            eprintln!(
                "\n⚠️ These permissions are defined in your Claude Code settings but conflict with"
            );
            eprintln!("  security rules in <llman config>/claude-code.toml");
            eprintln!();
        }

        // Execute claude command with all environment variables set
        let mut cmd = Command::new("claude");

        // Inject all environment variables from the group
        for (key, value) in group {
            cmd.env(key, value);
        }

        let status = cmd.status().context("Failed to execute claude command")?;

        if !status.success() {
            eprintln!("{}", t!("claude_code.error.failed_claude_command"));
        }
    }

    Ok(())
}

fn handle_account_command(action: Option<&AccountAction>) -> Result<()> {
    let mut config = Config::load().context("Failed to load configuration")?;

    match action {
        Some(cli_action) => execute_account_action(&mut config, cli_action)?,
        None => handle_list_groups(&config),
    }

    Ok(())
}

fn execute_account_action(config: &mut Config, action: &AccountAction) -> Result<()> {
    match action {
        AccountAction::List => handle_list_groups(config),
        AccountAction::Import { force } => handle_import_group(config, *force)?,
        AccountAction::Use { name, args } => handle_use_group(config, name, args.clone())?,
    }
    Ok(())
}

fn handle_import_group(config: &mut Config, force: bool) -> Result<()> {
    if let Some((name, group)) = interactive::prompt_import_config()? {
        // Check if group already exists
        if config.groups.contains_key(&name) {
            if !force {
                println!("{}", t!("claude_code.account.group_exists", name = name));
                println!("{}", t!("claude_code.account.use_different_name_or_force"));
                return Ok(());
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
            .with_context(|| "Failed to save configuration after import")?;
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
        if let Ok(warnings) = security_checker.check_claude_settings()
            && !warnings.is_empty()
        {
            eprintln!("\n🔒 {} Security Warnings Detected:", warnings.len());
            eprintln!("{}", "═".repeat(60));

            for warning in warnings {
                eprintln!(
                    "\n{} [{}] Dangerous Permission Detected",
                    warning.severity.display_symbol(),
                    warning.severity.display_name()
                );
                eprintln!("  📍 Location: {}", warning.config_path);
                eprintln!("  ⚙️  Setting: {}", warning.config_item);
                eprintln!("  🎯 Pattern: {}", warning.matched_pattern);
                eprintln!("  📝 Description: {}", warning.description);
                eprintln!("  💡 Recommendation: {}", warning.recommendation);
            }

            eprintln!(
                "\n⚠️ These permissions are defined in your Claude Code settings but conflict with"
            );
            eprintln!("  security rules in <llman config>/claude-code.toml");
            eprintln!();
        }

        // Execute claude command with all environment variables set
        let mut cmd = Command::new("claude");

        // Inject all environment variables from the group
        for (key, value) in group {
            cmd.env(key, value);
        }

        // Add any additional arguments
        for arg in args {
            cmd.arg(arg);
        }

        let status = cmd.status().context("Failed to execute claude command")?;

        if !status.success() {
            eprintln!("{}", t!("claude_code.error.failed_claude_command"));
        }
    } else {
        println!("{}", t!("claude_code.account.group_not_found", name = name));
        println!("{}", t!("claude_code.account.use_list_command"));
    }
    Ok(())
}

fn handle_run_command(
    interactive: bool,
    group_name: Option<&str>,
    args: Vec<String>,
) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    if config.is_empty() {
        println!("{}", t!("claude_code.main.no_configs_found"));
        println!();
        println!("{}", t!("claude_code.main.suggestion_import"));
        println!("  {}", t!("claude_code.main.command_import"));
        println!();
        println!("{}:", t!("claude_code.main.alternative_config"));
        println!(
            "  {}",
            Config::config_file_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        );
        return Ok(());
    }

    // 验证参数组合
    if !interactive && group_name.is_none() {
        eprintln!(
            "{}",
            t!("claude_code.run.error.group_required_non_interactive")
        );
        eprintln!("{}", t!("claude_code.run.error.use_i_or_group"));
        return Ok(());
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
        if let Ok(warnings) = security_checker.check_claude_settings()
            && !warnings.is_empty()
        {
            eprintln!("\n🔒 {} Security Warnings Detected:", warnings.len());
            eprintln!("{}", "═".repeat(60));

            for warning in warnings {
                eprintln!(
                    "\n{} [{}] Dangerous Permission Detected",
                    warning.severity.display_symbol(),
                    warning.severity.display_name()
                );
                eprintln!("  📍 Location: {}", warning.config_path);
                eprintln!("  ⚙️  Setting: {}", warning.config_item);
                eprintln!("  🎯 Pattern: {}", warning.matched_pattern);
                eprintln!("  📝 Description: {}", warning.description);
                eprintln!("  💡 Recommendation: {}", warning.recommendation);
            }

            eprintln!(
                "\n⚠️ These permissions are defined in your Claude Code settings but conflict with"
            );
            eprintln!("  security rules in <llman config>/claude-code.toml");
            eprintln!();
        }

        let mut cmd = Command::new("claude");

        // 注入环境变量
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        // 添加传递的参数
        for arg in claude_args {
            cmd.arg(arg);
        }

        let status = cmd.status().context("Failed to execute claude command")?;

        if !status.success() {
            eprintln!("{}", t!("claude_code.error.failed_claude_command"));
        }
    } else {
        println!(
            "{}",
            t!("claude_code.account.group_not_found", name = selected_group)
        );
        println!("{}", t!("claude_code.account.use_list_command"));
    }

    Ok(())
}

/// 处理交互模式：选择配置和输入参数
fn handle_interactive_mode(config: &Config) -> Result<(String, Vec<String>)> {
    // 选择配置组
    let selected_group = interactive::select_config_group(config)?
        .ok_or_else(|| anyhow::anyhow!("No configuration selected"))?;

    // 询问是否需要传递参数给 claude
    let use_args = inquire::Confirm::new(&t!("claude_code.run.interactive.prompt_args"))
        .with_default(false)
        .prompt()
        .context("Failed to prompt for arguments")?;

    let claude_args = if use_args {
        let args_text = inquire::Text::new(&t!("claude_code.run.interactive.enter_args"))
            .with_help_message(&t!("claude_code.run.interactive.args_help"))
            .prompt()
            .context("Failed to get claude arguments")?;

        // 简单的参数分割（可以用更复杂的方式处理引号等）
        args_text
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    Ok((selected_group, claude_args))
}
