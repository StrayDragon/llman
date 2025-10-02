use crate::prompt::PromptCommand;
use crate::tool::command::{ToolArgs, ToolCommands};
use crate::x::collect::command::{CollectArgs, CollectCommands};
use crate::x::cursor::command::CursorArgs;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage prompts and rules
    #[command(alias = "rule")]
    Prompt(PromptArgs),
    /// Project utilities
    Project(ProjectArgs),
    /// Experimental commands
    X(XArgs),
    /// Developer tools
    Tool(ToolArgs),
}

#[derive(Parser)]
pub struct PromptArgs {
    #[command(subcommand)]
    pub command: PromptCommands,
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

#[derive(Parser)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommands,
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Generate directory tree structure
    Tree(crate::x::collect::tree::TreeArgs),
}

#[derive(Subcommand)]
pub enum XCommands {
    /// Commands for interacting with Cursor
    Cursor(CursorArgs),
    /// A collection of commands for collecting information
    Collect(CollectArgs),
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Prompt(args) => handle_prompt_command(args),
        Commands::Project(args) => handle_project_command(args),
        Commands::X(args) => handle_x_command(args),
        Commands::Tool(args) => handle_tool_command(args),
    }
}

fn handle_prompt_command(args: &PromptArgs) -> Result<()> {
    let prompt_cmd = PromptCommand::new()?;

    match &args.command {
        PromptCommands::Gen {
            interactive,
            app,
            template,
            force,
            ..
        } => {
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
        PromptCommands::List { app } => {
            prompt_cmd.list_rules(app.as_deref())?;
        }
        PromptCommands::Upsert { app, name, content } => {
            prompt_cmd.upsert_rule(
                app,
                name,
                content.content.as_deref(),
                content.file.as_deref().and_then(|p| p.to_str()),
            )?;
        }
        PromptCommands::Rm { app, name } => {
            prompt_cmd.remove_rule(app, name)?;
        }
    }
    Ok(())
}

fn handle_x_command(args: &XArgs) -> Result<()> {
    match &args.command {
        XCommands::Cursor(cursor_args) => crate::x::cursor::command::run(cursor_args),
        XCommands::Collect(collect_args) => match &collect_args.command {
            CollectCommands::Tree(tree_args) => crate::x::collect::tree::run(tree_args),
        },
    }
}

fn handle_project_command(args: &ProjectArgs) -> Result<()> {
    match &args.command {
        ProjectCommands::Tree(tree_args) => crate::x::collect::tree::run(tree_args),
    }
}

fn handle_tool_command(args: &ToolArgs) -> Result<()> {
    match &args.command {
        ToolCommands::CleanUselessComments(args) => crate::tool::clean_comments::run(args),
    }
}
