use doido_cache::codec::{decode, encode};
use serde_json::json;

#[test]
fn round_trips_with_and_without_compression() {
    let value = json!({ "data": "x".repeat(2000), "n": 7 });

    let raw = encode(&value, false);
    let compressed = encode(&value, true);
    assert!(compressed.len() < raw.len(), "gzip shrinks repetitive data");

    assert_eq!(decode(&raw, false).unwrap(), value);
    assert_eq!(decode(&compressed, true).unwrap(), value);
}
