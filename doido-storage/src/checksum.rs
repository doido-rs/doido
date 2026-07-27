//! Integrity helpers: the base64-encoded MD5 checksum ActiveStorage stores on
//! each blob, plus byte size.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use md5::{Digest, Md5};

/// Base64-encoded MD5 digest of `data` — the same `checksum` format Rails stores.
pub fn md5_base64(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    B64.encode(hasher.finalize())
}

/// Byte size of `data`.
pub fn byte_size(data: &[u8]) -> i64 {
    data.len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_matches_known_vector() {
        // MD5("") = d41d8cd98f00b204e9800998ecf8427e → base64 of those 16 bytes.
        assert_eq!(md5_base64(b""), "1B2M2Y8AsgTpgAmY7PhCfg==");
    }

    #[test]
    fn byte_size_counts_bytes() {
        assert_eq!(byte_size(b"hello"), 5);
    }
}
