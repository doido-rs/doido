use doido_cable::{MemoryPubSub, PubSub};
use std::sync::Arc;

#[tokio::test]
async fn test_publish_and_receive() {
    let ps = Arc::new(MemoryPubSub::new());
    let mut rx = ps.subscribe("stream1").await.unwrap();
    ps.publish("stream1", "hello").await.unwrap();
    let msg = rx.recv().await.unwrap();
    assert_eq!(msg, "hello");
}

#[tokio::test]
async fn test_multiple_subscribers_receive_same_message() {
    let ps = Arc::new(MemoryPubSub::new());
    let mut rx1 = ps.subscribe("stream1").await.unwrap();
    let mut rx2 = ps.subscribe("stream1").await.unwrap();
    ps.publish("stream1", "broadcast").await.unwrap();
    assert_eq!(rx1.recv().await.unwrap(), "broadcast");
    assert_eq!(rx2.recv().await.unwrap(), "broadcast");
}

#[tokio::test]
async fn test_different_streams_are_isolated() {
    let ps = Arc::new(MemoryPubSub::new());
    let mut rx1 = ps.subscribe("stream1").await.unwrap();
    ps.publish("stream2", "other").await.unwrap();
    ps.publish("stream1", "mine").await.unwrap();
    let msg = rx1.recv().await.unwrap();
    assert_eq!(msg, "mine");
}

#[tokio::test]
async fn test_publish_without_subscribers_succeeds() {
    let ps = MemoryPubSub::default();
    ps.publish("orphan", "msg").await.unwrap();
}

#[tokio::test]
async fn test_unsubscribe_removes_stream() {
    let ps = Arc::new(MemoryPubSub::new());
    let _rx = ps.subscribe("gone").await.unwrap();
    ps.unsubscribe("gone").await.unwrap();
    ps.publish("gone", "after").await.unwrap();
    // Fresh subscribe should not receive the old channel's history.
    let mut rx2 = ps.subscribe("gone").await.unwrap();
    ps.publish("gone", "fresh").await.unwrap();
    assert_eq!(rx2.recv().await.unwrap(), "fresh");
}
