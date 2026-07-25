use doido_cable::{Cable, MemoryPubSub, ServerMessage};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn broadcast_sends_a_structured_server_message() {
    let cable = Cable::new(Arc::new(MemoryPubSub::new()));
    let mut rx = cable.pubsub().subscribe("room:1").await.unwrap();

    cable
        .broadcast("room:1", "ChatChannel", json!({ "text": "hi" }))
        .await
        .unwrap();

    let raw = rx.recv().await.unwrap();
    let msg = ServerMessage::parse(&raw).unwrap();
    assert_eq!(msg.identifier, "ChatChannel");
    assert_eq!(msg.message["text"], "hi");
}
