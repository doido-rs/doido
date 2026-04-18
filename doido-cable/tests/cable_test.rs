use doido_cable::{Cable, CableFrame, Channel, ChannelContext, MemoryPubSub, PubSub};
use doido_cable_macros::channel;
use std::sync::Arc;

#[channel]
struct ChatChannel;

#[async_trait::async_trait]
impl Channel for ChatChannel {
    async fn subscribed(&self, ctx: &ChannelContext) -> doido_core::Result<()> {
        let _ = ctx;
        Ok(())
    }
    async fn unsubscribed(&self, ctx: &ChannelContext) -> doido_core::Result<()> {
        let _ = ctx;
        Ok(())
    }
    async fn received(&self, _ctx: &ChannelContext, _data: serde_json::Value) -> doido_core::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_full_pubsub_and_cable_broadcast() {
    let ps = Arc::new(MemoryPubSub::new());
    let mut rx = ps.subscribe("chat:1").await.unwrap();
    let cable = Cable::new(ps);
    cable.broadcast_to("chat:1", "hello from cable").await.unwrap();
    let msg = rx.recv().await.unwrap();
    assert_eq!(msg, "hello from cable");
}

#[tokio::test]
async fn test_channel_macro_compiles() {
    let ch = ChatChannel;
    let ctx = ChannelContext { identifier: "ChatChannel".to_string(), stream: Some("chat:1".to_string()) };
    ch.subscribed(&ctx).await.unwrap();
    ch.received(&ctx, serde_json::json!({"action": "speak", "text": "hi"})).await.unwrap();
    ch.unsubscribed(&ctx).await.unwrap();
}

#[test]
fn test_cable_frame_protocol_parsing() {
    let frame = CableFrame::parse(r#"{"type":"subscribe","identifier":"ChatChannel"}"#).unwrap();
    assert_eq!(frame, CableFrame::Subscribe { identifier: "ChatChannel".to_string() });
}
