use sea_orm::{Database, DatabaseConnection};
use doido_core::Result;

pub struct TestDb {
    conn: DatabaseConnection,
}

impl TestDb {
    pub async fn new() -> Result<Self> {
        let conn = Database::connect("sqlite::memory:").await
            .map_err(|e| doido_core::anyhow::anyhow!("TestDb connect failed: {e}"))?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }
}
