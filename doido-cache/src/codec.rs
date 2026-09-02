//! Cache value serialization with optional compression (Rails cache
//! `:compress`/serializer options). Values are serialized to JSON bytes and,
//! when requested, gzip-compressed — worthwhile for large or repetitive entries.

use doido_core::Result;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::Value;
use std::io::{Read, Write};

use base64::Engine as _;

/// Prefix for gzip-compressed entries stored as Redis/Memcache strings.
pub const GZIP_PREFIX: &str = "doido:gzip:";

/// Serialize `value` to JSON bytes, gzip-compressing when `compress` is set.
pub fn encode(value: &Value, compress: bool) -> Vec<u8> {
    let json = serde_json::to_vec(value).unwrap_or_default();
    if compress {
        gzip(&json)
    } else {
        json
    }
}

/// Decode bytes produced by [`encode`] (must use the same `compressed` flag).
pub fn decode(bytes: &[u8], compressed: bool) -> Result<Value> {
    let json = if compressed {
        gunzip(bytes)?
    } else {
        bytes.to_vec()
    };
    serde_json::from_slice(&json)
        .map_err(|e| doido_core::anyhow::anyhow!("cache decode failed: {e}"))
}

/// Serialize a cache entry for string-backed stores (Redis, in-memory JSON).
pub fn pack(value: &Value, compress: bool) -> Result<String> {
    if compress {
        let payload = encode(value, true);
        Ok(format!(
            "{GZIP_PREFIX}{}",
            base64::engine::general_purpose::STANDARD.encode(payload)
        ))
    } else {
        Ok(serde_json::to_string(value)?)
    }
}

/// Deserialize a stored string; recognizes [`GZIP_PREFIX`] regardless of the
/// current compress config so entries survive toggling the flag.
pub fn unpack(raw: &str) -> Result<Value> {
    if let Some(encoded) = raw.strip_prefix(GZIP_PREFIX) {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| doido_core::anyhow::anyhow!("cache base64 decode failed: {e}"))?;
        decode(&bytes, true)
    } else {
        serde_json::from_str(raw).map_err(|e| doido_core::anyhow::anyhow!("cache decode failed: {e}"))
    }
}

fn gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    let _ = encoder.write_all(data);
    encoder.finish().unwrap_or_default()
}

fn gunzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| doido_core::anyhow::anyhow!("gunzip failed: {e}"))?;
    Ok(out)
}
