use doido::model::migration::alter_table;
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260812_151040_add_description_to_transactions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_table(manager, "transactions", |t| {
            t.add_column("description", |c| { c.text(); });
        })
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_table(manager, "transactions", |t| {
            t.drop_column("description");
        })
        .await
    }
}
