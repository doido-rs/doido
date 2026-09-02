use doido_cache::codec::{decode, encode, pack, unpack};
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

#[test]
fn pack_and_unpack_string_format() {
    let value = json!({ "hello": "world", "blob": "z".repeat(1000) });
    let plain = pack(&value, false).unwrap();
    assert!(plain.starts_with('{'));
    assert_eq!(unpack(&plain).unwrap(), value);

    let gz = pack(&value, true).unwrap();
    assert!(gz.starts_with(doido_cache::codec::GZIP_PREFIX));
    assert_eq!(unpack(&gz).unwrap(), value);
}
