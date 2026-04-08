use crate::agents::command::AgentsArgs;
use crate::config::{ENV_CONFIG_DIR, override_runtime_config_dir, resolve_config_dir_with};
use crate::config_schema::ensure_global_sample_config;
use crate::sdd::command::SddArgs;
use crate::self_command::SelfArgs;
use crate::skills::cli::command::SkillsArgs;
use crate::skills::cli::interactive::is_interactive;
use crate::tool::command::{ToolArgs, ToolCommands};
use crate::x::claude_code::command::ClaudeCodeArgs;
use crate::x::codex::command::CodexArgs;
use crate::x::cursor::command::CursorArgs;
use crate::x::sdd_eval::command::SddEvalArgs;
use anyhow::{Result, anyhow};
use clap::{CommandFactory, Parser, Subcommand};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Configuration directory for llman (default: ~/.config/llman)
    /// Required when running within llman project for development
    #[arg(short = 'C', long = "config-dir", global = true)]
    pub config_dir: Option<PathBuf>,

    /// Print the resolved configuration directory path and exit
    #[arg(long)]
    pub print_config_dir_path: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Prompt orchestrator (interactive-only)
    #[command(name = "prompts", aliases = ["prompt", "rule"])]
    Prompts(PromptsArgs),
    /// Manage skills
    #[command(alias = "skill")]
    Skills(SkillsArgs),
    /// Manage agent presets
    Agents(AgentsArgs),
    /// Spec-driven development workflow
    Sdd(SddArgs),
    /// Experimental commands
    X(XArgs),
    /// Developer tools
    Tool(ToolArgs),
    /// Self-management commands
    #[command(name = "self")]
    SelfCommand(SelfArgs),
}

#[derive(Parser)]
pub struct PromptsArgs {
    /// Print guidance and exit (use app-specific `llman x <app> prompts` commands for non-interactive usage)
    #[arg(long = "no-interactive")]
    pub no_interactive: bool,
}

#[derive(Parser)]
pub struct XArgs {
    #[command(subcommand)]
    pub command: XCommands,
}

#[derive(Subcommand)]
pub enum XCommands {
    /// Commands for interacting with Cursor
    Cursor(CursorArgs),
    /// Commands for managing Claude Code configurations
    #[command(alias = "cc")]
    ClaudeCode(ClaudeCodeArgs),
    /// Commands for managing Codex configurations
    Codex(CodexArgs),
    /// Experimental SDD evaluation pipeline (playbook-driven)
    #[command(name = "sdd-eval")]
    SddEval(SddEvalArgs),
}

pub fn run() -> Result<()> {
    let Cli {
        config_dir,
        print_config_dir_path,
        command,
    } = Cli::parse();

    if print_config_dir_path {
        let env_override = env::var(ENV_CONFIG_DIR).ok();
        let config_dir = resolve_config_dir_with(config_dir.as_deref(), env_override.as_deref())?;
        println!("{}", config_dir.display());
        return Ok(());
    }

    let Some(command) = command else {
        let mut command = Cli::command();
        command.print_help()?;
        println!();
        return Ok(());
    };

    // Determine and set config directory
    let config_dir = determine_config_dir(config_dir.as_ref())?;

    let _config_dir_guard = override_runtime_config_dir(config_dir.clone());
    ensure_global_sample_config(&config_dir)?;

    match command {
        Commands::Prompts(args) => handle_prompts_command(&args),
        Commands::Skills(args) => crate::skills::cli::command::run(&args),
        Commands::Agents(args) => crate::agents::command::run(&args),
        Commands::Sdd(args) => crate::sdd::command::run(&args),
        Commands::X(args) => handle_x_command(&args),
        Commands::Tool(args) => handle_tool_command(&args),
        Commands::SelfCommand(args) => crate::self_command::run(&args),
    }
}

/// Determine the configuration directory to use
fn determine_config_dir(cli_config_dir: Option<&PathBuf>) -> Result<PathBuf> {
    let has_cli_override = cli_config_dir.is_some();
    let env_override = env::var(ENV_CONFIG_DIR).ok();
    let has_env_override = env_override.is_some();

    // Check if we're in llman development project
    if !has_cli_override && !has_env_override && is_llman_dev_project() {
        let message = t!(
            "errors.dev_project_config_required",
            env_var = ENV_CONFIG_DIR
        );
        return Err(anyhow!(message));
    }

    resolve_config_dir_with(
        cli_config_dir.map(|path| path.as_path()),
        env_override.as_deref(),
    )
}

/// Check if current directory is an llman development project
fn is_llman_dev_project() -> bool {
    let Ok(current_dir) = env::current_dir() else {
        return false;
    };
    is_llman_dev_project_at(&current_dir)
}

fn is_llman_dev_project_at(current_dir: &Path) -> bool {
    let cargo_toml = current_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return false;
    }
    let Ok(content) = fs::read_to_string(&cargo_toml) else {
        return false;
    };
    let Ok(parsed) = toml::from_str::<toml::Value>(&content) else {
        return false;
    };
    parsed
        .get("package")
        .and_then(|pkg| pkg.get("name"))
        .and_then(|name| name.as_str())
        == Some("llman")
}

fn handle_prompts_command(args: &PromptsArgs) -> Result<()> {
    if args.no_interactive {
        print_prompts_delegation_guidance();
        return Ok(());
    }

    if !is_interactive() {
        return Err(anyhow!(
            "`llman prompts` is interactive-only; use `--no-interactive` for guidance."
        ));
    }

    let apps = vec![
        crate::config::CURSOR_APP,
        crate::config::CODEX_APP,
        crate::config::CLAUDE_CODE_APP,
    ];
    let picked = inquire::MultiSelect::new("Select target app(s):", apps).prompt()?;
    if picked.is_empty() {
        println!("{}", t!("messages.operation_cancelled"));
        return Ok(());
    }

    for app in picked {
        match app {
            crate::config::CURSOR_APP => {
                crate::x::cursor::prompts::run(&crate::x::cursor::prompts::CursorPromptsArgs {
                    command: None,
                })?;
            }
            crate::config::CODEX_APP => {
                crate::x::codex::prompts::run(&crate::x::codex::prompts::CodexPromptsArgs {
                    command: None,
                })?;
            }
            crate::config::CLAUDE_CODE_APP => {
                crate::x::claude_code::prompts::run(
                    &crate::x::claude_code::prompts::ClaudeCodePromptsArgs { command: None },
                )?;
            }
            _ => unreachable!("selected app comes from validated interactive options"),
        }
    }

    Ok(())
}

fn print_prompts_delegation_guidance() {
    println!("`llman prompts` is interactive-only.");
    println!("Use app-specific commands instead:");
    println!("  - llman x cursor prompts");
    println!("  - llman x codex prompts");
    println!("  - llman x claude-code prompts");
}

fn handle_x_command(args: &XArgs) -> Result<()> {
    match &args.command {
        XCommands::Cursor(cursor_args) => crate::x::cursor::command::run(cursor_args),
        XCommands::ClaudeCode(claude_code_args) => {
            crate::x::claude_code::command::run(claude_code_args)
        }
        XCommands::Codex(codex_args) => crate::x::codex::command::run(codex_args),
        XCommands::SddEval(sdd_eval_args) => crate::x::sdd_eval::command::run(sdd_eval_args),
    }
}

fn handle_tool_command(args: &ToolArgs) -> Result<()> {
    match &args.command {
        ToolCommands::CleanUselessComments(args) => crate::tool::clean_comments::run(args),
        ToolCommands::RmUselessDirs(args) => crate::tool::rm_empty_dirs::run(args),
        ToolCommands::RmEmptyDirs(args) => {
            eprintln!("{}", t!("tool.rm_empty_dirs.deprecated_alias_warning"));
            crate::tool::rm_empty_dirs::run(args)
        }
        ToolCommands::SyncIgnore(args) => crate::tool::sync_ignore::run(args),
    }
}

#[cfg(test)]
mod tests {
    use crate::config::resolve_config_dir_with;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_dir_cli_overrides_env() {
        let env_temp = TempDir::new().expect("temp dir");
        let cli_temp = TempDir::new().expect("temp dir");
        let env_dir = env_temp.path().to_path_buf();
        let cli_dir = cli_temp.path().to_path_buf();
        let resolved = resolve_config_dir_with(Some(&cli_dir), env_dir.to_str()).unwrap();
        assert_eq!(resolved, cli_dir);
    }

    #[test]
    fn test_is_llman_dev_project_does_not_match_comments() {
        let temp = TempDir::new().expect("temp dir");
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"
[package]
name = "not-llman" # name = "llman"
version = "0.1.0"
"#,
        )
        .expect("write Cargo.toml");
        assert!(!super::is_llman_dev_project_at(temp.path()));
    }

    #[test]
    fn test_is_llman_dev_project_matches_package_name() {
        let temp = TempDir::new().expect("temp dir");
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"
[package]
name = "llman"
version = "0.1.0"
"#,
        )
        .expect("write Cargo.toml");
        assert!(super::is_llman_dev_project_at(temp.path()));
    }
}
