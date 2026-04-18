use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};
use doido_core::Result;

const CHANNEL_CAPACITY: usize = 128;

#[async_trait::async_trait]
pub trait PubSub: Send + Sync {
    async fn subscribe(&self, stream: &str) -> Result<broadcast::Receiver<String>>;
    async fn publish(&self, stream: &str, message: &str) -> Result<()>;
    async fn unsubscribe(&self, stream: &str) -> Result<()>;
}

pub struct MemoryPubSub {
    senders: Mutex<HashMap<String, broadcast::Sender<String>>>,
}

impl MemoryPubSub {
    pub fn new() -> Self {
        Self { senders: Mutex::new(HashMap::new()) }
    }
}

impl Default for MemoryPubSub {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl PubSub for MemoryPubSub {
    async fn subscribe(&self, stream: &str) -> Result<broadcast::Receiver<String>> {
        let mut senders = self.senders.lock().await;
        let sender = senders
            .entry(stream.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);
        Ok(sender.subscribe())
    }

    async fn publish(&self, stream: &str, message: &str) -> Result<()> {
        let mut senders = self.senders.lock().await;
        let sender = senders
            .entry(stream.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);
        let _ = sender.send(message.to_string()); // ignore if no subscribers
        Ok(())
    }

    async fn unsubscribe(&self, stream: &str) -> Result<()> {
        self.senders.lock().await.remove(stream);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryPubSub, PubSub};
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
}
