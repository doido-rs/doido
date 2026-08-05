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

pub fn assert_table_absent(app: &Path, table: &str) {
    let conn = open_sqlite(app);
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name = ?1")
        .expect("prepare table query");
    let exists = stmt.exists(rusqlite::params![table]).expect("query table");
    assert!(
        !exists,
        "expected table `{table}` to be absent from sqlite schema"
    );
}

pub fn assert_migration_source_exists(app: &Path, module: &str) {
    let path = app.join("db/migration/src").join(format!("{module}.rs"));
    assert!(
        path.is_file(),
        "expected migration source at {}",
        path.display()
    );
}

pub fn assert_migration_source_absent(app: &Path, module: &str) {
    let path = app.join("db/migration/src").join(format!("{module}.rs"));
    assert!(
        !path.is_file(),
        "unexpected migration source at {}",
        path.display()
    );
}

pub fn assert_lib_registers_migration(app: &Path, module: &str) {
    let lib = std::fs::read_to_string(app.join("db/migration/src/lib.rs"))
        .expect("read db/migration/src/lib.rs");
    assert!(
        lib.contains(module),
        "lib.rs should register migration module `{module}`"
    );
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

/// Declared SQLite column type for `column` (PRAGMA table_info, index 2),
/// lowercased. Used to validate the DB side of the field-type mapping. Panics if
/// the column is absent.
pub fn column_type(app: &Path, table: &str, column: &str) -> String {
    let conn = open_sqlite(app);
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("prepare pragma");
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .expect("read columns")
        .filter_map(Result::ok)
        .collect();
    rows.iter()
        .find(|(name, _)| name == column)
        .map(|(_, ty)| ty.to_lowercase())
        .unwrap_or_else(|| {
            panic!(
                "column `{column}` not found on `{table}`, found: {:?}",
                rows.iter().map(|(n, _)| n).collect::<Vec<_>>()
            )
        })
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
