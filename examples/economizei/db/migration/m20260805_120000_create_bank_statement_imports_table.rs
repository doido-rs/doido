use doido::model::migration::{create_table, drop_table};
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260805_120000_create_bank_statement_imports_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_table(manager, "bank_statement_imports", |t| {
            t.references("user");
            t.references("bank_account");
            t.references("company");
            t.string("source").not_null();
            t.string("statement_type").not_null();
            t.string("original_filename").not_null();
            t.binary("compressed_data").not_null();
            t.string("file_checksum").not_null();
            t.big_integer("byte_size").not_null();
            t.integer("transactions_imported").not_null().default(0);
            t.string("status").not_null().default("completed");
            t.text("error_message");
            t.timestamp("created_at").not_null();
            t.timestamp("updated_at").not_null();
        })
        .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_bank_statement_imports_checksum_account")
                    .table(BankStatementImports::Table)
                    .col(BankStatementImports::BankAccountId)
                    .col(BankStatementImports::FileChecksum)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_table(manager, "bank_statement_imports").await
    }
}

#[derive(DeriveIden)]
enum BankStatementImports {
    Table,
    BankAccountId,
    FileChecksum,
}
