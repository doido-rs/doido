use doido_core::Result;
use serde_json::Value;

pub trait MessageCodec: Send + Sync {
    fn encode(&self, value: &Value) -> Result<Vec<u8>>;
    fn decode(&self, bytes: &[u8]) -> Result<Value>;
}

pub struct JsonCodec;

impl MessageCodec for JsonCodec {
    fn encode(&self, value: &Value) -> Result<Vec<u8>> {
        serde_json::to_vec(value)
            .map_err(|e| doido_core::anyhow::anyhow!("encode error: {e}"))
    }

    fn decode(&self, bytes: &[u8]) -> Result<Value> {
        serde_json::from_slice(bytes)
            .map_err(|e| doido_core::anyhow::anyhow!("decode error: {e}"))
    }
}
