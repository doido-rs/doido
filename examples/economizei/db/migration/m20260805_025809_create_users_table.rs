use doido::model::migration::{create_table, drop_table};
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260805_025809_create_users_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_table(manager, "users", |t| {
            t.string("email").not_null().unique_key();
            t.string("password_digest").not_null();
            t.timestamp("created_at").not_null();
            t.timestamp("updated_at").not_null();
        })
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_table(manager, "users").await
    }
}
