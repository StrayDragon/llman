use crate::tool::command::{SyncIgnoreArgs as ToolSyncIgnoreArgs, SyncIgnoreTarget};
use crate::x::cursor::prompts::CursorPromptsArgs;
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
#[command(
    author,
    version,
    about,
    long_about = "Commands for interacting with Cursor"
)]
pub struct CursorArgs {
    #[command(subcommand)]
    pub command: CursorCommands,
}

#[derive(Subcommand)]
pub enum CursorCommands {
    /// Manage Cursor prompt templates and rules
    Prompts(CursorPromptsArgs),
    /// Sync ignore rules to Cursor / other targets (forward to `llman tool sync-ignore`)
    #[command(name = "sync-ignore", alias = "si")]
    SyncIgnore(CursorSyncIgnoreArgs),
}

#[derive(Args, Debug, Clone)]
pub struct CursorSyncIgnoreArgs {
    /// Apply changes (default: dry-run preview)
    #[arg(short = 'y', long, action = clap::ArgAction::SetTrue)]
    pub yes: bool,

    /// Interactive mode (MultiSelect targets + preview + confirm)
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub interactive: bool,

    /// Force execution even when no git repository is found
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub force: bool,

    /// Verbose output
    #[arg(long, short = 'v', action = clap::ArgAction::SetTrue)]
    pub verbose: bool,

    /// Output target(s) to sync (default: cursor)
    #[arg(
        long,
        short = 't',
        value_enum,
        value_delimiter = ',',
        default_value = "cursor"
    )]
    pub target: Vec<SyncIgnoreTarget>,

    /// Additional input file path(s) to include as sources (repeatable)
    #[arg(long, short = 'i')]
    pub input: Vec<PathBuf>,
}

pub fn run(args: &CursorArgs) -> Result<()> {
    match &args.command {
        CursorCommands::Prompts(prompts) => crate::x::cursor::prompts::run(prompts),
        CursorCommands::SyncIgnore(sync_args) => {
            crate::tool::sync_ignore::run(&ToolSyncIgnoreArgs {
                yes: sync_args.yes,
                interactive: sync_args.interactive,
                force: sync_args.force,
                verbose: sync_args.verbose,
                target: sync_args.target.clone(),
                input: sync_args.input.clone(),
            })
        }
    }
}
