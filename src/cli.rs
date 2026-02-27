use crate::agents::command::AgentsArgs;
use crate::config::{ENV_CONFIG_DIR, resolve_config_dir_with};
use crate::config_schema::ensure_global_sample_config;
use crate::prompt::PromptCommand;
use crate::sdd::command::SddArgs;
use crate::self_command::SelfArgs;
use crate::skills::cli::command::SkillsArgs;
use crate::skills::cli::interactive::is_interactive;
use crate::tool::command::{ToolArgs, ToolCommands};
use crate::x::arena::command::ArenaArgs;
use crate::x::claude_code::command::ClaudeCodeArgs;
use crate::x::codex::command::CodexArgs;
use crate::x::cursor::command::CursorArgs;
use anyhow::{Result, anyhow};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
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
    /// Manage prompts and rules
    #[command(name = "prompts", aliases = ["prompt", "rule"])]
    Prompt(PromptArgs),
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
        /// Target scope for injection (codex/claude-code only)
        #[arg(long, value_enum, default_value = "project")]
        scope: PromptScopeArg,
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
        /// Skip confirmation prompts (required for non-interactive deletes)
        #[arg(long)]
        yes: bool,
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
    /// Arena: prompt/model challenge workflow
    Arena(ArenaArgs),
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum PromptScopeArg {
    User,
    Project,
    All,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.print_config_dir_path {
        let env_override = env::var(ENV_CONFIG_DIR).ok();
        let config_dir =
            resolve_config_dir_with(cli.config_dir.as_deref(), env_override.as_deref())?;
        println!("{}", config_dir.display());
        return Ok(());
    }

    if cli.command.is_none() {
        let mut command = Cli::command();
        command.print_help()?;
        println!();
        return Ok(());
    }

    // Determine and set config directory
    let config_dir = determine_config_dir(cli.config_dir.as_ref())?;

    set_config_dir_env(&config_dir);
    ensure_global_sample_config(&config_dir)?;

    match cli.command.as_ref().expect("command is present") {
        Commands::Prompt(args) => handle_prompt_command(args),
        Commands::Skills(args) => crate::skills::cli::command::run(args),
        Commands::Agents(args) => crate::agents::command::run(args),
        Commands::Sdd(args) => crate::sdd::command::run(args),
        Commands::X(args) => handle_x_command(args),
        Commands::Tool(args) => handle_tool_command(args),
        Commands::SelfCommand(args) => crate::self_command::run(args),
    }
}

fn set_config_dir_env(config_dir: &Path) {
    // SAFETY: Setting env vars is a process-global mutation that can be unsound if other threads
    // concurrently access environment variables. We do this at CLI startup before spawning any
    // background work, and only to pass config context to subcommands. If llman grows concurrent
    // startup behavior, refactor to pass config_dir through an explicit context instead.
    unsafe {
        env::set_var(ENV_CONFIG_DIR, config_dir);
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

fn handle_prompt_command(args: &PromptArgs) -> Result<()> {
    let prompt_cmd = PromptCommand::new()?;

    match &args.command {
        Some(PromptCommands::Gen {
            interactive,
            app,
            template,
            scope,
            name,
            force,
            ..
        }) => {
            if *interactive {
                prompt_cmd.generate_interactive()?;
            } else {
                prompt_cmd.generate_rules(
                    app.as_deref().unwrap(),
                    template.as_deref().unwrap(),
                    name.as_deref(),
                    (*scope).into(),
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
        Some(PromptCommands::Rm { app, name, yes }) => {
            prompt_cmd.remove_rule(app, name, *yes, is_interactive())?;
        }
        None => {
            prompt_cmd.generate_interactive()?;
        }
    }
    Ok(())
}

impl From<PromptScopeArg> for crate::prompt::PromptScope {
    fn from(value: PromptScopeArg) -> Self {
        match value {
            PromptScopeArg::User => Self::User,
            PromptScopeArg::Project => Self::Project,
            PromptScopeArg::All => Self::All,
        }
    }
}

fn handle_x_command(args: &XArgs) -> Result<()> {
    match &args.command {
        XCommands::Cursor(cursor_args) => crate::x::cursor::command::run(cursor_args),
        XCommands::ClaudeCode(claude_code_args) => {
            crate::x::claude_code::command::run(claude_code_args)
        }
        XCommands::Codex(codex_args) => crate::x::codex::command::run(codex_args),
        XCommands::Arena(arena_args) => crate::x::arena::command::run(arena_args),
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
    }
}

#[cfg(test)]
mod tests {
    use crate::config::resolve_config_dir_with;
    use crate::test_utils::TestProcess;
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

        let mut proc = TestProcess::new();
        proc.chdir(temp.path()).expect("chdir");

        assert!(!super::is_llman_dev_project());
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

        let mut proc = TestProcess::new();
        proc.chdir(temp.path()).expect("chdir");

        assert!(super::is_llman_dev_project());
    }
}
