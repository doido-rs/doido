use doido_model::sea_orm::ConnectionTrait;
use doido_model::seeds::run_seeds;
use doido_model::testing::TestDb;

#[tokio::test]
async fn run_seeds_executes_the_seeder() {
    let db = TestDb::new().await.unwrap();
    db.conn()
        .execute_unprepared("CREATE TABLE roles (id INTEGER PRIMARY KEY, name TEXT)")
        .await
        .unwrap();

    run_seeds(
        db.conn(),
        async |conn: &doido_model::sea_orm::DatabaseConnection| {
            conn.execute_unprepared("INSERT INTO roles (id, name) VALUES (1, 'admin')")
                .await?;
            Ok(())
        },
    )
    .await
    .unwrap();

    // seeded row exists -> re-inserting the same id conflicts
    assert!(db
        .conn()
        .execute_unprepared("INSERT INTO roles (id, name) VALUES (1, 'x')")
        .await
        .is_err());
}
