//! End-to-end WebSocket test: connect a real client to the axum cable server,
//! subscribe, and receive a broadcast forwarded through `stream_from`.

use doido_cable::{channel, Cable, Channel, ChannelContext, MemoryPubSub, PubSub};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;

#[channel]
struct RoomChannel;

#[async_trait::async_trait]
impl Channel for RoomChannel {
    async fn subscribed(&self, ctx: &ChannelContext) -> doido_core::Result<()> {
        ctx.stream_from("room:1").await;
        Ok(())
    }
    async fn unsubscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> {
        Ok(())
    }
    async fn received(
        &self,
        _ctx: &ChannelContext,
        _data: serde_json::Value,
    ) -> doido_core::Result<()> {
        Ok(())
    }
}

/// Read text frames until one contains `needle` (skipping welcome/pings).
async fn read_until<S>(ws: &mut S, needle: &str) -> String
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
            .await
            .expect("timed out waiting for frame")
            .expect("stream ended")
            .expect("ws error");
        if let Message::Text(text) = msg {
            if text.contains(needle) {
                return text.to_string();
            }
        }
    }
}

#[tokio::test]
async fn subscribe_then_receive_broadcast_over_websocket() {
    let pubsub: Arc<dyn PubSub> = Arc::new(MemoryPubSub::new());
    let app = doido_cable::cable!(pubsub.clone(), [RoomChannel]);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        doido_controller::axum::serve(listener, app).await.unwrap();
    });

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/cable"))
        .await
        .expect("connect");

    // Greeting.
    let welcome = read_until(&mut ws, "welcome").await;
    assert!(welcome.contains("welcome"));

    // Subscribe.
    let identifier = r#"{"channel":"RoomChannel"}"#;
    let subscribe = serde_json::json!({ "command": "subscribe", "identifier": identifier });
    ws.send(Message::text(subscribe.to_string())).await.unwrap();
    let confirm = read_until(&mut ws, "confirm_subscription").await;
    // The identifier is JSON-escaped inside the frame; check the channel name.
    assert!(confirm.contains("RoomChannel"), "got: {confirm}");

    // Broadcast to the stream the channel subscribed to; expect it at the client.
    let cable = Cable::new(pubsub.clone());
    cable
        .broadcast("room:1", identifier, serde_json::json!({ "text": "hello" }))
        .await
        .unwrap();

    let message = read_until(&mut ws, "\"text\"").await;
    assert!(message.contains("hello"), "got: {message}");
}
