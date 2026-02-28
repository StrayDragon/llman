use crate::usage_stats::tui::{ScanFn, StatsTuiScanRequest, run_stats_tui};
use crate::usage_stats::{
    OutputFormat, RenderOptions, SessionId, SessionRecord, StatsCliArgs, StatsJsonOutput,
    StatsQuery, StatsViewResult, ToolKind, ViewKind, build_session_detail_view,
    build_sessions_view, build_summary_view, build_trend_view, filter_sessions_v1,
    parse_time_range, render_stats_json, render_stats_table, validate_stats_cli_args,
};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use clap::Args;
use serde::Deserialize;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

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
    let cwd = std::env::current_dir().context("resolve current directory")?;
    let projects_dir = resolve_projects_dir(args.projects_dir.as_deref())?;
    let include_sidechain = !args.no_sidechain;

    if args.stats.tui {
        let initial = StatsTuiScanRequest {
            cwd: cwd.clone(),
            group_by: args.stats.group_by,
            range: args.stats.range.clone(),
            limit: args.stats.limit,
            verbose_paths: args.stats.verbose,
            with_breakdown: false,
            include_sidechain,
        };

        let scan_projects_dir = projects_dir.clone();
        let scan_fn: ScanFn = Arc::new(move |request, _tx| {
            let sessions =
                load_claude_sessions(&scan_projects_dir, &request.cwd, request.include_sidechain)?;

            let time_range = parse_time_range(&request.range, Utc::now())?;
            let query = StatsQuery {
                view: ViewKind::Summary,
                group_by: request.group_by,
                time_range,
                cwd: request.cwd.clone(),
                limit: request.limit,
                id: None,
            };
            Ok(filter_sessions_v1(&sessions, &query))
        });

        return run_stats_tui(ToolKind::ClaudeCode, initial, scan_fn);
    }

    let time_range = parse_time_range(&args.stats.range, Utc::now())?;

    let id = args
        .stats
        .id
        .as_deref()
        .map(|raw| SessionId(raw.to_string()));

    let query = StatsQuery {
        view: args.stats.view,
        group_by: args.stats.group_by,
        time_range,
        cwd,
        limit: args.stats.limit,
        id,
    };

    let sessions = load_claude_sessions(&projects_dir, &query.cwd, include_sidechain)?;
    let sessions = filter_sessions_v1(&sessions, &query);

    let view = match query.view {
        ViewKind::Summary => StatsViewResult::Summary(build_summary_view(&sessions)),
        ViewKind::Trend => StatsViewResult::Trend(build_trend_view(&sessions, query.group_by)),
        ViewKind::Sessions => StatsViewResult::Sessions(build_sessions_view(&sessions, query.limit)),
        ViewKind::Session => {
            let id = query.id.as_ref().expect("validated");
            let Some(view) = build_session_detail_view(&sessions, id) else {
                bail!("session not found for id: {id}");
            };
            StatsViewResult::Session(view)
        }
    };

    let output = StatsJsonOutput {
        tool: ToolKind::ClaudeCode,
        query,
        view,
    };

    match args.stats.format {
        OutputFormat::Json => {
            let json = render_stats_json(&output)?;
            println!("{json}");
        }
        OutputFormat::Table => {
            let table = render_stats_table(
                &output,
                &RenderOptions {
                    verbose_paths: args.stats.verbose,
                },
            );
            print!("{table}");
        }
    }

    Ok(())
}

fn resolve_projects_dir(override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    let home = crate::config::home_dir().context("resolve home directory")?;
    Ok(home.join(".claude").join("projects"))
}

#[derive(Debug, Clone, Deserialize)]
struct SessionsIndex {
    #[serde(default)]
    entries: Vec<SessionsIndexEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct SessionsIndexEntry {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "fullPath")]
    full_path: String,
    #[serde(rename = "projectPath")]
    project_path: String,
    #[serde(rename = "isSidechain", default)]
    is_sidechain: bool,
    #[serde(default)]
    created: Option<String>,
    #[serde(default)]
    modified: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ClaudeJsonlLine {
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    message: Option<ClaudeMessage>,
}

#[derive(Debug, Clone, Deserialize)]
struct ClaudeMessage {
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Clone, Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
}

#[derive(Default)]
struct UsageAccum {
    input_any: bool,
    output_any: bool,
    cache_any: bool,
    input_sum: u64,
    output_sum: u64,
    cache_sum: u64,
}

impl UsageAccum {
    fn add(&mut self, usage: &ClaudeUsage) {
        if let Some(v) = usage.input_tokens {
            self.input_any = true;
            self.input_sum = self.input_sum.saturating_add(v);
        }
        if let Some(v) = usage.output_tokens {
            self.output_any = true;
            self.output_sum = self.output_sum.saturating_add(v);
        }
        if let Some(v) = usage.cache_read_input_tokens {
            self.cache_any = true;
            self.cache_sum = self.cache_sum.saturating_add(v);
        }
        if let Some(v) = usage.cache_creation_input_tokens {
            self.cache_any = true;
            self.cache_sum = self.cache_sum.saturating_add(v);
        }
    }

    fn build(self) -> crate::usage_stats::TokenUsage {
        let input = self.input_any.then_some(self.input_sum);
        let output = self.output_any.then_some(self.output_sum);
        let cache = self.cache_any.then_some(self.cache_sum);

        let mut total_sum = 0u64;
        let mut any = false;
        for v in [input, output, cache] {
            if let Some(v) = v {
                any = true;
                total_sum = total_sum.saturating_add(v);
            }
        }

        crate::usage_stats::TokenUsage {
            total: any.then_some(total_sum),
            input,
            output,
            cache,
            reasoning: None,
        }
    }
}

fn load_claude_sessions(
    projects_dir: &Path,
    cwd_filter: &Path,
    include_sidechain: bool,
) -> Result<Vec<SessionRecord>> {
    let mut sessions = Vec::new();

    let entries = fs::read_dir(projects_dir).with_context(|| {
        format!(
            "read Claude projects directory: {}",
            projects_dir.display()
        )
    })?;

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let project_dir = entry.path();
        if !project_dir.is_dir() {
            continue;
        }

        let index_path = project_dir.join("sessions-index.json");
        if !index_path.exists() {
            continue;
        }

        let index_raw = match fs::read_to_string(&index_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let index: SessionsIndex = match serde_json::from_str(&index_raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        for entry in index.entries {
            if !include_sidechain && entry.is_sidechain {
                continue;
            }

            let project_path = PathBuf::from(&entry.project_path);
            if project_path != cwd_filter {
                continue;
            }

            let jsonl_path = PathBuf::from(&entry.full_path);
            let (usage, start_ts, end_ts) = if jsonl_path.exists() {
                parse_session_jsonl(&jsonl_path).unwrap_or_default()
            } else {
                Default::default()
            };

            let fallback_created = entry.created.as_deref().and_then(parse_rfc3339_utc);
            let fallback_modified = entry
                .modified
                .as_deref()
                .and_then(parse_rfc3339_utc)
                .or(fallback_created);

            let start_ts = start_ts.or(fallback_created);
            let Some(end_ts) = end_ts.or(fallback_modified) else {
                continue;
            };

            sessions.push(SessionRecord {
                tool: ToolKind::ClaudeCode,
                id: SessionId(entry.session_id),
                cwd: project_path,
                title: None,
                start_ts,
                end_ts,
                token_usage: usage,
                is_sidechain: Some(entry.is_sidechain),
            });
        }
    }

    Ok(sessions)
}

fn parse_session_jsonl(
    path: &Path,
) -> Result<(
    crate::usage_stats::TokenUsage,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
)> {
    let file =
        File::open(path).with_context(|| format!("open Claude session jsonl: {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut usage = UsageAccum::default();
    let mut min_ts: Option<DateTime<Utc>> = None;
    let mut max_ts: Option<DateTime<Utc>> = None;

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Ok(event) = serde_json::from_str::<ClaudeJsonlLine>(trimmed) else {
            continue;
        };

        if let Some(ts) = event.timestamp.as_deref().and_then(parse_rfc3339_utc) {
            min_ts = Some(min_ts.map_or(ts, |current| current.min(ts)));
            max_ts = Some(max_ts.map_or(ts, |current| current.max(ts)));
        }

        if let Some(msg) = event.message
            && let Some(usage_line) = msg.usage
        {
            usage.add(&usage_line);
        }
    }

    Ok((usage.build(), min_ts, max_ts))
}

fn parse_rfc3339_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_session_jsonl_sums_known_usage_and_timestamps() {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join("s.jsonl");
        fs::write(
            &path,
            r#"
{"timestamp":"2026-02-01T00:00:00Z","message":{"usage":{"input_tokens":10,"output_tokens":5}}}
{"timestamp":"2026-02-02T00:00:00Z","message":{"usage":{"cache_read_input_tokens":2,"cache_creation_input_tokens":3}}}
"#,
        )
        .expect("write jsonl");

        let (usage, start, end) = parse_session_jsonl(&path).expect("parse");
        assert_eq!(usage.input, Some(10));
        assert_eq!(usage.output, Some(5));
        assert_eq!(usage.cache, Some(5));
        assert_eq!(usage.total, Some(20));
        assert!(start.is_some());
        assert!(end.is_some());
        assert!(end.unwrap() > start.unwrap());
    }
}
