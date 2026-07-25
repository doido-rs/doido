//! DB pub/sub — hermetic (in-memory sqlite). Runs with `--features cable-db`
//! (see `make test-backends`); the feature is off in the fast verify gate.
#![cfg(feature = "cable-db")]

use doido_cable::db_pubsub::DbPubSub;
use doido_cable::PubSub;
use doido_model::testing::TestDb;

#[tokio::test]
async fn db_pubsub_persists_and_filters_by_id() {
    let db = TestDb::new().await.unwrap();
    let ps = DbPubSub::connect(db.conn().clone()).await.unwrap();

    ps.publish("room:1", "hello").await.unwrap();
    ps.publish("room:1", "world").await.unwrap();
    ps.publish("room:2", "other").await.unwrap();

    let msgs = ps.messages_since("room:1", 0).await.unwrap();
    let payloads: Vec<String> = msgs.iter().map(|(_, p)| p.clone()).collect();
    assert_eq!(
        payloads,
        vec!["hello", "world"],
        "durable, stream-scoped, ordered"
    );

    let after = ps.messages_since("room:1", msgs[0].0).await.unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].1, "world");
}

#[tokio::test]
async fn db_pubsub_fans_out_to_subscribers() {
    let db = TestDb::new().await.unwrap();
    let ps = DbPubSub::connect(db.conn().clone()).await.unwrap();
    let mut rx = ps.subscribe("room:x").await.unwrap();
    ps.publish("room:x", "hi").await.unwrap();
    assert_eq!(rx.recv().await.unwrap(), "hi");
}
