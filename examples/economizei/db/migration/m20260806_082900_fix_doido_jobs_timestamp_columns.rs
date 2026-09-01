use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260806_082900_fix_doido_jobs_timestamp_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE doido_jobs ALTER COLUMN run_at TYPE BIGINT;
                ALTER TABLE doido_jobs ALTER COLUMN locked_at TYPE BIGINT;
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE doido_jobs ALTER COLUMN run_at TYPE INTEGER;
                ALTER TABLE doido_jobs ALTER COLUMN locked_at TYPE INTEGER;
                "#,
            )
            .await?;

        Ok(())
    }
}
