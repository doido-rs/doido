use doido::model::migration::{add_index, create_table, drop_table, remove_index};
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
        create_table(manager, "doido_jobs", |t| {
            t.string("job_id").not_null().unique_key();
            t.string("queue").not_null();
            t.string("status").not_null();
            t.big_integer("priority");
            t.big_integer("run_at").not_null();
            t.big_integer("locked_at");
            t.text("data").not_null();
        })
        .await?;
        add_index(
            manager,
            "doido_jobs",
            &["queue", "status", "run_at"],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        remove_index(
            manager,
            "doido_jobs",
            &["queue", "status", "run_at"],
        )
        .await?;
        drop_table(manager, "doido_jobs").await
    }
}
