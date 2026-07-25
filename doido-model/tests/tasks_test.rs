use doido_model::sea_orm::ConnectionTrait;
use doido_model::tasks::{prepare, reset, setup};
use doido_model::testing::TestDb;

#[tokio::test]
async fn reset_replaces_the_schema() {
    let db = TestDb::new().await.unwrap();
    db.conn()
        .execute_unprepared("CREATE TABLE old_t (id INTEGER)")
        .await
        .unwrap();

    reset(db.conn(), "CREATE TABLE new_t (id INTEGER PRIMARY KEY);")
        .await
        .unwrap();

    db.conn()
        .execute_unprepared("INSERT INTO new_t (id) VALUES (1)")
        .await
        .expect("new table exists");
    assert!(
        db.conn()
            .execute_unprepared("INSERT INTO old_t (id) VALUES (1)")
            .await
            .is_err(),
        "old table dropped"
    );
}

#[tokio::test]
async fn setup_loads_schema_and_seeds() {
    let db = TestDb::new().await.unwrap();
    setup(
        db.conn(),
        "CREATE TABLE roles (id INTEGER PRIMARY KEY);",
        async |c: &doido_model::sea_orm::DatabaseConnection| {
            c.execute_unprepared("INSERT INTO roles (id) VALUES (1)")
                .await?;
            Ok(())
        },
    )
    .await
    .unwrap();
    assert!(
        db.conn()
            .execute_unprepared("INSERT INTO roles (id) VALUES (1)")
            .await
            .is_err(),
        "seeded row present"
    );
}

#[tokio::test]
async fn prepare_is_idempotent() {
    let db = TestDb::new().await.unwrap();
    prepare(db.conn(), "CREATE TABLE t (id INTEGER PRIMARY KEY);")
        .await
        .unwrap();
    // second prepare must not fail (schema not reloaded because tables exist)
    prepare(db.conn(), "CREATE TABLE t (id INTEGER PRIMARY KEY);")
        .await
        .unwrap();
}
