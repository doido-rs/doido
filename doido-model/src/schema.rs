//! Schema dump/load (Rails `db:schema:dump` / `db:schema:load`). `dump` reads a
//! SQLite database's table definitions; `load` replays them into a database.

use crate::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use doido_core::Result;

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
