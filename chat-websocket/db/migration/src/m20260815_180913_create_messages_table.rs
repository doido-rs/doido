use doido::model::migration::{create_table, drop_table};
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260815_180913_create_messages_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // `create_table` adds an auto-incrementing `id` primary key for you.
        create_table(manager, "messages", |t| {
            t.text("body");
            t.references("conversation");
            t.references("user");
            t.string("message_type").not_null();
            t.timestamp("created_at").not_null();
        })
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_table(manager, "messages").await
    }
}
