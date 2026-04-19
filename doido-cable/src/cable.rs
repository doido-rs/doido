use crate::pubsub::PubSub;
use doido_core::Result;
use std::sync::Arc;

pub struct Cable {
    pubsub: Arc<dyn PubSub>,
}

impl Cable {
    pub fn new(pubsub: Arc<dyn PubSub>) -> Self {
        Self { pubsub }
    }

    pub async fn broadcast_to(&self, stream: &str, message: &str) -> Result<()> {
        self.pubsub.publish(stream, message).await
    }
}
