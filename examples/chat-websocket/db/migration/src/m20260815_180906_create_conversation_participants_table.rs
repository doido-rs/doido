use doido::model::migration::{add_index, create_table, drop_table};
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260815_180906_create_conversation_participants_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_table(manager, "conversation_participants", |t| {
            t.references("conversation");
            t.references("user");
        })
        .await?;
        add_index(manager, "conversation_participants", &["conversation_id", "user_id"]).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_table(manager, "conversation_participants").await
    }
}
