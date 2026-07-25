use doido_model::migrate::{redo, rollback};
use doido_model::sea_orm::{ConnectionTrait, DbErr};
use doido_model::sea_orm_migration::{MigrationName, MigrationTrait, MigratorTrait, SchemaManager};
use doido_model::testing::TestDb;

struct CreateThing;
impl MigrationName for CreateThing {
    fn name(&self) -> &str {
        "m0001_create_thing"
    }
}
#[doido_core::async_trait]
impl MigrationTrait for CreateThing {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("CREATE TABLE thing (id INTEGER PRIMARY KEY)")
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE thing")
            .await?;
        Ok(())
    }
}

struct Migrator;
#[doido_core::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(CreateThing)]
    }
}

#[tokio::test]
async fn rollback_undoes_the_last_migration() {
    let db = TestDb::new().await.unwrap();
    Migrator::up(db.conn(), None).await.unwrap();
    db.conn()
        .execute_unprepared("INSERT INTO thing (id) VALUES (1)")
        .await
        .unwrap();

    rollback::<Migrator>(db.conn(), 1).await.unwrap();
    assert!(
        db.conn()
            .execute_unprepared("INSERT INTO thing (id) VALUES (2)")
            .await
            .is_err(),
        "thing table was dropped by rollback"
    );
}

#[tokio::test]
async fn redo_reapplies_the_migration() {
    let db = TestDb::new().await.unwrap();
    Migrator::up(db.conn(), None).await.unwrap();

    redo::<Migrator>(db.conn(), 1).await.unwrap();
    db.conn()
        .execute_unprepared("INSERT INTO thing (id) VALUES (1)")
        .await
        .expect("thing exists again after redo");
}
