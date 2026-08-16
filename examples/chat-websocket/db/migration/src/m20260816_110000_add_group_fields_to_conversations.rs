use doido::model::migration::add_column;
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260816_110000_add_group_fields_to_conversations"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_column(manager, "conversations", "kind", |c| {
            c.string().not_null().default("direct");
        })
        .await?;
        add_column(manager, "conversations", "name", |c| {
            c.string();
        })
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("conversations"))
                    .drop_column(Alias::new("name"))
                    .drop_column(Alias::new("kind"))
                    .to_owned(),
            )
            .await
    }
}
