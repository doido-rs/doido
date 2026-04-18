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
