//! Schema dump/load (Rails `db:schema:dump` / `db:schema:load`). `dump` reads a
//! SQLite database's table definitions; `load` replays them into a database.

use crate::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use doido_core::Result;
use std::path::Path;

/// Dump the SQLite schema: the `CREATE TABLE` statements, one per line.
pub async fn dump(conn: &DatabaseConnection) -> Result<String> {
    let stmt = Statement::from_string(
        DbBackend::Sqlite,
        "SELECT sql FROM sqlite_master WHERE type = 'table' \
         AND name NOT LIKE 'sqlite_%' AND sql IS NOT NULL ORDER BY name",
    );
    let rows = conn
        .query_all_raw(stmt)
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("schema dump failed: {e}"))?;
    let mut out = String::new();
    for row in rows {
        let sql: String = row
            .try_get("", "sql")
            .map_err(|e| doido_core::anyhow::anyhow!("schema row: {e}"))?;
        out.push_str(sql.trim());
        out.push_str(";\n");
    }
    Ok(out)
}

/// Dumps the live schema from `conn` and writes it to `path`.
pub async fn dump_to_file(conn: &DatabaseConnection, path: impl AsRef<Path>) -> Result<()> {
    let sql = dump(conn).await?;
    write_file(path, &sql)
}

/// Writes `schema` to `path`.
///
/// Creates parent directories and the file when missing; truncates and replaces
/// the entire file when it already exists (Rails `db:schema:dump` semantics).
pub fn write_file(path: impl AsRef<Path>, schema: &str) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| doido_core::anyhow::anyhow!("create {}: {e}", parent.display()))?;
        }
    }
    std::fs::write(path, schema)
        .map_err(|e| doido_core::anyhow::anyhow!("write {}: {e}", path.display()))
}

/// Load a schema (as produced by [`dump`]) into `conn`, executing each statement.
pub async fn load(conn: &DatabaseConnection, schema: &str) -> Result<()> {
    for statement in schema.split(';') {
        let sql = statement.trim();
        if sql.is_empty() {
            continue;
        }
        conn.execute_unprepared(sql)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("schema load failed on `{sql}`: {e}"))?;
    }
    Ok(())
}
