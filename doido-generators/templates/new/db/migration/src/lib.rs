pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // Register each migration here, oldest first. New migrations generated
        // with `doido generate migration <name>` should be added to this list.
        vec![]
    }
}
