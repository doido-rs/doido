use doido::model::migration::drop_table;
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260101000001_create_doido_jobs_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // `doido_jobs` uses a TEXT primary key (job id), not the implicit bigint `id`
        // that `create_table` adds, so the schema is emitted as raw SQL.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS doido_jobs (
                    id TEXT PRIMARY KEY,
                    queue TEXT NOT NULL,
                    status TEXT NOT NULL,
                    priority INTEGER NOT NULL DEFAULT 0,
                    run_at BIGINT NOT NULL,
                    locked_at BIGINT,
                    data TEXT NOT NULL
                )",
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_doido_jobs_reserve
                    ON doido_jobs (queue, status, run_at)",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_doido_jobs_reserve")
            .await?;
        drop_table(manager, "doido_jobs").await
    }
}
