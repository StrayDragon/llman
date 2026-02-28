use diesel::prelude::*;
use diesel::sql_query;
use diesel::sqlite::SqliteConnection;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn llman_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_llman"))
}

fn run_llman(args: &[&str], work_dir: &Path, config_dir: &Path) -> Output {
    Command::new(llman_bin())
        .args(["--config-dir", config_dir.to_str().expect("config dir")])
        .args(args)
        .current_dir(work_dir)
        .output()
        .expect("run llman")
}

fn create_state_db(path: &std::path::Path) {
    let database_url = path.to_string_lossy().to_string();
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
    .expect("create threads table");
}

fn insert_thread(
    conn: &mut SqliteConnection,
    id: &str,
    cwd: &str,
    tokens_used: i64,
    rollout_path: Option<&str>,
) {
    let rollout_path = rollout_path.map(|s| s.to_string());
    sql_query(
        "INSERT INTO threads (id, rollout_path, created_at, updated_at, cwd, title, tokens_used) VALUES (?1, ?2, 1, 2, ?3, 't', ?4);",
    )
    .bind::<diesel::sql_types::Text, _>(id)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(rollout_path)
    .bind::<diesel::sql_types::Text, _>(cwd)
    .bind::<diesel::sql_types::BigInt, _>(tokens_used)
    .execute(conn)
    .expect("insert thread");
}

#[test]
fn codex_stats_summary_json_filters_by_cwd() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir).expect("mkdir work");
    let work_dir = fs::canonicalize(&work_dir).expect("canonicalize work");
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).expect("mkdir config");

    let db_path = temp.path().join("state_1.sqlite");
    create_state_db(&db_path);

    {
        let database_url = db_path.to_string_lossy().to_string();
        let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");
        insert_thread(&mut conn, "t1", &work_dir.to_string_lossy(), 9, None);
        insert_thread(&mut conn, "t2", "/other", 100, None);
    }

    let output = run_llman(
        &[
            "x",
            "codex",
            "stats",
            "--state-db",
            db_path.to_str().unwrap(),
            "--format",
            "json",
        ],
        &work_dir,
        &config_dir,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let v: Value = serde_json::from_slice(&output.stdout).expect("parse json");
    assert_eq!(v["tool"], "codex");
    assert_eq!(v["view"], "summary");
    assert_eq!(v["result"]["coverage"]["total_sessions"], 1);
    assert_eq!(v["result"]["totals"]["tokens_total_known"], 9);
}

#[test]
fn codex_stats_session_json_with_breakdown() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir).expect("mkdir work");
    let work_dir = fs::canonicalize(&work_dir).expect("canonicalize work");
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).expect("mkdir config");

    let rollout = temp.path().join("rollout.jsonl");
    fs::write(
        &rollout,
        r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1,"cached_input_tokens":2,"output_tokens":3,"reasoning_output_tokens":4,"total_tokens":10}}}}"#,
    )
    .expect("write rollout");

    let db_path = temp.path().join("state_1.sqlite");
    create_state_db(&db_path);

    {
        let database_url = db_path.to_string_lossy().to_string();
        let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");
        insert_thread(
            &mut conn,
            "t1",
            &work_dir.to_string_lossy(),
            9,
            Some(&rollout.to_string_lossy()),
        );
    }

    let output = run_llman(
        &[
            "x",
            "codex",
            "stats",
            "--state-db",
            db_path.to_str().unwrap(),
            "--with-breakdown",
            "--view",
            "session",
            "--id",
            "t1",
            "--format",
            "json",
        ],
        &work_dir,
        &config_dir,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let v: Value = serde_json::from_slice(&output.stdout).expect("parse json");
    assert_eq!(v["view"], "session");
    assert_eq!(v["result"]["session"]["id"], "t1");
    assert_eq!(v["result"]["session"]["token_usage"]["total"], 9);
    assert_eq!(v["result"]["session"]["token_usage"]["input"], 1);
    assert_eq!(v["result"]["session"]["token_usage"]["cache"], 2);
    assert_eq!(v["result"]["session"]["token_usage"]["output"], 3);
    assert_eq!(v["result"]["session"]["token_usage"]["reasoning"], 4);
}

#[test]
fn codex_stats_table_color_auto_has_no_ansi_when_captured() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir).expect("mkdir work");
    let work_dir = fs::canonicalize(&work_dir).expect("canonicalize work");
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).expect("mkdir config");

    let db_path = temp.path().join("state_1.sqlite");
    create_state_db(&db_path);

    {
        let database_url = db_path.to_string_lossy().to_string();
        let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");
        insert_thread(&mut conn, "t1", &work_dir.to_string_lossy(), 9, None);
    }

    let output = run_llman(
        &[
            "x",
            "codex",
            "stats",
            "--state-db",
            db_path.to_str().unwrap(),
            "--format",
            "table",
            "--color",
            "auto",
        ],
        &work_dir,
        &config_dir,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stdout.windows(2).any(|w| w == b"\x1b["),
        "expected no ANSI escape sequences in captured output, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn codex_stats_table_color_always_includes_ansi_when_captured() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir).expect("mkdir work");
    let work_dir = fs::canonicalize(&work_dir).expect("canonicalize work");
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).expect("mkdir config");

    let db_path = temp.path().join("state_1.sqlite");
    create_state_db(&db_path);

    {
        let database_url = db_path.to_string_lossy().to_string();
        let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");
        insert_thread(&mut conn, "t1", &work_dir.to_string_lossy(), 9, None);
    }

    let output = run_llman(
        &[
            "x",
            "codex",
            "stats",
            "--state-db",
            db_path.to_str().unwrap(),
            "--format",
            "table",
            "--color",
            "always",
        ],
        &work_dir,
        &config_dir,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.windows(2).any(|w| w == b"\x1b["),
        "expected ANSI escape sequences with --color always, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn codex_stats_json_is_never_colored() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir).expect("mkdir work");
    let work_dir = fs::canonicalize(&work_dir).expect("canonicalize work");
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).expect("mkdir config");

    let db_path = temp.path().join("state_1.sqlite");
    create_state_db(&db_path);

    {
        let database_url = db_path.to_string_lossy().to_string();
        let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");
        insert_thread(&mut conn, "t1", &work_dir.to_string_lossy(), 9, None);
    }

    let output = run_llman(
        &[
            "x",
            "codex",
            "stats",
            "--state-db",
            db_path.to_str().unwrap(),
            "--format",
            "json",
            "--color",
            "always",
        ],
        &work_dir,
        &config_dir,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stdout.windows(2).any(|w| w == b"\x1b["),
        "expected no ANSI escape sequences in json output, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let v: Value = serde_json::from_slice(&output.stdout).expect("parse json");
    assert_eq!(v["tool"], "codex");
}

#[test]
#[cfg(unix)]
fn codex_stats_table_no_color_env_disables_ansi_in_tty_auto_mode() {
    use expectrl::{Session, WaitStatus};
    use std::io::Read;

    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir).expect("mkdir work");
    let work_dir = fs::canonicalize(&work_dir).expect("canonicalize work");
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).expect("mkdir config");

    let db_path = temp.path().join("state_1.sqlite");
    create_state_db(&db_path);

    {
        let database_url = db_path.to_string_lossy().to_string();
        let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");
        insert_thread(&mut conn, "t1", &work_dir.to_string_lossy(), 9, None);
    }

    let mut cmd = Command::new(llman_bin());
    cmd.env("NO_COLOR", "1")
        .args(["--config-dir", config_dir.to_str().expect("config dir")])
        .args([
            "x",
            "codex",
            "stats",
            "--state-db",
            db_path.to_str().unwrap(),
            "--format",
            "table",
            "--color",
            "auto",
        ])
        .current_dir(&work_dir);

    let mut session = Session::spawn(cmd).expect("spawn llman in pty");
    let mut buf = Vec::new();
    session.read_to_end(&mut buf).expect("read stdout");
    assert_eq!(
        session.wait().expect("wait"),
        WaitStatus::Exited(session.pid(), 0)
    );
    assert!(
        !buf.windows(2).any(|w| w == b"\x1b["),
        "expected no ANSI escape sequences with NO_COLOR=1 in tty auto mode, got:\n{}",
        String::from_utf8_lossy(&buf)
    );
}
