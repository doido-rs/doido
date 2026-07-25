use doido_model::databases::{Databases, Role};
use doido_model::sea_orm::ConnectionTrait;
use doido_model::testing::TestDb;

#[tokio::test]
async fn reads_route_to_the_replica_when_configured() {
    let writing = TestDb::new().await.unwrap();
    let reading = TestDb::new().await.unwrap();
    // A marker table that exists only in the replica.
    reading
        .conn()
        .execute_unprepared("CREATE TABLE only_on_replica (id INTEGER)")
        .await
        .unwrap();

    let dbs = Databases::new(writing.conn().clone()).with_reading(reading.conn().clone());
    assert!(dbs.has_replica());
    assert!(
        dbs.connection(Role::Reading)
            .execute_unprepared("SELECT COUNT(*) FROM only_on_replica")
            .await
            .is_ok(),
        "reads go to the replica"
    );
    assert!(
        dbs.connection(Role::Writing)
            .execute_unprepared("SELECT COUNT(*) FROM only_on_replica")
            .await
            .is_err(),
        "the writer has no replica-only table"
    );
}

#[tokio::test]
async fn reads_fall_back_to_the_writer_without_a_replica() {
    let writing = TestDb::new().await.unwrap();
    let dbs = Databases::new(writing.conn().clone());
    assert!(!dbs.has_replica());
    dbs.connection(Role::Reading).ping().await.unwrap();
    dbs.connection(Role::Writing).ping().await.unwrap();
}
