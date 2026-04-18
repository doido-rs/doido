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

#[cfg(test)]
mod tests {
    use super::{JsonCodec, MessageCodec};
    use serde_json::json;

    #[test]
    fn test_json_codec_encode_decode_roundtrip() {
        let codec = JsonCodec;
        let value = json!({"user_id": 42, "event": "signup"});
        let bytes = codec.encode(&value).unwrap();
        let decoded = codec.decode(&bytes).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn test_json_codec_invalid_bytes_returns_error() {
        let codec = JsonCodec;
        let result = codec.decode(b"not valid json {{{{");
        assert!(result.is_err());
    }

    #[test]
    fn test_codec_is_object_safe() {
        let codec: &dyn MessageCodec = &JsonCodec;
        let bytes = codec.encode(&serde_json::json!({})).unwrap();
        assert!(!bytes.is_empty());
    }
}
