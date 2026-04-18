use doido_model::testing::TestDb;

#[tokio::test]
async fn test_testdb_connects_to_sqlite_in_memory() {
    let db = TestDb::new().await.unwrap();
    db.conn().ping().await.unwrap();
}
