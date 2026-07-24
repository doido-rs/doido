//! Tests the `#[channel]` macro: it wires a struct into cable by deriving the
//! channel's registration name (its struct name, used to route ActionCable
//! `identifier` messages, per docs/12-cable.md), while leaving the user's own
//! `Channel` impl untouched.

use doido_cable::{Cable, Channel, ChannelContext, ChannelName, MemoryPubSub, PubSub};
use doido_cable_macros::channel;
use std::sync::Arc;

#[channel]
struct RoomChannel;

#[async_trait::async_trait]
impl Channel for RoomChannel {
    async fn subscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> {
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

#[test]
fn channel_macro_derives_registration_name() {
    assert_eq!(RoomChannel::channel_name(), "RoomChannel");
}

#[tokio::test]
async fn subscribe_broadcast_and_receive_flow() {
    // subscribe -> broadcast -> receive over the pub/sub
    let ps = Arc::new(MemoryPubSub::new());
    let mut rx = ps.subscribe("room:1").await.unwrap();
    let cable = Cable::new(ps.clone());
    cable
        .broadcast_to("room:1", r#"{"content":"hi"}"#)
        .await
        .unwrap();
    assert_eq!(rx.recv().await.unwrap(), r#"{"content":"hi"}"#);

    // the #[channel] type handles the lifecycle + a received frame
    let ch = RoomChannel;
    let ctx = ChannelContext {
        identifier: RoomChannel::channel_name().to_string(),
        stream: Some("room:1".to_string()),
    };
    ch.subscribed(&ctx).await.unwrap();
    ch.received(&ctx, serde_json::json!({ "action": "speak" }))
        .await
        .unwrap();
    ch.unsubscribed(&ctx).await.unwrap();
}
