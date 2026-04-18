use std::sync::Arc;
use crate::pubsub::PubSub;
use doido_core::Result;

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

#[cfg(test)]
mod tests {
    use super::Cable;
    use crate::pubsub::{MemoryPubSub, PubSub};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_cable_broadcast_to() {
        let ps = Arc::new(MemoryPubSub::new());
        let mut rx = ps.subscribe("room:1").await.unwrap();
        let cable = Cable::new(ps.clone());
        cable.broadcast_to("room:1", r#"{"event":"new_message"}"#).await.unwrap();
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg, r#"{"event":"new_message"}"#);
    }
}
