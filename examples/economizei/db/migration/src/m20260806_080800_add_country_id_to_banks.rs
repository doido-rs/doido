use doido::model::migration::{
    add_column, add_foreign_key, remove_column, remove_foreign_key,
};
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260806_080800_add_country_id_to_banks"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_column(manager, "banks", "country_id", |c| {
            c.big_integer();
        })
        .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                INSERT INTO countries (name, code)
                SELECT 'Brazil', 'BR'
                WHERE NOT EXISTS (SELECT 1 FROM countries WHERE code = 'BR');

                UPDATE banks
                SET country_id = (SELECT id FROM countries WHERE code = 'BR' LIMIT 1)
                WHERE country_id IS NULL;
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE banks ALTER COLUMN country_id SET NOT NULL",
            )
            .await?;

        add_foreign_key(manager, "banks", "country_id", "countries", "id").await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        remove_foreign_key(manager, "banks", "country_id").await?;
        remove_column(manager, "banks", "country_id").await
    }
}
