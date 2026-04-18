#[derive(Debug, Clone)]
pub struct ConsumerContext {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<Vec<u8>>,
}

impl ConsumerContext {
    pub fn new(topic: impl Into<String>, partition: i32, offset: i64) -> Self {
        Self { topic: topic.into(), partition, offset, key: None }
    }
}

#[async_trait::async_trait]
pub trait Consumer: Send + Sync {
    async fn handle(&self, ctx: &ConsumerContext, payload: serde_json::Value) -> doido_core::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::{Consumer, ConsumerContext};
    use serde_json::json;

    struct FakeConsumer;

    #[async_trait::async_trait]
    impl Consumer for FakeConsumer {
        async fn handle(&self, _ctx: &ConsumerContext, _payload: serde_json::Value) -> doido_core::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_consumer_is_object_safe() {
        let _c: &dyn Consumer = &FakeConsumer;
    }

    #[tokio::test]
    async fn test_consumer_handle_called() {
        let c = FakeConsumer;
        let ctx = ConsumerContext::new("user.created", 0, 0);
        c.handle(&ctx, json!({"user_id": 1})).await.unwrap();
    }
}
