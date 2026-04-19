use doido_kafka::{Consumer, ConsumerContext, JsonCodec, MessageCodec};
use doido_kafka_macros::{consumer, topic};
use serde_json::json;

#[consumer(group = "myapp")]
struct EventConsumer;

#[async_trait::async_trait]
impl Consumer for EventConsumer {
    #[topic("user.created")]
    async fn handle(
        &self,
        ctx: &ConsumerContext,
        payload: serde_json::Value,
    ) -> doido_core::Result<()> {
        let _ = (ctx, payload);
        Ok(())
    }
}

#[test]
fn test_json_codec_roundtrip() {
    let codec = JsonCodec;
    let val = json!({"event": "user.created", "user_id": 99});
    let bytes = codec.encode(&val).unwrap();
    let decoded = codec.decode(&bytes).unwrap();
    assert_eq!(decoded["user_id"], 99);
}

#[tokio::test]
async fn test_consumer_dispatch_via_fake() {
    let consumer = EventConsumer;
    let ctx = ConsumerContext::new("user.created", 0, 100);
    let payload = json!({"user_id": 42});
    consumer.handle(&ctx, payload).await.unwrap();
}

#[test]
fn test_consumer_context_fields() {
    let ctx = ConsumerContext::new("orders.placed", 2, 9999);
    assert_eq!(ctx.topic, "orders.placed");
    assert_eq!(ctx.partition, 2);
    assert_eq!(ctx.offset, 9999);
    assert!(ctx.key.is_none());
}
