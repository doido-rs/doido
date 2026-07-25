//! Cache value serialization with optional compression (Rails cache
//! `:compress`/serializer options). Values are serialized to JSON bytes and,
//! when requested, gzip-compressed — worthwhile for large or repetitive entries.

use doido_core::Result;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::Value;
use std::io::{Read, Write};

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
