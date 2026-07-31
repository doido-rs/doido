//! Blob metadata records and their database operations.
//!
//! A [`Blob`] is one stored object's metadata (key, filename, content type,
//! checksum, size…). The raw bytes live in a [`crate::Service`]; this module owns
//! the `storage_blobs` row. Rows are addressed by the unique `key` (a UUID).
//!
//! Statements target SQLite with `?` placeholders (portable raw-SQL pattern used
//! across the framework's DB-backed stores).

use crate::content_type;
use crate::error::StorageError;
use doido_core::Result;
use doido_model::sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value as DbValue,
};
use serde_json::Value;
use uuid::Uuid;

/// One stored object's metadata.
#[derive(Debug, Clone)]
pub struct Blob {
    /// Storage key (UUID) — the object's address in the service and its identity
    /// for attachments/variants.
    pub key: String,
    /// Original filename.
    pub filename: String,
    /// Detected content type, if known.
    pub content_type: Option<String>,
    /// Arbitrary metadata (analyzer output, custom fields).
    pub metadata: Option<Value>,
    /// Name of the service the bytes were written to.
    pub service_name: String,
    /// Size in bytes.
    pub byte_size: i64,
    /// Base64 MD5 checksum.
    pub checksum: Option<String>,
    /// Creation timestamp (RFC 3339).
    pub created_at: String,
}

impl Blob {
    /// Whether the blob is an image.
    pub fn image(&self) -> bool {
        self.content_type
            .as_deref()
            .is_some_and(content_type::is_image)
    }
    /// Whether the blob is a video.
    pub fn video(&self) -> bool {
        self.content_type
            .as_deref()
            .is_some_and(content_type::is_video)
    }
    /// Whether the blob is audio.
    pub fn audio(&self) -> bool {
        self.content_type
            .as_deref()
            .is_some_and(content_type::is_audio)
    }
    /// Whether the blob is textual.
    pub fn text(&self) -> bool {
        self.content_type
            .as_deref()
            .is_some_and(content_type::is_text)
    }
}

/// Generate a fresh storage key (a dashless UUID, like Rails' 28-char keys).
pub fn new_key() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Build a [`Blob`] record for `data` without touching the database or service —
/// computes checksum, content type and byte size. Used by higher-level upload.
pub fn build(filename: &str, data: &[u8], service_name: &str, metadata: Option<Value>) -> Blob {
    Blob {
        key: new_key(),
        filename: filename.to_string(),
        content_type: Some(content_type::detect(filename, data)),
        metadata,
        service_name: service_name.to_string(),
        byte_size: crate::checksum::byte_size(data),
        checksum: Some(crate::checksum::md5_base64(data)),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn db_err(e: impl std::fmt::Display) -> StorageError {
    StorageError::Db(e.to_string())
}

/// Insert a blob metadata row.
pub async fn insert(conn: &DatabaseConnection, blob: &Blob) -> Result<()> {
    let metadata = blob
        .metadata
        .as_ref()
        .map(|m| m.to_string())
        .map(DbValue::from)
        .unwrap_or(DbValue::from(None::<String>));
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO storage_blobs \
         (key, filename, content_type, metadata, service_name, byte_size, checksum, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        [
            DbValue::from(blob.key.clone()),
            DbValue::from(blob.filename.clone()),
            DbValue::from(blob.content_type.clone()),
            metadata,
            DbValue::from(blob.service_name.clone()),
            DbValue::from(blob.byte_size),
            DbValue::from(blob.checksum.clone()),
            DbValue::from(blob.created_at.clone()),
        ],
    );
    conn.execute_raw(stmt).await.map_err(db_err)?;
    Ok(())
}

/// Look up a blob by key.
pub async fn find(conn: &DatabaseConnection, key: &str) -> Result<Option<Blob>> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "SELECT key, filename, content_type, metadata, service_name, byte_size, checksum, created_at \
         FROM storage_blobs WHERE key = ?",
        [DbValue::from(key)],
    );
    let row = conn.query_one_raw(stmt).await.map_err(db_err)?;
    let Some(row) = row else { return Ok(None) };
    let metadata: Option<String> = row.try_get("", "metadata").ok().flatten();
    Ok(Some(Blob {
        key: row.try_get("", "key").map_err(db_err)?,
        filename: row.try_get("", "filename").map_err(db_err)?,
        content_type: row.try_get("", "content_type").ok().flatten(),
        metadata: metadata.and_then(|m| serde_json::from_str(&m).ok()),
        service_name: row.try_get("", "service_name").map_err(db_err)?,
        byte_size: row.try_get("", "byte_size").map_err(db_err)?,
        checksum: row.try_get("", "checksum").ok().flatten(),
        created_at: row.try_get("", "created_at").map_err(db_err)?,
    }))
}

/// Delete a blob metadata row by key.
pub async fn delete(conn: &DatabaseConnection, key: &str) -> Result<()> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "DELETE FROM storage_blobs WHERE key = ?",
        [DbValue::from(key)],
    );
    conn.execute_raw(stmt).await.map_err(db_err)?;
    Ok(())
}
