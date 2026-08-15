use doido::model::migration::alter_table;
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260814_205400_add_auth_module_columns_to_users"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_table(manager, "users", |t| {
            t.add_column("remember_created_at", |c| {
                c.timestamp();
            });
            t.add_column("reset_password_token", |c| {
                c.string();
            });
            t.add_column("reset_password_sent_at", |c| {
                c.timestamp();
            });
        })
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_table(manager, "users", |t| {
            t.drop_column("remember_created_at");
            t.drop_column("reset_password_token");
            t.drop_column("reset_password_sent_at");
        })
        .await
    }
}
