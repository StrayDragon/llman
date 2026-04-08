use clap::{Parser, Subcommand, ValueEnum};
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
    /// Remove useless directories recursively
    RmUselessDirs(RmUselessDirsArgs),
    /// Deprecated: use rm-useless-dirs instead
    #[command(hide = true)]
    RmEmptyDirs(RmUselessDirsArgs),
    /// Sync ignore rules across OpenCode/Cursor/Claude Code
    #[command(alias = "si")]
    SyncIgnore(SyncIgnoreArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct CleanUselessCommentsArgs {
    /// Configuration file path (default: .llman/config.yaml)
    #[arg(long, short = 'c')]
    pub config: Option<PathBuf>,

    /// Dry run mode, show changes without applying them
    #[arg(long, short = 'd')]
    pub dry_run: bool,

    /// Apply changes (default: dry run)
    #[arg(long, short = 'y')]
    pub yes: bool,

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
pub struct RmUselessDirsArgs {
    /// Configuration file path (default: .llman/config.yaml)
    #[arg(long, short = 'c')]
    pub config: Option<PathBuf>,

    /// Directory to scan (default: current directory)
    pub path: Option<PathBuf>,

    /// Actually delete empty directories (default: dry run)
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Path to a .gitignore file to honor (default: ./.gitignore)
    #[arg(long)]
    pub gitignore: Option<PathBuf>,

    /// Treat directories containing only ignored entries as removable (deletes ignored files/dirs)
    #[arg(long)]
    pub prune_ignored: bool,

    /// Verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SyncIgnoreTarget {
    /// Project root `.ignore` (OpenCode / ripgrep)
    Opencode,
    /// Project root `.cursorignore`
    Cursor,
    /// `.claude/settings.json`
    ClaudeShared,
    /// `.claude/settings.local.json`
    ClaudeLocal,
    /// All supported targets
    All,
}

#[derive(Parser, Debug, Clone)]
#[command(
    after_help = "Examples:\n  llman tool sync-ignore\n  llman tool sync-ignore -y\n  llman tool sync-ignore --target cursor --target claude-shared -y\n  llman tool sync-ignore --interactive\n"
)]
pub struct SyncIgnoreArgs {
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

    /// Output target(s) to sync (repeatable, or comma-separated)
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub target: Vec<SyncIgnoreTarget>,

    /// Additional input file path(s) to include as sources (repeatable)
    #[arg(long, short = 'i')]
    pub input: Vec<PathBuf>,
}
