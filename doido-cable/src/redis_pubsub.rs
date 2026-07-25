//! Redis-backed pub/sub (feature `cable-redis`), so broadcasts fan out across
//! processes. Publishing uses `PUBLISH`; subscribing bridges a Redis `SUBSCRIBE`
//! stream into a local `broadcast` channel.

use crate::pubsub::PubSub;
use doido_core::Result;
use futures_util::StreamExt;
use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};

const CHANNEL_CAPACITY: usize = 128;

/// A [`PubSub`] backed by a Redis server.
pub struct RedisPubSub {
    client: redis::Client,
    publish: Mutex<redis::aio::MultiplexedConnection>,
    senders: Mutex<HashMap<String, broadcast::Sender<String>>>,
}

impl RedisPubSub {
    /// Connect to Redis at `url` (e.g. `redis://127.0.0.1/`).
    pub async fn connect(url: &str) -> Result<Self> {
        let client = redis::Client::open(url)
            .map_err(|e| doido_core::anyhow::anyhow!("redis open failed: {e}"))?;
        let publish = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("redis connect failed: {e}"))?;
        Ok(Self {
            client,
            publish: Mutex::new(publish),
            senders: Mutex::new(HashMap::new()),
        })
    }
}

#[async_trait::async_trait]
impl PubSub for RedisPubSub {
    async fn subscribe(&self, stream: &str) -> Result<broadcast::Receiver<String>> {
        let mut senders = self.senders.lock().await;
        if let Some(sender) = senders.get(stream) {
            return Ok(sender.subscribe());
        }

        let (tx, rx) = broadcast::channel(CHANNEL_CAPACITY);
        senders.insert(stream.to_string(), tx.clone());

        // Bridge the Redis SUBSCRIBE stream into the broadcast channel.
        let mut pubsub = self
            .client
            .get_async_pubsub()
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("redis pubsub connect failed: {e}"))?;
        pubsub
            .subscribe(stream)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("redis subscribe failed: {e}"))?;
        tokio::spawn(async move {
            let mut messages = pubsub.on_message();
            while let Some(msg) = messages.next().await {
                if let Ok(payload) = msg.get_payload::<String>() {
                    let _ = tx.send(payload);
                }
            }
        });

        Ok(rx)
    }

    async fn publish(&self, stream: &str, message: &str) -> Result<()> {
        use redis::AsyncCommands;
        let mut conn = self.publish.lock().await;
        let _: () = conn
            .publish(stream, message)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("redis publish failed: {e}"))?;
        Ok(())
    }

    async fn unsubscribe(&self, stream: &str) -> Result<()> {
        self.senders.lock().await.remove(stream);
        Ok(())
    }
}
