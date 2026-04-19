use doido_core::Result;
use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};

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
        Self {
            senders: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryPubSub {
    fn default() -> Self {
        Self::new()
    }
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
