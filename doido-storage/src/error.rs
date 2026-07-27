//! Typed storage errors (`thiserror` per crate, per the framework convention).
//!
//! Public APIs return `doido_core::Result<T>` (anyhow) like the rest of the
//! framework; [`StorageError`] gives callers a matchable error type and converts
//! into `anyhow::Error` automatically via the `?` operator.

/// Errors raised by the storage layer.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// A blob/attachment/object was not found.
    #[error("storage: not found: {0}")]
    NotFound(String),

    /// A filesystem or backend I/O failure.
    #[error("storage: io error: {0}")]
    Io(String),

    /// The selected service backend failed (S3/Azure/disk).
    #[error("storage: backend error: {0}")]
    Backend(String),

    /// The `storage` configuration is invalid or selects an unavailable backend.
    #[error("storage: config error: {0}")]
    Config(String),

    /// A signed id / signed URL is malformed, tampered with, or expired.
    #[error("storage: invalid signature: {0}")]
    InvalidSignature(String),

    /// A database (metadata) failure.
    #[error("storage: database error: {0}")]
    Db(String),
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(e.to_string())
    }
}
