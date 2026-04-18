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

#[cfg(test)]
mod tests {
    use super::{Channel, ChannelContext};

    struct EchoChannel;

    #[async_trait::async_trait]
    impl Channel for EchoChannel {
        async fn subscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> { Ok(()) }
        async fn unsubscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> { Ok(()) }
        async fn received(&self, _ctx: &ChannelContext, _data: serde_json::Value) -> doido_core::Result<()> { Ok(()) }
    }

    #[test]
    fn test_channel_trait_is_object_safe() {
        let _ch: &dyn Channel = &EchoChannel;
    }

    #[tokio::test]
    async fn test_channel_subscribed_called() {
        let ch = EchoChannel;
        let ctx = ChannelContext { identifier: "test".to_string(), stream: None };
        ch.subscribed(&ctx).await.unwrap();
    }
}
