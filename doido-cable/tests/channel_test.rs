use doido_cable::{Channel, ChannelContext};

struct EchoChannel;

#[async_trait::async_trait]
impl Channel for EchoChannel {
    async fn subscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> {
        Ok(())
    }
    async fn unsubscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> {
        Ok(())
    }
    async fn received(
        &self,
        _ctx: &ChannelContext,
        _data: serde_json::Value,
    ) -> doido_core::Result<()> {
        Ok(())
    }
}

#[test]
fn test_channel_trait_is_object_safe() {
    let _ch: &dyn Channel = &EchoChannel;
}

#[tokio::test]
async fn test_channel_subscribed_called() {
    let ch = EchoChannel;
    let ctx = ChannelContext {
        identifier: "test".to_string(),
        stream: None,
    };
    ch.subscribed(&ctx).await.unwrap();
}
