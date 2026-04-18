use doido_kafka::{Consumer, ConsumerContext};
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
