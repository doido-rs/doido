use doido_model::schema::{dump, load};
use doido_model::sea_orm::ConnectionTrait;
use doido_model::testing::TestDb;

#[tokio::test]
async fn dump_then_load_recreates_the_schema() {
    let src = TestDb::new().await.unwrap();
    src.conn()
        .execute_unprepared("CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT)")
        .await
        .unwrap();

    let schema = dump(src.conn()).await.unwrap();
    assert!(schema.contains("CREATE TABLE"), "{schema}");
    assert!(schema.contains("widgets"));

    // Load into a fresh database and use the table.
    let dest = TestDb::new().await.unwrap();
    load(dest.conn(), &schema).await.unwrap();
    dest.conn()
        .execute_unprepared("INSERT INTO widgets (id, name) VALUES (1, 'ok')")
        .await
        .expect("table exists after load");
}
