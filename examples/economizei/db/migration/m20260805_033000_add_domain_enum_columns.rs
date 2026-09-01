use doido::model::migration::alter_table;
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260805_033000_add_domain_enum_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_table(manager, "memberships", |t| {
            t.add_column("role", |c| {
                c.string().not_null().default("member");
            });
        })
        .await?;

        alter_table(manager, "bank_accounts", |t| {
            t.add_column("account_type", |c| {
                c.string().not_null().default("corrente");
            });
        })
        .await?;

        alter_table(manager, "transactions", |t| {
            t.add_column("operation", |c| {
                c.string().not_null().default("SAIDA");
            });
            t.add_column("movement_type", |c| {
                c.string().not_null().default("balance");
            });
            t.add_column("counterparty_id", |c| {
                c.big_integer();
            });
        })
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_table(manager, "transactions", |t| {
            t.drop_column("counterparty_id");
            t.drop_column("movement_type");
            t.drop_column("operation");
        })
        .await?;

        alter_table(manager, "bank_accounts", |t| {
            t.drop_column("account_type");
        })
        .await?;

        alter_table(manager, "memberships", |t| {
            t.drop_column("role");
        })
        .await?;

        Ok(())
    }
}
