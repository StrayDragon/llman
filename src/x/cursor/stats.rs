use crate::usage_stats::tui::{
    ScanFn, ScanMessage, ScanProgress, StatsTuiScanRequest, run_stats_tui,
};
use crate::usage_stats::{
    OutputFormat, RenderOptions, SessionId, SessionRecord, StatsCliArgs, StatsJsonOutput,
    StatsQuery, StatsViewResult, ToolKind, ViewKind, build_session_detail_view,
    build_sessions_view, build_summary_view, build_trend_view, filter_sessions_v1,
    parse_time_range, render_stats_json, render_stats_table, validate_stats_cli_args,
};
use crate::x::cursor::database::CursorDatabase;
use crate::x::cursor::models::ComposerData;
use anyhow::{Context, Result, bail};
use chrono::{DateTime, TimeZone, Utc};
use clap::Args;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sqlite::SqliteConnection;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

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
    let cwd = std::env::current_dir().context("resolve current directory")?;
    let workspace_db = resolve_workspace_db_path(args.db_path.as_deref(), &cwd)?;
    let global_db = resolve_global_db_path(args.global_db_path.as_deref())?;

    if args.stats.tui {
        let initial = StatsTuiScanRequest {
            cwd: cwd.clone(),
            group_by: args.stats.group_by,
            range: args.stats.range.clone(),
            limit: args.stats.limit,
            verbose_paths: args.stats.verbose,
            with_breakdown: false,
            include_sidechain: true,
        };

        let scan_workspace_db = workspace_db.clone();
        let scan_global_db = global_db.clone();
        let scan_fn: ScanFn = Arc::new(move |request, tx| {
            let tx_progress = tx.clone();
            let mut report = move |progress: CursorScanProgress| {
                let CursorScanProgress::Composers { done, total } = progress;
                let _ = tx_progress.send(ScanMessage::Progress(ScanProgress {
                    label: "Scanning composers",
                    done,
                    total,
                }));
            };

            let sessions = load_cursor_sessions(
                &scan_workspace_db,
                &scan_global_db,
                &request.cwd,
                Some(&mut report),
            )?;

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

        return run_stats_tui(ToolKind::Cursor, initial, scan_fn);
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
        cwd: cwd.clone(),
        limit: args.stats.limit,
        id,
    };

    let sessions = load_cursor_sessions(&workspace_db, &global_db, &cwd, None)?;
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
        tool: ToolKind::Cursor,
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

fn resolve_workspace_db_path(
    override_path: Option<&std::path::Path>,
    cwd: &std::path::Path,
) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }

    let workspaces = CursorDatabase::find_all_workspaces().context("scan Cursor workspaces")?;
    let Some(workspace) = workspaces
        .into_iter()
        .find(|w| w.project_path.as_deref() == Some(cwd))
    else {
        bail!("no Cursor workspace found for current directory; use --db-path");
    };

    Ok(workspace.db_path)
}

fn resolve_global_db_path(override_path: Option<&std::path::Path>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    CursorDatabase::get_global_db_path().context("resolve Cursor global state db")
}

#[derive(Debug, Clone, Copy)]
enum CursorScanProgress {
    Composers { done: usize, total: usize },
}

fn load_cursor_sessions(
    workspace_db: &std::path::Path,
    global_db: &std::path::Path,
    cwd: &std::path::Path,
    mut progress: Option<&mut dyn FnMut(CursorScanProgress)>,
) -> Result<Vec<SessionRecord>> {
    let composer_data = read_composer_data(workspace_db)?;
    let mut global_conn = connect_sqlite_ro(global_db)
        .with_context(|| format!("open Cursor global db: {}", global_db.display()))?;

    let total = composer_data.all_composers.len();
    let mut done = 0usize;

    let mut sessions = Vec::with_capacity(total);
    for composer in composer_data.all_composers {
        done = done.saturating_add(1);
        if let Some(progress) = progress.as_deref_mut() {
            progress(CursorScanProgress::Composers { done, total });
        }

        let (token_usage, start_ts, end_ts) = read_composer_usage(
            &mut global_conn,
            &composer.composer_id,
            composer.created_at,
            composer.last_updated_at,
        )?;

        let Some(end_ts) = end_ts else {
            continue;
        };

        sessions.push(SessionRecord {
            tool: ToolKind::Cursor,
            id: SessionId(format!("composer:{}", composer.composer_id)),
            cwd: cwd.to_path_buf(),
            title: composer.name.and_then(|s| {
                let trimmed = s.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }),
            start_ts,
            end_ts,
            token_usage,
            is_sidechain: None,
        });
    }

    Ok(sessions)
}

fn connect_sqlite_ro(path: &std::path::Path) -> Result<SqliteConnection> {
    let database_url = format!("file:{}?mode=ro", path.display());
    SqliteConnection::establish(&database_url)
        .with_context(|| format!("open sqlite read-only: {}", path.display()))
}

#[derive(QueryableByName, Debug)]
struct ItemRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Binary>)]
    value: Option<Vec<u8>>,
}

fn read_composer_data(workspace_db: &std::path::Path) -> Result<ComposerData> {
    let mut conn = connect_sqlite_ro(workspace_db)
        .with_context(|| format!("open Cursor workspace db: {}", workspace_db.display()))?;

    let rows: Vec<ItemRow> =
        sql_query("SELECT value FROM ItemTable WHERE key = 'composer.composerData' LIMIT 1")
            .load(&mut conn)
            .context("query composer.composerData")?;

    let Some(raw) = rows.into_iter().next().and_then(|row| row.value) else {
        return Ok(ComposerData {
            all_composers: vec![],
            selected_composer_ids: None,
        });
    };

    serde_json::from_slice(&raw).context("parse composer.composerData json")
}

#[derive(QueryableByName, Debug)]
struct BubbleRow {
    #[allow(dead_code)]
    #[diesel(sql_type = diesel::sql_types::Integer)]
    rowid: i32,
    #[allow(dead_code)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    key: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Binary>)]
    value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Deserialize)]
struct CursorBubbleValue {
    #[serde(rename = "createdAt", default)]
    created_at: Option<CursorCreatedAt>,
    #[serde(rename = "tokenCount", default)]
    token_count: Option<CursorTokenCount>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum CursorCreatedAt {
    Rfc3339(String),
    EpochMs(i64),
}

#[derive(Debug, Clone, Deserialize)]
struct CursorTokenCount {
    #[serde(rename = "inputTokens", default)]
    input_tokens: Option<u64>,
    #[serde(rename = "outputTokens", default)]
    output_tokens: Option<u64>,
}

#[derive(Default)]
struct TokenAccum {
    input_any: bool,
    output_any: bool,
    input_sum: u64,
    output_sum: u64,
}

impl TokenAccum {
    fn add(&mut self, token_count: &CursorTokenCount) {
        if let Some(v) = token_count.input_tokens {
            self.input_any = true;
            self.input_sum = self.input_sum.saturating_add(v);
        }
        if let Some(v) = token_count.output_tokens {
            self.output_any = true;
            self.output_sum = self.output_sum.saturating_add(v);
        }
    }

    fn build(self) -> crate::usage_stats::TokenUsage {
        let input = self.input_any.then_some(self.input_sum);
        let output = self.output_any.then_some(self.output_sum);

        let total = match (input, output) {
            (Some(a), Some(b)) => Some(a.saturating_add(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        crate::usage_stats::TokenUsage {
            total,
            input,
            output,
            cache: None,
            reasoning: None,
        }
    }
}

type CursorComposerUsage = (
    crate::usage_stats::TokenUsage,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

fn read_composer_usage(
    conn: &mut SqliteConnection,
    composer_id: &str,
    composer_created_at_ms: i64,
    composer_last_updated_at_ms: Option<i64>,
) -> Result<CursorComposerUsage> {
    let pattern = format!("bubbleId:{composer_id}:%");
    let rows: Vec<BubbleRow> =
        sql_query("SELECT rowid, key, value FROM cursorDiskKV WHERE key LIKE ?1 ORDER BY rowid")
            .bind::<diesel::sql_types::Text, _>(&pattern)
            .load(conn)
            .with_context(|| format!("query cursorDiskKV bubbles for composer: {composer_id}"))?;

    let mut tokens = TokenAccum::default();
    let mut min_ts: Option<DateTime<Utc>> = None;
    let mut max_ts: Option<DateTime<Utc>> = None;

    for row in rows {
        let Some(raw) = row.value else {
            continue;
        };
        let Ok(bubble) = serde_json::from_slice::<CursorBubbleValue>(&raw) else {
            continue;
        };

        if let Some(token_count) = &bubble.token_count {
            tokens.add(token_count);
        }

        if let Some(created_at) = &bubble.created_at
            && let Some(ts) = parse_cursor_created_at(created_at)
        {
            min_ts = Some(min_ts.map_or(ts, |current| current.min(ts)));
            max_ts = Some(max_ts.map_or(ts, |current| current.max(ts)));
        }
    }

    let fallback_start = parse_epoch_ms_utc(composer_created_at_ms);
    let fallback_end = composer_last_updated_at_ms
        .and_then(parse_epoch_ms_utc)
        .or_else(|| parse_epoch_ms_utc(composer_created_at_ms));

    let start_ts = min_ts.or(fallback_start);
    let end_ts = max_ts.or(fallback_end);

    Ok((tokens.build(), start_ts, end_ts))
}

fn parse_cursor_created_at(raw: &CursorCreatedAt) -> Option<DateTime<Utc>> {
    match raw {
        CursorCreatedAt::Rfc3339(s) => DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc)),
        CursorCreatedAt::EpochMs(ms) => parse_epoch_ms_utc(*ms),
    }
}

fn parse_epoch_ms_utc(ms: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms).single()
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::Connection;
    use diesel::RunQueryDsl;
    use tempfile::TempDir;

    fn create_workspace_db(path: &std::path::Path, composer_data_json: &str) {
        let mut conn =
            SqliteConnection::establish(&path.to_string_lossy()).expect("establish sqlite");
        diesel::sql_query("CREATE TABLE ItemTable (key TEXT PRIMARY KEY, value BLOB);")
            .execute(&mut conn)
            .expect("create ItemTable");
        diesel::sql_query("INSERT INTO ItemTable (key, value) VALUES (?1, ?2);")
            .bind::<diesel::sql_types::Text, _>("composer.composerData")
            .bind::<diesel::sql_types::Binary, _>(composer_data_json.as_bytes().to_vec())
            .execute(&mut conn)
            .expect("insert composerData");
    }

    fn create_global_db(path: &std::path::Path, rows: Vec<(&str, &str)>) {
        let mut conn =
            SqliteConnection::establish(&path.to_string_lossy()).expect("establish sqlite");
        diesel::sql_query(
            "CREATE TABLE cursorDiskKV (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);",
        )
        .execute(&mut conn)
        .expect("create cursorDiskKV");
        for (key, json) in rows {
            diesel::sql_query("INSERT INTO cursorDiskKV (key, value) VALUES (?1, ?2);")
                .bind::<diesel::sql_types::Text, _>(key)
                .bind::<diesel::sql_types::Binary, _>(json.as_bytes().to_vec())
                .execute(&mut conn)
                .expect("insert bubble");
        }
    }

    #[test]
    fn load_cursor_sessions_parses_bubble_created_at_and_token_count() {
        let temp = TempDir::new().expect("temp dir");
        let workspace_db = temp.path().join("state.vscdb");
        let global_db = temp.path().join("global.vscdb");

        let composer_json = r#"{
  "allComposers": [
    { "composerId": "c1", "createdAt": 1700000000000, "lastUpdatedAt": 1700000002000, "unifiedMode": "agent", "name": "One" }
  ]
}"#;
        create_workspace_db(&workspace_db, composer_json);

        create_global_db(
            &global_db,
            vec![
                (
                    "bubbleId:c1:b1",
                    r#"{"createdAt":"2026-02-01T00:00:00.000Z","tokenCount":{"inputTokens":10,"outputTokens":5}}"#,
                ),
                (
                    "bubbleId:c1:b2",
                    r#"{"createdAt":1760000000000,"tokenCount":{"inputTokens":3,"outputTokens":4}}"#,
                ),
                ("bubbleId:c1:b3", r#"{"createdAt":"bad","tokenCount":{}}"#),
            ],
        );

        let cwd = std::path::PathBuf::from("/p");
        let sessions =
            load_cursor_sessions(&workspace_db, &global_db, &cwd, None).expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id.0, "composer:c1");
        assert_eq!(sessions[0].token_usage.total, Some(22));
        assert!(sessions[0].start_ts.is_some());
    }

    #[test]
    fn load_cursor_sessions_handles_empty_bubbles_with_fallback_timestamps() {
        let temp = TempDir::new().expect("temp dir");
        let workspace_db = temp.path().join("state.vscdb");
        let global_db = temp.path().join("global.vscdb");

        let composer_json = r#"{
  "allComposers": [
    { "composerId": "c1", "createdAt": 1700000000000, "unifiedMode": "agent", "name": "One" }
  ]
}"#;
        create_workspace_db(&workspace_db, composer_json);
        create_global_db(&global_db, vec![]);

        let cwd = std::path::PathBuf::from("/p");
        let sessions =
            load_cursor_sessions(&workspace_db, &global_db, &cwd, None).expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].token_usage.total, None);
        assert!(sessions[0].end_ts.timestamp_millis() > 0);
    }
}
