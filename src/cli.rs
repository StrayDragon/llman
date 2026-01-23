use crate::config::{ENV_CONFIG_DIR, resolve_config_dir};
use crate::prompt::PromptCommand;
use crate::sdd::command::SddArgs;
use crate::skills::command::SkillsArgs;
use crate::tool::command::{ToolArgs, ToolCommands};
use crate::x::claude_code::command::ClaudeCodeArgs;
use crate::x::codex::command::CodexArgs;
use crate::x::cursor::command::CursorArgs;
use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Configuration directory for llman (default: ~/.config/llman)
    /// Required when running within llman project for development
    #[arg(short = 'C', long = "config-dir", global = true)]
    pub config_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage prompts and rules
    #[command(alias = "rule")]
    Prompt(PromptArgs),
    /// Manage skills
    Skills(SkillsArgs),
    /// Spec-driven development workflow
    Sdd(SddArgs),
    /// Experimental commands
    X(XArgs),
    /// Developer tools
    Tool(ToolArgs),
}

#[derive(Parser)]
#[command(subcommand_required = false)]
pub struct PromptArgs {
    #[command(subcommand)]
    pub command: Option<PromptCommands>,
}

#[derive(Subcommand)]
pub enum PromptCommands {
    /// Generate a new prompt
    Gen {
        #[arg(short, long)]
        interactive: bool,
        #[arg(long, required_unless_present = "interactive")]
        app: Option<String>,
        #[arg(long, required_unless_present = "interactive")]
        template: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        force: bool,
    },
    /// List all prompts
    List {
        #[arg(long)]
        app: Option<String>,
    },
    /// Create or update a prompt
    Upsert {
        #[arg(long)]
        app: String,
        #[arg(long)]
        name: String,
        #[command(flatten)]
        content: ContentSource,
    },
    /// Remove a prompt
    Rm {
        #[arg(long)]
        app: String,
        #[arg(long)]
        name: String,
    },
}

#[derive(Parser)]
#[group(required = true, multiple = false)]
pub struct ContentSource {
    #[arg(long)]
    pub content: Option<String>,
    #[arg(long)]
    pub file: Option<PathBuf>,
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
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    // Determine and set config directory
    let config_dir = determine_config_dir(cli.config_dir.as_ref())?;

    // Set LLMAN_CONFIG_DIR environment variable for all subcommands
    unsafe {
        env::set_var(ENV_CONFIG_DIR, &config_dir);
    }

    match &cli.command {
        Commands::Prompt(args) => handle_prompt_command(args),
        Commands::Skills(args) => crate::skills::command::run(args),
        Commands::Sdd(args) => crate::sdd::command::run(args),
        Commands::X(args) => handle_x_command(args),
        Commands::Tool(args) => handle_tool_command(args),
    }
}

/// Determine the configuration directory to use
fn determine_config_dir(cli_config_dir: Option<&PathBuf>) -> Result<PathBuf> {
    let has_cli_override = cli_config_dir.is_some();
    let has_env_override = env::var(ENV_CONFIG_DIR).is_ok();

    // Check if we're in llman development project
    if !has_cli_override && !has_env_override && is_llman_dev_project() {
        let message = t!(
            "errors.dev_project_config_required",
            env_var = ENV_CONFIG_DIR
        );
        return Err(anyhow!(message));
    }

    resolve_config_dir(cli_config_dir.map(|path| path.as_path()))
}

/// Check if current directory is an llman development project
fn is_llman_dev_project() -> bool {
    if let Ok(current_dir) = env::current_dir() {
        let cargo_toml = current_dir.join("Cargo.toml");

        // Check for Cargo.toml with llman package name
        if cargo_toml.exists()
            && let Ok(content) = fs::read_to_string(&cargo_toml)
        {
            return content.contains("name = \"llman\"");
        }
    }
    false
}

fn handle_prompt_command(args: &PromptArgs) -> Result<()> {
    let prompt_cmd = PromptCommand::new()?;

    match &args.command {
        Some(PromptCommands::Gen {
            interactive,
            app,
            template,
            force,
            ..
        }) => {
            if *interactive {
                prompt_cmd.generate_interactive()?;
            } else {
                prompt_cmd.generate_rules(
                    app.as_deref().unwrap(),
                    template.as_deref().unwrap(),
                    *force,
                )?;
            }
        }
        Some(PromptCommands::List { app }) => {
            prompt_cmd.list_rules(app.as_deref())?;
        }
        Some(PromptCommands::Upsert { app, name, content }) => {
            prompt_cmd.upsert_rule(
                app,
                name,
                content.content.as_deref(),
                content.file.as_deref().and_then(|p| p.to_str()),
            )?;
        }
        Some(PromptCommands::Rm { app, name }) => {
            prompt_cmd.remove_rule(app, name)?;
        }
        None => {
            prompt_cmd.generate_interactive()?;
        }
    }
    Ok(())
}

fn handle_x_command(args: &XArgs) -> Result<()> {
    match &args.command {
        XCommands::Cursor(cursor_args) => crate::x::cursor::command::run(cursor_args),
        XCommands::ClaudeCode(claude_code_args) => {
            crate::x::claude_code::command::run(claude_code_args)
        }
        XCommands::Codex(codex_args) => crate::x::codex::command::run(codex_args),
    }
}

fn handle_tool_command(args: &ToolArgs) -> Result<()> {
    match &args.command {
        ToolCommands::CleanUselessComments(args) => crate::tool::clean_comments::run(args),
        ToolCommands::RmEmptyDirs(args) => crate::tool::rm_empty_dirs::run(args),
    }
}

#[cfg(test)]
mod tests {
    use super::determine_config_dir;
    use crate::config::ENV_CONFIG_DIR;
    use crate::test_utils::ENV_MUTEX;
    use std::env;

    #[test]
    fn test_config_dir_cli_overrides_env() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let env_dir = env::temp_dir().join("llman_env_dir");
        let cli_dir = env::temp_dir().join("llman_cli_dir");

        unsafe {
            env::set_var(ENV_CONFIG_DIR, &env_dir);
        }

        let resolved = determine_config_dir(Some(&cli_dir)).unwrap();
        assert_eq!(resolved, cli_dir);

        unsafe {
            env::remove_var(ENV_CONFIG_DIR);
        }
    }
}
