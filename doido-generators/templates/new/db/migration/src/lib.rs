pub use sea_orm_migration::prelude::*;

// @generated-migrations-mod — `doido generate model` inserts `mod` declarations above this line. Do not remove.

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // Register migrations here, oldest first. `doido generate model` inserts
        // entries above the marker below. Do not remove the marker.
        vec![
            // @generated-migrations-list
        ]
    }
}
