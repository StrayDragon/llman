use crate::usage_stats::{StatsCliArgs, ToolKind, validate_stats_cli_args};
use anyhow::{Result, bail};
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug, Clone)]
pub struct CursorStatsArgs {
    #[command(flatten)]
    pub stats: StatsCliArgs,

    /// Override the workspace Cursor state.vscdb path.
    #[arg(long)]
    pub db_path: Option<PathBuf>,

    /// Override the global Cursor state.vscdb path (used for bubble KV).
    #[arg(long)]
    pub global_db_path: Option<PathBuf>,
}

pub fn run_stats(args: &CursorStatsArgs) -> Result<()> {
    validate_stats_cli_args(&args.stats)?;
    bail!(
        "stats for {} is not implemented yet",
        tool_label(ToolKind::Cursor)
    )
}

fn tool_label(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Codex => "codex",
        ToolKind::ClaudeCode => "claude-code",
        ToolKind::Cursor => "cursor",
    }
}
