mod common;

use common::{prepare_work_and_config_dirs, run_llman};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sqlite::SqliteConnection;
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

fn create_workspace_db(path: &Path, composer_data_json: &str) {
    let database_url = path.to_string_lossy().to_string();
    let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");

    sql_query("CREATE TABLE ItemTable (key TEXT PRIMARY KEY, value BLOB);")
        .execute(&mut conn)
        .expect("create ItemTable");

    sql_query("INSERT INTO ItemTable (key, value) VALUES (?1, ?2);")
        .bind::<diesel::sql_types::Text, _>("composer.composerData")
        .bind::<diesel::sql_types::Binary, _>(composer_data_json.as_bytes().to_vec())
        .execute(&mut conn)
        .expect("insert composerData");
}

fn create_global_db(path: &Path, rows: Vec<(&str, &str)>) {
    let database_url = path.to_string_lossy().to_string();
    let mut conn = SqliteConnection::establish(&database_url).expect("establish sqlite");

    sql_query("CREATE TABLE cursorDiskKV (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);")
        .execute(&mut conn)
        .expect("create cursorDiskKV");

    for (key, json) in rows {
        sql_query("INSERT INTO cursorDiskKV (key, value) VALUES (?1, ?2);")
            .bind::<diesel::sql_types::Text, _>(key)
            .bind::<diesel::sql_types::Binary, _>(json.as_bytes().to_vec())
            .execute(&mut conn)
            .expect("insert bubble");
    }
}

#[test]
fn cursor_stats_summary_json_aggregates_composer_bubbles() {
    let temp = TempDir::new().expect("temp dir");
    let (work_dir, config_dir) = prepare_work_and_config_dirs(temp.path());

    let workspace_db = temp.path().join("state.vscdb");
    let global_db = temp.path().join("global.vscdb");

    let composer_json = r#"{
  "allComposers": [
    { "composerId": "c1", "createdAt": 1700000000000, "lastUpdatedAt": 1700000002000, "unifiedMode": "agent", "name": "One" },
    { "composerId": "c2", "createdAt": 1600000000000, "unifiedMode": "agent", "name": "Two" }
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

    let output = run_llman(
        &[
            "x",
            "cursor",
            "stats",
            "--db-path",
            workspace_db.to_str().unwrap(),
            "--global-db-path",
            global_db.to_str().unwrap(),
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
    assert_eq!(v["tool"], "cursor");
    assert_eq!(v["view"], "summary");
    assert_eq!(v["result"]["coverage"]["total_sessions"], 2);
    assert_eq!(v["result"]["coverage"]["known_token_sessions"], 1);
    assert_eq!(v["result"]["totals"]["tokens_total_known"], 22);
}

#[test]
fn cursor_stats_session_json_by_id() {
    let temp = TempDir::new().expect("temp dir");
    let (work_dir, config_dir) = prepare_work_and_config_dirs(temp.path());

    let workspace_db = temp.path().join("state.vscdb");
    let global_db = temp.path().join("global.vscdb");

    let composer_json = r#"{
  "allComposers": [
    { "composerId": "c1", "createdAt": 1700000000000, "unifiedMode": "agent", "name": "One" }
  ]
}"#;
    create_workspace_db(&workspace_db, composer_json);
    create_global_db(
        &global_db,
        vec![(
            "bubbleId:c1:b1",
            r#"{"createdAt":"2026-02-01T00:00:00.000Z","tokenCount":{"inputTokens":1,"outputTokens":2}}"#,
        )],
    );

    let output = run_llman(
        &[
            "x",
            "cursor",
            "stats",
            "--db-path",
            workspace_db.to_str().unwrap(),
            "--global-db-path",
            global_db.to_str().unwrap(),
            "--view",
            "session",
            "--id",
            "composer:c1",
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
    assert_eq!(v["result"]["session"]["id"], "composer:c1");
    assert_eq!(v["result"]["session"]["token_usage"]["total"], 3);
    assert_eq!(v["result"]["session"]["token_usage"]["input"], 1);
    assert_eq!(v["result"]["session"]["token_usage"]["output"], 2);
}
