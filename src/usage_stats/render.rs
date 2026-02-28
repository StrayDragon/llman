use crate::usage_stats::aggregate::{SessionDetailView, SessionsView, SummaryView, TrendView};
use crate::usage_stats::model::ToolKind;
use crate::usage_stats::path_display::display_path;
use crate::usage_stats::query::{ColorMode, StatsQuery, TimeRange, TimeRangeMode};
use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub verbose_paths: bool,
    pub color: ColorMode,
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
    let ansi_enabled = should_enable_ansi(
        options.color,
        stdout_is_tty(),
        std::env::var_os("NO_COLOR").is_some(),
    );

    let tool = tool_label(output.tool);
    let range = format_time_range(&output.query.time_range);
    let view = view_label(&output.view);

    let mut meta = table_base(ansi_enabled);
    meta.add_row(vec![
        cell_key("Tool", ansi_enabled),
        Cell::new(sanitize_cell_text(tool)),
    ]);
    meta.add_row(vec![
        cell_key("Range", ansi_enabled),
        Cell::new(sanitize_cell_text(&range)),
    ]);
    meta.add_row(vec![
        cell_key("View", ansi_enabled),
        Cell::new(sanitize_cell_text(view)),
    ]);
    if let StatsViewResult::Sessions(sessions) = &output.view {
        meta.add_row(vec![
            cell_key("Sessions (returned)", ansi_enabled),
            Cell::new(format_usize_count(sessions.returned_sessions)),
        ]);
        meta.add_row(vec![
            cell_key("Sessions (total)", ansi_enabled),
            Cell::new(format_usize_count(sessions.total_sessions)),
        ]);
    }
    if let StatsViewResult::Trend(trend) = &output.view {
        meta.add_row(vec![
            cell_key("Group-by", ansi_enabled),
            Cell::new(sanitize_cell_text(group_by_label(trend.group_by))),
        ]);
    }

    let mut out = String::new();
    out.push_str(&meta.to_string());
    out.push('\n');
    out.push('\n');

    let view_table = match &output.view {
        StatsViewResult::Summary(view) => render_summary_table(view, ansi_enabled),
        StatsViewResult::Trend(view) => render_trend_table(view, ansi_enabled),
        StatsViewResult::Sessions(view) => {
            render_sessions_table(output.tool, view, &output.query, options, ansi_enabled)
        }
        StatsViewResult::Session(view) => {
            render_session_detail_table(output.tool, view, &output.query, options, ansi_enabled)
        }
    };

    out.push_str(&view_table.to_string());
    out.push('\n');

    out
}

fn stdout_is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

fn should_enable_ansi(color: ColorMode, stdout_is_tty: bool, no_color: bool) -> bool {
    match color {
        ColorMode::Auto => stdout_is_tty && !no_color,
        ColorMode::Always => true,
        ColorMode::Never => false,
    }
}

fn table_base(ansi_enabled: bool) -> Table {
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    if ansi_enabled {
        table.enforce_styling();
    }
    table
}

fn cell_key(text: &str, ansi_enabled: bool) -> Cell {
    let mut cell = Cell::new(sanitize_cell_text(text));
    if ansi_enabled {
        cell = cell.fg(Color::Yellow).add_attribute(Attribute::Bold);
    }
    cell
}

fn cell_header(text: &str, ansi_enabled: bool) -> Cell {
    let mut cell = Cell::new(sanitize_cell_text(text));
    if ansi_enabled {
        cell = cell.fg(Color::Cyan).add_attribute(Attribute::Bold);
    }
    cell
}

fn sanitize_cell_text(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            '\t' | '\r' | '\n' => ' ',
            other => other,
        })
        .collect()
}

fn format_u64_count(value: u64) -> String {
    let s = value.to_string();
    let mut out = String::with_capacity(s.len().saturating_add(s.len() / 3));
    for (idx, ch) in s.chars().enumerate() {
        out.push(ch);
        let remaining = s.len().saturating_sub(idx).saturating_sub(1);
        if remaining > 0 && remaining % 3 == 0 {
            out.push(',');
        }
    }
    out
}

fn format_usize_count(value: usize) -> String {
    format_u64_count(value as u64)
}

fn format_opt_u64_count(value: Option<u64>) -> String {
    value
        .map(format_u64_count)
        .unwrap_or_else(|| "-".to_string())
}

fn view_label(view: &StatsViewResult) -> &'static str {
    match view {
        StatsViewResult::Summary(_) => "summary",
        StatsViewResult::Trend(_) => "trend",
        StatsViewResult::Sessions(_) => "sessions",
        StatsViewResult::Session(_) => "session",
    }
}

fn render_summary_table(view: &SummaryView, ansi_enabled: bool) -> Table {
    let mut table = table_base(ansi_enabled);
    table.set_header(vec![
        cell_header("Metric", ansi_enabled),
        cell_header("Value", ansi_enabled),
    ]);

    table.add_row(vec![
        Cell::new("Sessions (total)"),
        Cell::new(format_usize_count(view.coverage.total_sessions)),
    ]);
    table.add_row(vec![
        Cell::new("Sessions (known tokens)"),
        Cell::new(format_usize_count(view.coverage.known_token_sessions)),
    ]);
    table.add_row(vec![
        Cell::new("Tokens (known-only total)"),
        Cell::new(format_u64_count(view.totals.tokens_total_known)),
    ]);

    if let Some(sidechain) = &view.sidechain_totals {
        table.add_row(vec![
            Cell::new("Tokens (known-only primary)"),
            Cell::new(format_u64_count(sidechain.primary.tokens_total_known)),
        ]);
        table.add_row(vec![
            Cell::new("Tokens (known-only sidechain)"),
            Cell::new(format_u64_count(sidechain.sidechain.tokens_total_known)),
        ]);
    }

    table.add_row(vec![
        Cell::new("Tokens (known input)"),
        Cell::new(format_opt_u64_count(view.totals.tokens_input_known)),
    ]);
    table.add_row(vec![
        Cell::new("Tokens (known output)"),
        Cell::new(format_opt_u64_count(view.totals.tokens_output_known)),
    ]);
    table.add_row(vec![
        Cell::new("Tokens (known cache)"),
        Cell::new(format_opt_u64_count(view.totals.tokens_cache_known)),
    ]);
    table.add_row(vec![
        Cell::new("Tokens (known reasoning)"),
        Cell::new(format_opt_u64_count(view.totals.tokens_reasoning_known)),
    ]);
    table.add_row(vec![
        Cell::new("Latest"),
        Cell::new(
            view.latest_end_ts
                .map(format_dt_local)
                .unwrap_or_else(|| "-".to_string()),
        ),
    ]);

    table
}

fn render_trend_table(view: &TrendView, ansi_enabled: bool) -> Table {
    let mut table = table_base(ansi_enabled);
    let has_sidechain = view
        .buckets
        .iter()
        .any(|bucket| bucket.sidechain_totals.is_some());

    if has_sidechain {
        table.set_header(vec![
            cell_header("Bucket", ansi_enabled),
            cell_header("Overall", ansi_enabled),
            cell_header("Primary", ansi_enabled),
            cell_header("Sidechain", ansi_enabled),
            cell_header("Sessions (known/total)", ansi_enabled),
        ]);

        for bucket in &view.buckets {
            let (primary, sidechain) = bucket
                .sidechain_totals
                .as_ref()
                .map(|totals| {
                    (
                        format_u64_count(totals.primary.tokens_total_known),
                        format_u64_count(totals.sidechain.tokens_total_known),
                    )
                })
                .unwrap_or_else(|| ("-".to_string(), "-".to_string()));

            table.add_row(vec![
                Cell::new(sanitize_cell_text(&bucket.label)),
                Cell::new(format_u64_count(bucket.totals.tokens_total_known)),
                Cell::new(primary),
                Cell::new(sidechain),
                Cell::new(format!(
                    "{}/{}",
                    format_usize_count(bucket.coverage.known_token_sessions),
                    format_usize_count(bucket.coverage.total_sessions)
                )),
            ]);
        }
    } else {
        table.set_header(vec![
            cell_header("Bucket", ansi_enabled),
            cell_header("Known tokens", ansi_enabled),
            cell_header("Sessions (known/total)", ansi_enabled),
        ]);
        for bucket in &view.buckets {
            table.add_row(vec![
                Cell::new(sanitize_cell_text(&bucket.label)),
                Cell::new(format_u64_count(bucket.totals.tokens_total_known)),
                Cell::new(format!(
                    "{}/{}",
                    format_usize_count(bucket.coverage.known_token_sessions),
                    format_usize_count(bucket.coverage.total_sessions)
                )),
            ]);
        }
    }

    table
}

fn render_sessions_table(
    tool: ToolKind,
    view: &SessionsView,
    query: &StatsQuery,
    options: &RenderOptions,
    ansi_enabled: bool,
) -> Table {
    let mut table = table_base(ansi_enabled);

    let is_claude = tool == ToolKind::ClaudeCode;
    if is_claude {
        table.set_header(vec![
            cell_header("End", ansi_enabled),
            cell_header("Known tokens", ansi_enabled),
            cell_header("Id", ansi_enabled),
            cell_header("Sidechain", ansi_enabled),
            cell_header("Cwd", ansi_enabled),
            cell_header("Title", ansi_enabled),
        ]);
    } else {
        table.set_header(vec![
            cell_header("End", ansi_enabled),
            cell_header("Known tokens", ansi_enabled),
            cell_header("Id", ansi_enabled),
            cell_header("Cwd", ansi_enabled),
            cell_header("Title", ansi_enabled),
        ]);
    }

    for session in &view.sessions {
        let end = format_dt_local(session.end_ts);
        let tokens = format_opt_u64_count(session.token_usage.total);
        let cwd = display_path(&session.cwd, &query.cwd, options.verbose_paths);
        let title = session.title.as_deref().unwrap_or("-");
        if is_claude {
            let sidechain = match session.is_sidechain {
                Some(true) => "yes",
                Some(false) => "no",
                None => "-",
            };
            table.add_row(vec![
                Cell::new(end),
                Cell::new(tokens),
                Cell::new(sanitize_cell_text(&session.id.to_string())),
                Cell::new(sidechain),
                Cell::new(sanitize_cell_text(&cwd)),
                Cell::new(sanitize_cell_text(&truncate(title, 48))),
            ]);
        } else {
            table.add_row(vec![
                Cell::new(end),
                Cell::new(tokens),
                Cell::new(sanitize_cell_text(&session.id.to_string())),
                Cell::new(sanitize_cell_text(&cwd)),
                Cell::new(sanitize_cell_text(&truncate(title, 48))),
            ]);
        }
    }

    table
}

fn render_session_detail_table(
    tool: ToolKind,
    view: &SessionDetailView,
    query: &StatsQuery,
    options: &RenderOptions,
    ansi_enabled: bool,
) -> Table {
    let session = &view.session;

    let mut table = table_base(ansi_enabled);
    table.set_header(vec![
        cell_header("Field", ansi_enabled),
        cell_header("Value", ansi_enabled),
    ]);

    table.add_row(vec![Cell::new("Id"), Cell::new(session.id.to_string())]);
    table.add_row(vec![
        Cell::new("End"),
        Cell::new(format_dt_local(session.end_ts)),
    ]);
    if let Some(start) = session.start_ts {
        table.add_row(vec![Cell::new("Start"), Cell::new(format_dt_local(start))]);
    }
    if tool == ToolKind::ClaudeCode
        && let Some(is_sidechain) = session.is_sidechain
    {
        table.add_row(vec![
            Cell::new("Sidechain"),
            Cell::new(if is_sidechain { "yes" } else { "no" }),
        ]);
    }

    let cwd = display_path(&session.cwd, &query.cwd, options.verbose_paths);
    table.add_row(vec![Cell::new("Cwd"), Cell::new(sanitize_cell_text(&cwd))]);
    if let Some(title) = &session.title {
        table.add_row(vec![
            Cell::new("Title"),
            Cell::new(sanitize_cell_text(title)),
        ]);
    }

    table.add_row(vec![
        Cell::new("Tokens (known total)"),
        Cell::new(format_opt_u64_count(session.token_usage.total)),
    ]);
    table.add_row(vec![
        Cell::new("Tokens (known input)"),
        Cell::new(format_opt_u64_count(session.token_usage.input)),
    ]);
    table.add_row(vec![
        Cell::new("Tokens (known output)"),
        Cell::new(format_opt_u64_count(session.token_usage.output)),
    ]);
    table.add_row(vec![
        Cell::new("Tokens (known cache)"),
        Cell::new(format_opt_u64_count(session.token_usage.cache)),
    ]);
    table.add_row(vec![
        Cell::new("Tokens (known reasoning)"),
        Cell::new(format_opt_u64_count(session.token_usage.reasoning)),
    ]);

    table
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
    use crate::usage_stats::query::{ColorMode, GroupBy, TimeRange};
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
                sidechain_totals: None,
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
                color: ColorMode::Auto,
            },
        );
        assert!(table.contains("t1"));
        assert!(table.contains("sessions"));
        assert!(!table.contains('\t'));
    }

    #[test]
    fn ansi_policy_auto_requires_tty_and_no_color_unset() {
        assert!(!should_enable_ansi(ColorMode::Auto, false, false));
        assert!(!should_enable_ansi(ColorMode::Auto, true, true));
        assert!(should_enable_ansi(ColorMode::Auto, true, false));
    }

    #[test]
    fn ansi_policy_overrides() {
        assert!(should_enable_ansi(ColorMode::Always, false, true));
        assert!(should_enable_ansi(ColorMode::Always, false, false));
        assert!(!should_enable_ansi(ColorMode::Never, true, false));
        assert!(!should_enable_ansi(ColorMode::Never, true, true));
    }

    #[test]
    fn table_sanitizes_cell_content() {
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
            title: Some("hello\tworld\nagain".to_string()),
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
                color: ColorMode::Never,
            },
        );

        assert!(!table.contains('\t'));
        assert!(table.contains("hello world again"));
    }
}
