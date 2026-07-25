//! Migration rollback helpers (Rails `db:rollback STEP=n` / `db:migrate:redo`)
//! over sea-orm's `MigratorTrait`.

use crate::sea_orm::DatabaseConnection;
use crate::sea_orm_migration::MigratorTrait;
use doido_core::Result;

/// Roll back the last `steps` migrations (Rails `db:rollback STEP=steps`).
pub async fn rollback<M: MigratorTrait>(conn: &DatabaseConnection, steps: u32) -> Result<()> {
    M::down(conn, Some(steps))
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("rollback failed: {e}"))
}

/// Roll back then re-apply the last `steps` migrations (Rails `db:migrate:redo`).
pub async fn redo<M: MigratorTrait>(conn: &DatabaseConnection, steps: u32) -> Result<()> {
    M::down(conn, Some(steps))
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("redo down failed: {e}"))?;
    M::up(conn, Some(steps))
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("redo up failed: {e}"))
}
