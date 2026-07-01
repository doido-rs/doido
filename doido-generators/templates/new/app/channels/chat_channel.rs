//! Example doido-cable channel.
//!
//! A `Channel` reacts to a client's WebSocket lifecycle — `subscribed`,
//! `unsubscribed`, and each `received` message — and broadcasts to other
//! clients through a pub/sub backend (`MemoryPubSub` here; Redis/DB are
//! swappable via config once wired). Broadcasting is done through a shared
//! `Cable` handle: everything published to a named stream is fanned out to
//! every subscriber of that stream.
//!
//! The `#[tokio::test]` at the bottom is a runnable, self-contained demo of the
//! whole subscribe → broadcast → receive round-trip.
#![allow(dead_code)]

use doido_cable::{Cable, Channel, ChannelContext};
use serde_json::Value;
use std::sync::Arc;

/// The stream this channel fans messages out on.
const STREAM: &str = "chat";

pub struct ChatChannel {
    /// Shared broadcast handle. In an app you build one `Cable` at startup
    /// (wrapping your configured pub/sub backend) and hand a clone to each
    /// channel.
    cable: Arc<Cable>,
}

impl ChatChannel {
    pub fn new(cable: Arc<Cable>) -> Self {
        Self { cable }
    }
}

#[async_trait::async_trait]
impl Channel for ChatChannel {
    /// A client subscribed — authorize and set up per-connection state here.
    async fn subscribed(&self, ctx: &ChannelContext) -> doido::Result<()> {
        doido::core::tracing::info!("chat: subscribed ({})", ctx.identifier);
        Ok(())
    }

    /// A client unsubscribed / disconnected — clean up here.
    async fn unsubscribed(&self, ctx: &ChannelContext) -> doido::Result<()> {
        doido::core::tracing::info!("chat: unsubscribed ({})", ctx.identifier);
        Ok(())
    }

    /// A message arrived from a subscribed client — relay it to everyone on the
    /// `chat` stream.
    async fn received(&self, _ctx: &ChannelContext, data: Value) -> doido::Result<()> {
        self.cable.broadcast_to(STREAM, &data.to_string()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doido_cable::{MemoryPubSub, PubSub};

    #[tokio::test]
    async fn received_broadcasts_to_stream_subscribers() {
        // 1. Build a pub/sub backend and a Cable handle over it.
        let pubsub = Arc::new(MemoryPubSub::new());
        let cable = Arc::new(Cable::new(pubsub.clone()));

        // 2. A client is listening on the `chat` stream.
        let mut subscriber = pubsub.subscribe(STREAM).await.unwrap();

        // 3. Another client sends a message; the channel relays it.
        let channel = ChatChannel::new(cable);
        let ctx = ChannelContext {
            identifier: r#"{"channel":"ChatChannel"}"#.to_string(),
            stream: Some(STREAM.to_string()),
        };
        channel
            .received(&ctx, serde_json::json!({ "message": "hello" }))
            .await
            .unwrap();

        // 4. The listener receives the broadcast.
        let delivered = subscriber.recv().await.unwrap();
        assert!(delivered.contains("hello"));
    }
}
