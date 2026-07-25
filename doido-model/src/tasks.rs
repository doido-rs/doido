//! Database task orchestration (Rails `db:reset`/`db:setup`/`db:prepare`),
//! composing the schema/seed primitives.

use crate::schema;
use crate::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use doido_core::Result;

/// Drop every user table in a SQLite database.
pub async fn drop_all_tables(conn: &DatabaseConnection) -> Result<()> {
    let stmt = Statement::from_string(
        DbBackend::Sqlite,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
    );
    let rows = conn
        .query_all_raw(stmt)
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("list tables failed: {e}"))?;
    for row in rows {
        let name: String = row
            .try_get("", "name")
            .map_err(|e| doido_core::anyhow::anyhow!("table row: {e}"))?;
        conn.execute_unprepared(&format!("DROP TABLE IF EXISTS \"{name}\""))
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("drop {name} failed: {e}"))?;
    }
    Ok(())
}

/// `db:reset` — drop everything, then load `schema`.
pub async fn reset(conn: &DatabaseConnection, schema: &str) -> Result<()> {
    drop_all_tables(conn).await?;
    schema::load(conn, schema).await
}

/// `db:setup` — load `schema`, then run the seeder.
pub async fn setup(
    conn: &DatabaseConnection,
    schema: &str,
    seeder: impl AsyncFnOnce(&DatabaseConnection) -> Result<()>,
) -> Result<()> {
    schema::load(conn, schema).await?;
    seeder(conn).await
}

/// `db:prepare` — load `schema` only if the database has no user tables yet
/// (idempotent).
pub async fn prepare(conn: &DatabaseConnection, schema: &str) -> Result<()> {
    let stmt = Statement::from_string(
        DbBackend::Sqlite,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
    );
    let rows = conn
        .query_all_raw(stmt)
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("list tables failed: {e}"))?;
    if rows.is_empty() {
        schema::load(conn, schema).await?;
    }
    Ok(())
}
