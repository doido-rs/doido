use doido::model::migration::{create_table, drop_table};
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260805_032538_create_bank_accounts_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // `create_table` adds an auto-incrementing `id` primary key for you.
        create_table(manager, "bank_accounts", |t| {
            t.references("user");
            t.references("bank");
            t.string("agency").not_null();
            t.string("account_number").not_null();
            t.string("cpf_cnpj").not_null();
        })
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_table(manager, "bank_accounts").await
    }
}
