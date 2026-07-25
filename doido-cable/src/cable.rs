use crate::protocol::ServerMessage;
use crate::pubsub::PubSub;
use doido_core::Result;
use serde_json::Value;
use std::sync::Arc;

pub struct Cable {
    pubsub: Arc<dyn PubSub>,
}

impl Cable {
    pub fn new(pubsub: Arc<dyn PubSub>) -> Self {
        Self { pubsub }
    }

    /// Publish a raw string to `stream`.
    pub async fn broadcast_to(&self, stream: &str, message: &str) -> Result<()> {
        self.pubsub.publish(stream, message).await
    }

    /// Broadcast a structured ActionCable message (identifier + data) to a
    /// stream from anywhere in the app (Rails `Channel.broadcast_to`). Subscribers
    /// receive it as a JSON [`ServerMessage`].
    pub async fn broadcast(&self, stream: &str, identifier: &str, message: Value) -> Result<()> {
        let frame = ServerMessage::new(identifier, message);
        self.pubsub.publish(stream, &frame.to_json()?).await
    }

    /// The underlying pub/sub backend (e.g. to subscribe a connection).
    pub fn pubsub(&self) -> &Arc<dyn PubSub> {
        &self.pubsub
    }
}
