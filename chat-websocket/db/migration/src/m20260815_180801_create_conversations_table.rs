use doido::model::migration::{create_table, drop_table};
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260815_180801_create_conversations_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // `create_table` adds an auto-incrementing `id` primary key for you.
        // Add columns with the builder, e.g. `t.string("name").not_null();`.
        create_table(manager, "conversations", |_t| {}).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_table(manager, "conversations").await
    }
}
