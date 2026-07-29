use crate::sea_orm_migration::MigratorTrait;
use doido_core::Result;
use sea_orm::{Database, DatabaseConnection};

pub struct TestDb {
    conn: DatabaseConnection,
}

impl TestDb {
    pub async fn new() -> Result<Self> {
        let conn = Database::connect("sqlite::memory:")
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("TestDb connect failed: {e}"))?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// Run all of `M`'s migrations against this database (test convenience for
    /// the spec's `testing::run_migrations()`).
    pub async fn run_migrations<M: MigratorTrait>(&self) -> Result<()> {
        M::up(&self.conn, None)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("run_migrations failed: {e}"))
    }

    /// Run a seed routine against this database (test convenience for the spec's
    /// `testing::seed()`). Delegates to [`crate::seeds::run_seeds`].
    pub async fn seed(
        &self,
        seeder: impl AsyncFnOnce(&DatabaseConnection) -> Result<()>,
    ) -> Result<()> {
        crate::seeds::run_seeds(&self.conn, seeder).await
    }
}
