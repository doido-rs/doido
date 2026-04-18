use doido_model::testing::TestDb;

#[tokio::test]
async fn test_sea_orm_re_exports_are_accessible() {
    let db = TestDb::new().await.unwrap();
    db.conn().ping().await.unwrap();
}

#[tokio::test]
async fn test_testdb_connection_is_alive() {
    let db = TestDb::new().await.unwrap();
    // basic: second call to ping also works
    db.conn().ping().await.unwrap();
    db.conn().ping().await.unwrap();
}
