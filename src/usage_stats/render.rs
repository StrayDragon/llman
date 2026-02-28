use crate::usage_stats::aggregate::{SessionDetailView, SessionsView, SummaryView, TrendView};
use crate::usage_stats::model::ToolKind;
use crate::usage_stats::path_display::display_path;
use crate::usage_stats::query::{StatsQuery, TimeRange, TimeRangeMode};
use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub verbose_paths: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "view", content = "result", rename_all = "kebab-case")]
pub enum StatsViewResult {
    Summary(SummaryView),
    Trend(TrendView),
    Sessions(SessionsView),
    Session(SessionDetailView),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsJsonOutput {
    pub tool: ToolKind,
    pub query: StatsQuery,
    #[serde(flatten)]
    pub view: StatsViewResult,
}

pub fn render_stats_json(output: &StatsJsonOutput) -> Result<String> {
    serde_json::to_string_pretty(output).context("serialize stats json")
}

pub fn render_stats_table(output: &StatsJsonOutput, options: &RenderOptions) -> String {
    let mut out = String::new();

    let tool = tool_label(output.tool);
    let range = format_time_range(&output.query.time_range);

    let _ = writeln!(out, "Tool: {tool}");
    let _ = writeln!(out, "Range: {range}");

    match &output.view {
        StatsViewResult::Summary(view) => {
            let _ = writeln!(out, "View: summary");
            let _ = writeln!(
                out,
                "Sessions: total={} known_tokens={}",
                view.coverage.total_sessions, view.coverage.known_token_sessions
            );
            let _ = writeln!(
                out,
                "Tokens (known-only): total={}",
                view.totals.tokens_total_known
            );
            if let Some(v) = view.totals.tokens_input_known {
                let _ = writeln!(out, "  input={v}");
            }
            if let Some(v) = view.totals.tokens_output_known {
                let _ = writeln!(out, "  output={v}");
            }
            if let Some(v) = view.totals.tokens_cache_known {
                let _ = writeln!(out, "  cache={v}");
            }
            if let Some(v) = view.totals.tokens_reasoning_known {
                let _ = writeln!(out, "  reasoning={v}");
            }
            if let Some(ts) = view.latest_end_ts {
                let _ = writeln!(out, "Latest: {}", format_dt_local(ts));
            }
        }
        StatsViewResult::Trend(view) => {
            let _ = writeln!(out, "View: trend");
            let _ = writeln!(out, "Group-by: {}", group_by_label(view.group_by));
            let _ = writeln!(out, "bucket\tknown_tokens\tsessions(known/total)");
            for bucket in &view.buckets {
                let _ = writeln!(
                    out,
                    "{}\t{}\t{}/{}",
                    bucket.label,
                    bucket.totals.tokens_total_known,
                    bucket.coverage.known_token_sessions,
                    bucket.coverage.total_sessions
                );
            }
        }
        StatsViewResult::Sessions(view) => {
            let _ = writeln!(out, "View: sessions");
            let _ = writeln!(
                out,
                "Sessions: returned={} total={}",
                view.returned_sessions, view.total_sessions
            );
            let _ = writeln!(out, "end\tknown_tokens\tid\tcwd\ttitle");
            for session in &view.sessions {
                let end = format_dt_local(session.end_ts);
                let tokens = session
                    .token_usage
                    .total
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let cwd = display_path(&session.cwd, &output.query.cwd, options.verbose_paths);
                let title = session.title.as_deref().unwrap_or("-");
                let _ = writeln!(
                    out,
                    "{end}\t{tokens}\t{}\t{cwd}\t{}",
                    session.id,
                    truncate(title, 48)
                );
            }
        }
        StatsViewResult::Session(view) => {
            let session = &view.session;
            let _ = writeln!(out, "View: session");
            let _ = writeln!(out, "Id: {}", session.id);
            let _ = writeln!(out, "End: {}", format_dt_local(session.end_ts));
            if let Some(start) = session.start_ts {
                let _ = writeln!(out, "Start: {}", format_dt_local(start));
            }
            let cwd = display_path(&session.cwd, &output.query.cwd, options.verbose_paths);
            let _ = writeln!(out, "Cwd: {cwd}");
            if let Some(title) = &session.title {
                let _ = writeln!(out, "Title: {title}");
            }
            if let Some(total) = session.token_usage.total {
                let _ = writeln!(out, "Tokens (known): total={total}");
            } else {
                let _ = writeln!(out, "Tokens: unknown");
            }
            if let Some(v) = session.token_usage.input {
                let _ = writeln!(out, "  input={v}");
            }
            if let Some(v) = session.token_usage.output {
                let _ = writeln!(out, "  output={v}");
            }
            if let Some(v) = session.token_usage.cache {
                let _ = writeln!(out, "  cache={v}");
            }
            if let Some(v) = session.token_usage.reasoning {
                let _ = writeln!(out, "  reasoning={v}");
            }
        }
    }

    out
}

fn tool_label(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Codex => "codex",
        ToolKind::ClaudeCode => "claude-code",
        ToolKind::Cursor => "cursor",
    }
}

fn group_by_label(group_by: crate::usage_stats::query::GroupBy) -> &'static str {
    match group_by {
        crate::usage_stats::query::GroupBy::Day => "day",
        crate::usage_stats::query::GroupBy::Week => "week",
        crate::usage_stats::query::GroupBy::Month => "month",
    }
}

fn format_time_range(range: &TimeRange) -> String {
    match range.mode {
        TimeRangeMode::All => "all".to_string(),
        TimeRangeMode::LastDays => range
            .last_days
            .map(|days| format!("last {days}d"))
            .unwrap_or_else(|| "last".to_string()),
        TimeRangeMode::SinceUntil => {
            let since = range.since.map(format_dt_local_short);
            let until = range.until.map(format_dt_local_short);
            match (since, until) {
                (Some(since), Some(until)) => format!("since {since} until {until} (exclusive)"),
                (Some(since), None) => format!("since {since}"),
                (None, Some(until)) => format!("until {until} (exclusive)"),
                (None, None) => "all".to_string(),
            }
        }
    }
}

fn format_dt_local(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn format_dt_local_short(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out = String::with_capacity(max_chars + 1);
    for (i, ch) in s.chars().enumerate() {
        if i + 1 >= max_chars {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage_stats::aggregate::{Coverage, TokenTotals};
    use crate::usage_stats::model::{SessionId, SessionRecord, TokenUsage};
    use crate::usage_stats::query::{GroupBy, TimeRange};
    use chrono::TimeZone;
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn json_output_is_parseable() {
        let query = StatsQuery {
            view: crate::usage_stats::query::ViewKind::Summary,
            group_by: GroupBy::Day,
            time_range: TimeRange::all(),
            cwd: PathBuf::from("/p"),
            limit: 200,
            id: None,
        };
        let output = StatsJsonOutput {
            tool: ToolKind::Codex,
            query,
            view: StatsViewResult::Summary(SummaryView {
                totals: TokenTotals {
                    tokens_total_known: 123,
                    tokens_input_known: None,
                    tokens_output_known: None,
                    tokens_cache_known: None,
                    tokens_reasoning_known: None,
                },
                coverage: Coverage {
                    total_sessions: 2,
                    known_token_sessions: 1,
                },
                latest_end_ts: None,
            }),
        };

        let json = render_stats_json(&output).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["tool"], "codex");
        assert_eq!(v["view"], "summary");
        assert_eq!(v["result"]["coverage"]["total_sessions"], 2);
    }

    #[test]
    fn table_output_includes_view_and_rows() {
        let query = StatsQuery {
            view: crate::usage_stats::query::ViewKind::Sessions,
            group_by: GroupBy::Day,
            time_range: TimeRange::all(),
            cwd: PathBuf::from("/repo"),
            limit: 200,
            id: None,
        };
        let session = SessionRecord {
            tool: ToolKind::Codex,
            id: SessionId("t1".to_string()),
            cwd: PathBuf::from("/repo/a/b"),
            title: Some("hello".to_string()),
            start_ts: None,
            end_ts: Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
            token_usage: TokenUsage {
                total: Some(10),
                ..TokenUsage::default()
            },
            is_sidechain: None,
        };
        let output = StatsJsonOutput {
            tool: ToolKind::Codex,
            query,
            view: StatsViewResult::Sessions(SessionsView {
                total_sessions: 1,
                returned_sessions: 1,
                sessions: vec![session],
            }),
        };
        let table = render_stats_table(
            &output,
            &RenderOptions {
                verbose_paths: false,
            },
        );
        assert!(table.contains("View: sessions"));
        assert!(table.contains("t1"));
    }
}
