use doido_core::Result;

pub struct ChannelContext {
    pub identifier: String,
    pub stream: Option<String>,
}

#[async_trait::async_trait]
pub trait Channel: Send + Sync {
    async fn subscribed(&self, ctx: &ChannelContext) -> Result<()>;
    async fn unsubscribed(&self, ctx: &ChannelContext) -> Result<()>;
    async fn received(&self, ctx: &ChannelContext, data: serde_json::Value) -> Result<()>;
}
