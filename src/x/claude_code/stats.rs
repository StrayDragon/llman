use crate::usage_stats::{StatsCliArgs, ToolKind, validate_stats_cli_args};
use anyhow::{Result, bail};
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug, Clone)]
pub struct ClaudeCodeStatsArgs {
    #[command(flatten)]
    pub stats: StatsCliArgs,

    /// Override the Claude projects directory (default: ~/.claude/projects).
    #[arg(long)]
    pub projects_dir: Option<PathBuf>,

    /// Exclude sidechain/subagent sessions.
    #[arg(long)]
    pub no_sidechain: bool,
}

pub fn run_stats(args: &ClaudeCodeStatsArgs) -> Result<()> {
    validate_stats_cli_args(&args.stats)?;
    bail!(
        "stats for {} is not implemented yet",
        tool_label(ToolKind::ClaudeCode)
    )
}

fn tool_label(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Codex => "codex",
        ToolKind::ClaudeCode => "claude-code",
        ToolKind::Cursor => "cursor",
    }
}
