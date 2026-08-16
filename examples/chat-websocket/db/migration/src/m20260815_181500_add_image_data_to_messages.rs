use doido::model::migration::add_column;
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260815_181500_add_image_data_to_messages"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_column(manager, "messages", "image_data", |c| {
            c.binary();
        })
        .await?;
        add_column(manager, "messages", "image_content_type", |c| {
            c.string();
        })
        .await?;
        add_column(manager, "messages", "image_filename", |c| {
            c.string();
        })
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("messages"))
                    .drop_column(Alias::new("image_filename"))
                    .drop_column(Alias::new("image_content_type"))
                    .drop_column(Alias::new("image_data"))
                    .to_owned(),
            )
            .await
    }
}
