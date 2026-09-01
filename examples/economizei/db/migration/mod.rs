pub use doido::model::sea_orm_migration::prelude::*;

mod m20260101000000_create_storage_tables;
mod m20260101000001_create_doido_jobs_table;
mod m20260805_025809_create_users_table;
mod m20260805_032521_create_companies_table;
mod m20260805_032532_create_banks_table;
mod m20260805_032535_create_memberships_table;
mod m20260805_032538_create_bank_accounts_table;
mod m20260805_032541_create_categories_table;
mod m20260805_032545_create_counterparties_table;
mod m20260805_032551_create_transactions_table;
mod m20260805_033000_add_domain_enum_columns;
mod m20260805_120000_create_bank_statement_imports_table;
mod m20260806_080706_create_countries_table;
mod m20260806_080800_add_country_id_to_banks;
mod m20260806_082900_fix_doido_jobs_timestamp_columns;
mod m20260812_151040_add_description_to_transactions;
mod m20260814_205400_add_auth_module_columns_to_users;
// @generated-migrations-mod — `doido generate model` inserts `mod` declarations above this line. Do not remove.

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // Register migrations here, oldest first. `doido generate model` inserts
        // entries above the marker below. Do not remove the marker.
        vec![
            Box::new(m20260101000000_create_storage_tables::Migration),
            Box::new(m20260101000001_create_doido_jobs_table::Migration),
            Box::new(m20260805_025809_create_users_table::Migration),
            Box::new(m20260805_032521_create_companies_table::Migration),
            Box::new(m20260805_032532_create_banks_table::Migration),
            Box::new(m20260805_032535_create_memberships_table::Migration),
            Box::new(m20260805_032538_create_bank_accounts_table::Migration),
            Box::new(m20260805_032541_create_categories_table::Migration),
            Box::new(m20260805_032545_create_counterparties_table::Migration),
            Box::new(m20260805_032551_create_transactions_table::Migration),
            Box::new(m20260805_033000_add_domain_enum_columns::Migration),
            Box::new(
                m20260805_120000_create_bank_statement_imports_table::Migration,
            ),
            Box::new(m20260806_080706_create_countries_table::Migration),
            Box::new(m20260806_080800_add_country_id_to_banks::Migration),
            Box::new(
                m20260806_082900_fix_doido_jobs_timestamp_columns::Migration,
            ),
            Box::new(m20260812_151040_add_description_to_transactions::Migration),
            Box::new(m20260814_205400_add_auth_module_columns_to_users::Migration),
            // @generated-migrations-list
        ]
    }
}
