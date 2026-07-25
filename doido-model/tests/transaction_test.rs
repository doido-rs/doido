use doido_model::sea_orm::ConnectionTrait;
use doido_model::testing::TestDb;
use doido_model::transaction::transaction;

#[tokio::test]
async fn transaction_commits_on_ok() {
    let db = TestDb::new().await.unwrap();
    db.conn()
        .execute_unprepared("CREATE TABLE items (id INTEGER PRIMARY KEY)")
        .await
        .unwrap();

    transaction(db.conn(), async |txn| {
        txn.execute_unprepared("INSERT INTO items (id) VALUES (1)")
            .await?;
        Ok(())
    })
    .await
    .unwrap();

    // Committed: re-inserting the same id now conflicts on the primary key.
    assert!(
        db.conn()
            .execute_unprepared("INSERT INTO items (id) VALUES (1)")
            .await
            .is_err(),
        "the row was committed"
    );
}

#[tokio::test]
async fn transaction_rolls_back_on_err() {
    let db = TestDb::new().await.unwrap();
    db.conn()
        .execute_unprepared("CREATE TABLE items (id INTEGER PRIMARY KEY)")
        .await
        .unwrap();

    let result: doido_core::Result<()> = transaction(db.conn(), async |txn| {
        txn.execute_unprepared("INSERT INTO items (id) VALUES (2)")
            .await?;
        Err(doido_core::anyhow::anyhow!("boom"))
    })
    .await;

    assert!(result.is_err());
    // Rolled back: inserting id=2 now succeeds (no conflict).
    assert!(
        db.conn()
            .execute_unprepared("INSERT INTO items (id) VALUES (2)")
            .await
            .is_ok(),
        "the insert was rolled back"
    );
}
