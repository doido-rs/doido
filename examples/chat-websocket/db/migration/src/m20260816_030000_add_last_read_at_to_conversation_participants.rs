use doido::model::migration::add_column;
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260816_030000_add_last_read_at_to_conversation_participants"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_column(
            manager,
            "conversation_participants",
            "last_read_at",
            |c| {
                c.timestamp();
            },
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("conversation_participants"))
                    .drop_column(Alias::new("last_read_at"))
                    .to_owned(),
            )
            .await
    }
}
