use doido_model::sea_orm::{ConnectionTrait, DbErr};
use doido_model::sea_orm_migration::{MigrationName, MigrationTrait, MigratorTrait, SchemaManager};
use doido_model::testing::TestDb;

#[tokio::test]
async fn test_testdb_connects_to_sqlite_in_memory() {
    let db = TestDb::new().await.unwrap();
    db.conn().ping().await.unwrap();
}

struct CreateWidget;
impl MigrationName for CreateWidget {
    fn name(&self) -> &str {
        "m0001_create_widget"
    }
}
#[doido_core::async_trait]
impl MigrationTrait for CreateWidget {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("CREATE TABLE widget (id INTEGER PRIMARY KEY)")
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE widget")
            .await?;
        Ok(())
    }
}
struct Migrator;
#[doido_core::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(CreateWidget)]
    }
}

#[tokio::test]
async fn test_run_migrations_applies_all() {
    let db = TestDb::new().await.unwrap();
    db.run_migrations::<Migrator>().await.unwrap();
    db.conn()
        .execute_unprepared("INSERT INTO widget (id) VALUES (1)")
        .await
        .expect("widget table exists after run_migrations");
}

#[tokio::test]
async fn test_seed_runs_the_seeder() {
    let db = TestDb::new().await.unwrap();
    db.run_migrations::<Migrator>().await.unwrap();
    db.seed(async |conn| {
        conn.execute_unprepared("INSERT INTO widget (id) VALUES (7)")
            .await?;
        Ok(())
    })
    .await
    .unwrap();
    // The seeded row is present — inserting the same PK again now fails.
    assert!(db
        .conn()
        .execute_unprepared("INSERT INTO widget (id) VALUES (7)")
        .await
        .is_err());
}
