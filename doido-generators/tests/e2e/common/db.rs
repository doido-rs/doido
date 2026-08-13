//! Database setup and migration assertions for generated apps.

use rusqlite::Connection;
use std::path::Path;

use super::cli::{run_app, run_app_capture};

pub fn prepare_database(bin: &Path, app: &Path) {
    run_app(bin, app, &["db", "create"]);
    run_app(bin, app, &["db", "migrate"]);
    assert_all_migrations_applied(bin, app);
}

pub fn run_seed(bin: &Path, app: &Path) {
    run_app(bin, app, &["db", "seed"]);
}

pub fn create_database(bin: &Path, app: &Path) {
    run_app(bin, app, &["db", "create"]);
}

pub fn schema_dump(bin: &Path, app: &Path) {
    run_app(bin, app, &["db", "schema", "dump"]);
}

pub fn schema_load(bin: &Path, app: &Path) {
    run_app(bin, app, &["db", "schema", "load"]);
}

pub fn db_reset(bin: &Path, app: &Path) {
    run_app(bin, app, &["db", "reset"]);
}

pub fn db_prepare(bin: &Path, app: &Path) {
    run_app(bin, app, &["db", "prepare"]);
}

pub fn schema_file(app: &Path) -> std::path::PathBuf {
    app.join("db/schema.sql")
}

pub fn assert_schema_contains(app: &Path, needle: &str) {
    let path = schema_file(app);
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("read {}: {e}", path.display());
    });
    assert!(
        content.contains(needle),
        "expected `{needle}` in {}, got:\n{content}",
        path.display()
    );
}

pub fn remove_sqlite_database(app: &Path) {
    let path = sqlite_path(app);
    if path.is_file() {
        std::fs::remove_file(&path).expect("remove sqlite database file");
    }
}

pub fn exec_sqlite(app: &Path, sql: &str) {
    let conn = open_sqlite(app);
    conn.execute(sql, [])
        .unwrap_or_else(|e| panic!("exec `{sql}`: {e}"));
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

pub fn assert_seed_crate_scaffolded(app: &Path) {
    let cargo = app.join("db/seed/Cargo.toml");
    let main_rs = app.join("db/seed/src/main.rs");
    assert!(
        cargo.is_file(),
        "expected seed crate at {}",
        cargo.display()
    );
    assert!(
        main_rs.is_file(),
        "expected seed main at {}",
        main_rs.display()
    );

    let cargo_content = std::fs::read_to_string(&cargo).expect("read db/seed/Cargo.toml");
    assert!(
        cargo_content.contains("doido ="),
        "seed Cargo.toml must depend on the doido meta crate"
    );
    assert!(
        cargo_content.contains("serde ="),
        "seed Cargo.toml must declare serde for generated app/models"
    );

    let main_content = std::fs::read_to_string(&main_rs).expect("read db/seed/src/main.rs");
    assert!(
        main_content.contains("app/models/mod.rs"),
        "seed main must wire app/models"
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

pub fn assert_row_count(app: &Path, table: &str, expected: i64) {
    let conn = open_sqlite(app);
    let count: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows");
    assert_eq!(
        count, expected,
        "expected {expected} row(s) in `{table}`, found {count}"
    );
}

/// Asserts at least one row in `table` has `column = value` (e.g. a seeded user).
pub fn assert_row_exists(app: &Path, table: &str, column: &str, value: &str) {
    let conn = open_sqlite(app);
    let count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE {column} = ?1"),
            rusqlite::params![value],
            |row| row.get(0),
        )
        .expect("count matching rows");
    assert!(
        count >= 1,
        "expected a row in `{table}` where {column} = {value:?}, found none"
    );
}
