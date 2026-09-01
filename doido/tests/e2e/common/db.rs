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

pub fn schema_diagram(bin: &Path, app: &Path) {
    run_app(bin, app, &["db", "schema", "diagram"]);
}

pub fn schema_diagram_file(app: &Path) -> std::path::PathBuf {
    app.join("db/schema.html")
}

pub fn parse_schema_design_json(html: &str) -> serde_json::Value {
    let marker = r#"<script type="application/json" id="doido-schema-design">"#;
    let start = html
        .find(marker)
        .unwrap_or_else(|| panic!("embedded schema json marker not found"))
        + marker.len();
    let rest = &html[start..];
    let end = rest.find("</script>").expect("closing script tag");
    serde_json::from_str(&rest[..end]).expect("parse embedded schema json")
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

/// Asserts the in-binary seeder is scaffolded: a `db/seeds.rs` module exposing
/// `run`, wired and registered on the `Doido` builder in `src/main.rs`. (The old
/// standalone `db/seed` crate is gone — the seeder runs from the app binary.)
pub fn assert_seeds_scaffolded(app: &Path) {
    let seeds = app.join("db/seeds.rs");
    assert!(seeds.is_file(), "expected seeder at {}", seeds.display());

    let seeds_content = std::fs::read_to_string(&seeds).expect("read db/seeds.rs");
    assert!(
        seeds_content.contains("pub async fn run(db: &DatabaseConnection)"),
        "db/seeds.rs must expose `pub async fn run(db: &DatabaseConnection)`"
    );

    let main_content = std::fs::read_to_string(app.join("src/main.rs")).expect("read src/main.rs");
    assert!(
        main_content.contains("mod seed;") && main_content.contains(".seeder(seed::run)"),
        "src/main.rs must wire and register the seeder"
    );
    assert!(
        !app.join("db/seed/Cargo.toml").is_file(),
        "the standalone db/seed crate must be gone"
    );
}

pub fn assert_migrator_scaffolded(app: &Path) {
    let mod_rs = app.join("db/migration/mod.rs");
    assert!(
        mod_rs.is_file(),
        "expected migration module at {}",
        mod_rs.display()
    );

    let mod_content = std::fs::read_to_string(&mod_rs).expect("read db/migration/mod.rs");
    assert!(
        mod_content.contains("pub struct Migrator"),
        "db/migration/mod.rs must export `Migrator`"
    );

    let main_content = std::fs::read_to_string(app.join("src/main.rs")).expect("read src/main.rs");
    assert!(
        main_content.contains("mod migration;")
            && main_content.contains(".migrator::<migration::Migrator>()"),
        "src/main.rs must wire and register the migrator"
    );
    assert!(
        !app.join("db/migration/Cargo.toml").is_file(),
        "the standalone db/migration crate must be gone"
    );
}

pub fn assert_migration_source_exists(app: &Path, module: &str) {
    let path = app.join("db/migration").join(format!("{module}.rs"));
    assert!(
        path.is_file(),
        "expected migration source at {}",
        path.display()
    );
}

pub fn assert_migration_source_absent(app: &Path, module: &str) {
    let path = app.join("db/migration").join(format!("{module}.rs"));
    assert!(
        !path.is_file(),
        "unexpected migration source at {}",
        path.display()
    );
}

pub fn assert_mod_registers_migration(app: &Path, module: &str) {
    let mod_rs =
        std::fs::read_to_string(app.join("db/migration/mod.rs")).expect("read db/migration/mod.rs");
    assert!(
        mod_rs.contains(module),
        "mod.rs should register migration module `{module}`"
    );
}

/// Backward-compatible alias for older scenarios.
pub fn assert_lib_registers_migration(app: &Path, module: &str) {
    assert_mod_registers_migration(app, module);
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
