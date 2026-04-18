use doido_kafka::{JsonCodec, MessageCodec};
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
    let bytes = codec.encode(&json!({})).unwrap();
    assert!(!bytes.is_empty());
}
