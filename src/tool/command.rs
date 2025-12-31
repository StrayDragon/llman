use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
pub struct ToolArgs {
    #[command(subcommand)]
    pub command: ToolCommands,
}

#[derive(Subcommand)]
pub enum ToolCommands {
    /// Clean useless comments from source code
    #[command(alias = "cuc")]
    CleanUselessComments(CleanUselessCommentsArgs),
    /// Remove empty directories recursively
    RmEmptyDirs(RmEmptyDirsArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct CleanUselessCommentsArgs {
    /// Configuration file path (default: .llman/config.yaml)
    #[arg(long, short = 'c')]
    pub config: Option<PathBuf>,

    /// Dry run mode, show changes without applying them
    #[arg(long, short = 'd')]
    pub dry_run: bool,

    /// Interactive mode, confirm changes before applying
    #[arg(long, short = 'i')]
    pub interactive: bool,

    /// Force execution, skip confirmation prompts
    #[arg(long, short = 'f')]
    pub force: bool,

    /// Verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Only process Git tracked files
    #[arg(long)]
    pub git_only: bool,

    /// Files to process (if not specified, use config scope)
    pub files: Vec<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
pub struct RmEmptyDirsArgs {
    /// Directory to scan (default: current directory)
    pub path: Option<PathBuf>,

    /// Actually delete empty directories (default: dry run)
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Path to a .gitignore file to honor (default: ./.gitignore)
    #[arg(long)]
    pub gitignore: Option<PathBuf>,

    /// Verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,
}
