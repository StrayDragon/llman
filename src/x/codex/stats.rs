use crate::usage_stats::tui::{
    ScanFn, ScanMessage, ScanProgress, StatsTuiScanRequest, run_stats_tui,
};
use crate::usage_stats::{
    OutputFormat, RenderOptions, SessionId, SessionRecord, StatsCliArgs, StatsJsonOutput,
    StatsQuery, StatsViewResult, ToolKind, ViewKind, build_session_detail_view,
    build_sessions_view, build_summary_view, build_trend_view, filter_sessions_v1,
    parse_time_range, render_stats_json, render_stats_table, validate_stats_cli_args,
};
use anyhow::{Context, Result, bail};
use chrono::{TimeZone, Utc};
use clap::Args;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sqlite::SqliteConnection;
use glob::glob;
use serde::Deserialize;
use std::cmp::Reverse;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Args, Debug, Clone)]
pub struct CodexStatsArgs {
    #[command(flatten)]
    pub stats: StatsCliArgs,

    /// Override the Codex state sqlite path (default: ~/.codex/state_*.sqlite highest).
    #[arg(long)]
    pub state_db: Option<PathBuf>,

    /// Parse rollout JSONL to populate token breakdown (slower; read-only).
    #[arg(long)]
    pub with_breakdown: bool,
}

pub fn run_stats(args: &CodexStatsArgs) -> Result<()> {
    validate_stats_cli_args(&args.stats)?;
    let cwd = std::env::current_dir().context("resolve current directory")?;
    let state_db = resolve_state_db_path(args.state_db.as_deref())?;

    if args.stats.tui {
        let initial = StatsTuiScanRequest {
            cwd: cwd.clone(),
            group_by: args.stats.group_by,
            range: args.stats.range.clone(),
            limit: args.stats.limit,
            verbose_paths: args.stats.verbose,
            with_breakdown: args.with_breakdown,
            include_sidechain: true,
        };

        let scan_state_db = state_db.clone();
        let scan_fn: ScanFn = Arc::new(move |request, tx| {
            let tx_progress = tx.clone();
            let mut report = move |progress: CodexScanProgress| {
                let CodexScanProgress::Breakdown { done, total } = progress;
                let _ = tx_progress.send(ScanMessage::Progress(ScanProgress {
                    label: "Parsing rollouts",
                    done,
                    total,
                }));
            };

            let sessions =
                load_codex_sessions(&scan_state_db, request.with_breakdown, Some(&mut report))?;

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

        return run_stats_tui(ToolKind::Codex, initial, scan_fn);
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

    let sessions = load_codex_sessions(&state_db, args.with_breakdown, None)?;
    let sessions = filter_sessions_v1(&sessions, &query);

    let view = match query.view {
        ViewKind::Summary => StatsViewResult::Summary(build_summary_view(&sessions)),
        ViewKind::Trend => StatsViewResult::Trend(build_trend_view(&sessions, query.group_by)),
        ViewKind::Sessions => {
            StatsViewResult::Sessions(build_sessions_view(&sessions, query.limit))
        }
        ViewKind::Session => {
            let id = query.id.as_ref().expect("validated");
            let Some(view) = build_session_detail_view(&sessions, id) else {
                bail!("session not found for id: {id}");
            };
            StatsViewResult::Session(view)
        }
    };

    let output = StatsJsonOutput {
        tool: ToolKind::Codex,
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
                    color: args.stats.color,
                },
            );
            print!("{table}");
        }
    }

    Ok(())
}

fn resolve_state_db_path(override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }

    let home = crate::config::home_dir().context("resolve home directory")?;
    let codex_dir = home.join(".codex");
    let pattern = codex_dir.join("state_*.sqlite");
    let matches: Vec<PathBuf> = glob(&pattern.to_string_lossy())?
        .filter_map(|entry| entry.ok())
        .collect();

    if matches.is_empty() {
        bail!("no Codex state db found under {}", codex_dir.display());
    }

    Ok(select_highest_state_db(matches))
}

fn select_highest_state_db(mut candidates: Vec<PathBuf>) -> PathBuf {
    // Prefer numeric suffixes: state_<N>.sqlite (highest N).
    let mut numeric: Vec<(u64, PathBuf)> = Vec::new();
    let mut other: Vec<PathBuf> = Vec::new();

    for path in candidates.drain(..) {
        let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
            other.push(path);
            continue;
        };
        let Some(stem) = file_name
            .strip_prefix("state_")
            .and_then(|s| s.strip_suffix(".sqlite"))
        else {
            other.push(path);
            continue;
        };
        if let Ok(n) = stem.parse::<u64>() {
            numeric.push((n, path));
        } else {
            other.push(path);
        }
    }

    if !numeric.is_empty() {
        numeric.sort_by_key(|(n, _)| Reverse(*n));
        return numeric[0].1.clone();
    }

    other.sort();
    other.last().cloned().expect("non-empty candidates")
}

#[derive(Debug, Clone, Copy)]
enum CodexScanProgress {
    Breakdown { done: usize, total: usize },
}

fn load_codex_sessions(
    state_db: &Path,
    with_breakdown: bool,
    mut progress: Option<&mut dyn FnMut(CodexScanProgress)>,
) -> Result<Vec<SessionRecord>> {
    let database_url = format!("file:{}?mode=ro", state_db.display());
    let mut connection = SqliteConnection::establish(&database_url)
        .with_context(|| format!("open Codex state db: {}", state_db.display()))?;

    #[derive(QueryableByName, Debug)]
    struct ThreadRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        id: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        cwd: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        created_at: i64,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        updated_at: i64,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        tokens_used: i64,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        rollout_path: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        title: Option<String>,
    }

    let rows: Vec<ThreadRow> = sql_query(
        "SELECT id, cwd, created_at, updated_at, tokens_used, rollout_path, title FROM threads",
    )
    .load(&mut connection)
    .context("query Codex threads table")?;

    let breakdown_total = if with_breakdown {
        rows.iter()
            .filter(|row| {
                row.rollout_path
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|s| !s.is_empty())
            })
            .count()
    } else {
        0
    };
    let mut breakdown_done = 0usize;

    let mut sessions = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at = Utc.timestamp_opt(row.created_at, 0).single();
        let end_ts = match Utc.timestamp_opt(row.updated_at, 0).single() {
            Some(ts) => ts,
            None => continue,
        };

        let mut record = SessionRecord {
            tool: ToolKind::Codex,
            id: SessionId(row.id),
            cwd: PathBuf::from(row.cwd),
            title: row.title.and_then(|s| {
                let trimmed = s.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }),
            start_ts: created_at,
            end_ts,
            token_usage: crate::usage_stats::TokenUsage {
                total: u64::try_from(row.tokens_used).ok(),
                ..crate::usage_stats::TokenUsage::default()
            },
            is_sidechain: None,
        };

        if with_breakdown
            && let Some(path) = row
                .rollout_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        {
            breakdown_done = breakdown_done.saturating_add(1);
            if let Some(progress) = progress.as_deref_mut() {
                progress(CodexScanProgress::Breakdown {
                    done: breakdown_done,
                    total: breakdown_total,
                });
            }

            if let Ok(breakdown) = parse_rollout_breakdown(Path::new(path)) {
                record.token_usage.input = breakdown.input_tokens;
                record.token_usage.output = breakdown.output_tokens;
                record.token_usage.cache = breakdown.cached_input_tokens;
                record.token_usage.reasoning = breakdown.reasoning_output_tokens;
            }
        }

        sessions.push(record);
    }

    Ok(sessions)
}

#[derive(Debug, Clone, Deserialize)]
struct RolloutEvent {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    payload: Option<RolloutPayload>,
}

#[derive(Debug, Clone, Deserialize)]
struct RolloutPayload {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    info: Option<RolloutInfo>,
}

#[derive(Debug, Clone, Deserialize)]
struct RolloutInfo {
    #[serde(default)]
    total_token_usage: Option<TokenUsageTotals>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct TokenUsageTotals {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    cached_input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    reasoning_output_tokens: Option<u64>,
}

fn parse_rollout_breakdown(path: &Path) -> Result<TokenUsageTotals> {
    let file =
        File::open(path).with_context(|| format!("open rollout jsonl: {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut last_total: Option<TokenUsageTotals> = None;
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        let Ok(event) = serde_json::from_str::<RolloutEvent>(&line) else {
            continue;
        };
        if event.r#type.as_deref() != Some("event_msg") {
            continue;
        }
        let Some(payload) = event.payload else {
            continue;
        };
        if payload.r#type.as_deref() != Some("token_count") {
            continue;
        }
        let Some(info) = payload.info else {
            continue;
        };
        if let Some(totals) = info.total_token_usage {
            last_total = Some(totals);
        }
    }

    last_total.context("no token_count.total_token_usage found in rollout jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::Connection;
    use diesel::RunQueryDsl;
    use tempfile::TempDir;

    #[test]
    fn select_highest_state_db_prefers_numeric_suffix() {
        let p1 = PathBuf::from("/tmp/state_1.sqlite");
        let p2 = PathBuf::from("/tmp/state_12.sqlite");
        let p3 = PathBuf::from("/tmp/state_a.sqlite");
        let picked = select_highest_state_db(vec![p1, p2.clone(), p3]);
        assert_eq!(picked, p2);
    }

    #[test]
    fn rollout_breakdown_uses_last_token_count() {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join("rollout.jsonl");
        std::fs::write(
            &path,
            r#"
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1,"cached_input_tokens":2,"output_tokens":3,"reasoning_output_tokens":4,"total_tokens":10}}}}
malformed
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":20,"output_tokens":30,"reasoning_output_tokens":40,"total_tokens":100}}}}
"#,
        )
        .expect("write jsonl");

        let totals = parse_rollout_breakdown(&path).unwrap();
        assert_eq!(totals.input_tokens, Some(10));
        assert_eq!(totals.cached_input_tokens, Some(20));
        assert_eq!(totals.output_tokens, Some(30));
        assert_eq!(totals.reasoning_output_tokens, Some(40));
    }

    #[test]
    fn load_codex_sessions_supports_optional_breakdown() {
        let temp = TempDir::new().expect("temp dir");
        let db_path = temp.path().join("state_1.sqlite");
        let database_url = db_path.to_string_lossy().to_string();
        let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");

        sql_query(
            r#"
CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    rollout_path TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    cwd TEXT NOT NULL,
    title TEXT,
    tokens_used INTEGER NOT NULL DEFAULT 0
);
"#,
        )
        .execute(&mut conn)
        .expect("create table");

        let rollout = temp.path().join("rollout.jsonl");
        std::fs::write(
            &rollout,
            r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1,"cached_input_tokens":2,"output_tokens":3,"reasoning_output_tokens":4,"total_tokens":10}}}}"#,
        )
        .expect("write jsonl");

        sql_query(
            "INSERT INTO threads (id, rollout_path, created_at, updated_at, cwd, title, tokens_used) VALUES (?1, ?2, 1, 2, ?3, ?4, 9);",
        )
        .bind::<diesel::sql_types::Text, _>("t1")
        .bind::<diesel::sql_types::Text, _>(rollout.to_string_lossy().to_string())
        .bind::<diesel::sql_types::Text, _>("/p")
        .bind::<diesel::sql_types::Text, _>("hello")
        .execute(&mut conn)
        .expect("insert");

        // Without breakdown.
        let sessions = load_codex_sessions(&db_path, false, None).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].token_usage.total, Some(9));
        assert_eq!(sessions[0].token_usage.input, None);

        // With breakdown.
        let sessions = load_codex_sessions(&db_path, true, None).unwrap();
        assert_eq!(sessions[0].token_usage.total, Some(9));
        assert_eq!(sessions[0].token_usage.input, Some(1));
        assert_eq!(sessions[0].token_usage.cache, Some(2));
        assert_eq!(sessions[0].token_usage.output, Some(3));
        assert_eq!(sessions[0].token_usage.reasoning, Some(4));
    }

    #[test]
    fn load_codex_sessions_with_breakdown_ignores_missing_rollout_path() {
        let temp = TempDir::new().expect("temp dir");
        let db_path = temp.path().join("state_1.sqlite");
        let database_url = db_path.to_string_lossy().to_string();
        let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");

        sql_query(
            r#"
CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    rollout_path TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    cwd TEXT NOT NULL,
    title TEXT,
    tokens_used INTEGER NOT NULL DEFAULT 0
);
"#,
        )
        .execute(&mut conn)
        .expect("create table");

        sql_query("INSERT INTO threads (id, created_at, updated_at, cwd, tokens_used) VALUES ('t1', 1, 2, '/p', 9);")
            .execute(&mut conn)
            .expect("insert");

        let sessions = load_codex_sessions(&db_path, true, None).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].token_usage.total, Some(9));
        assert_eq!(sessions[0].token_usage.input, None);
    }

    #[test]
    fn load_codex_sessions_with_breakdown_supports_partial_fields() {
        let temp = TempDir::new().expect("temp dir");
        let db_path = temp.path().join("state_1.sqlite");
        let database_url = db_path.to_string_lossy().to_string();
        let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");

        sql_query(
            r#"
CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    rollout_path TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    cwd TEXT NOT NULL,
    title TEXT,
    tokens_used INTEGER NOT NULL DEFAULT 0
);
"#,
        )
        .execute(&mut conn)
        .expect("create table");

        let rollout = temp.path().join("rollout.jsonl");
        std::fs::write(
            &rollout,
            r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1,"output_tokens":3,"total_tokens":10}}}}"#,
        )
        .expect("write jsonl");

        sql_query(
            "INSERT INTO threads (id, rollout_path, created_at, updated_at, cwd, tokens_used) VALUES (?1, ?2, 1, 2, ?3, 9);",
        )
        .bind::<diesel::sql_types::Text, _>("t1")
        .bind::<diesel::sql_types::Text, _>(rollout.to_string_lossy().to_string())
        .bind::<diesel::sql_types::Text, _>("/p")
        .execute(&mut conn)
        .expect("insert");

        let sessions = load_codex_sessions(&db_path, true, None).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].token_usage.total, Some(9));
        assert_eq!(sessions[0].token_usage.input, Some(1));
        assert_eq!(sessions[0].token_usage.output, Some(3));
        assert_eq!(sessions[0].token_usage.cache, None);
        assert_eq!(sessions[0].token_usage.reasoning, None);
    }
}
