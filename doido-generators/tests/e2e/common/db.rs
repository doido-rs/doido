//! Database setup and migration assertions for generated apps.

use rusqlite::Connection;
use std::path::Path;

use super::cli::{run_app, run_app_capture};

pub fn prepare_database(bin: &Path, app: &Path) {
    run_app(bin, app, &["db", "create"]);
    run_app(bin, app, &["db", "migrate"]);
    assert_all_migrations_applied(bin, app);
}

pub fn assert_all_migrations_applied(bin: &Path, app: &Path) {
    let output = run_app_capture(bin, app, &["db", "migrate", "status"]);
    assert!(
        output.status.success(),
        "db migrate status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.to_ascii_lowercase().contains("pending"),
        "pending migrations remain:\n{combined}"
    );
}

pub fn sqlite_path(app: &Path) -> std::path::PathBuf {
    app.join("db/development.db")
}

pub fn open_sqlite(app: &Path) -> Connection {
    Connection::open(sqlite_path(app)).expect("open sqlite database")
}

pub fn assert_table_exists(app: &Path, table: &str) {
    let conn = open_sqlite(app);
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name = ?1")
        .expect("prepare table query");
    let exists = stmt.exists(rusqlite::params![table]).expect("query table");
    assert!(exists, "expected table `{table}` in sqlite schema");
}

pub fn assert_column_exists(app: &Path, table: &str, column: &str) {
    let conn = open_sqlite(app);
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("prepare pragma");
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("read columns")
        .filter_map(Result::ok)
        .collect();
    assert!(
        columns.iter().any(|c| c == column),
        "expected column `{column}` on `{table}`, found: {columns:?}"
    );
}

pub fn assert_column_absent(app: &Path, table: &str, column: &str) {
    let conn = open_sqlite(app);
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("prepare pragma");
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("read columns")
        .filter_map(Result::ok)
        .collect();
    assert!(
        !columns.iter().any(|c| c == column),
        "expected column `{column}` to be absent on `{table}`, found: {columns:?}"
    );
}
