use doido_cable::{Cable, MemoryPubSub, PubSub};
use std::sync::Arc;

#[tokio::test]
async fn test_cable_broadcast_to() {
    let ps = Arc::new(MemoryPubSub::new());
    let mut rx = ps.subscribe("room:1").await.unwrap();
    let cable = Cable::new(ps.clone());
    cable
        .broadcast_to("room:1", r#"{"event":"new_message"}"#)
        .await
        .unwrap();
    let msg = rx.recv().await.unwrap();
    assert_eq!(msg, r#"{"event":"new_message"}"#);
}
