//! Metadata tables for blobs, attachments and variant records.
//!
//! [`ensure_tables`] creates them with `CREATE TABLE IF NOT EXISTS` (SQLite-shaped
//! raw SQL) — a convenience for tests and quick development. The canonical path in
//! a real app is the migration emitted by the `storage:install` generator (which
//! uses the portable `doido_model::migration` builders in generated apps).
//!
//! Blobs are linked to attachments and variant records by their unique `key`
//! (a UUID), not by the numeric `id`, so inserts never need the auto-increment
//! value back.

use doido_core::Result;
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection};

/// Table holding one row per stored object.
pub const BLOBS_TABLE: &str = "storage_blobs";
/// Polymorphic join between application records and blobs.
pub const ATTACHMENTS_TABLE: &str = "storage_attachments";
/// Tracks generated variants of a blob.
pub const VARIANT_RECORDS_TABLE: &str = "storage_variant_records";

const CREATE_BLOBS: &str = "CREATE TABLE IF NOT EXISTS storage_blobs (\
    id INTEGER PRIMARY KEY AUTOINCREMENT, \
    key TEXT NOT NULL, \
    filename TEXT NOT NULL, \
    content_type TEXT, \
    metadata TEXT, \
    service_name TEXT NOT NULL, \
    byte_size INTEGER NOT NULL, \
    checksum TEXT, \
    created_at TEXT NOT NULL)";

const CREATE_ATTACHMENTS: &str = "CREATE TABLE IF NOT EXISTS storage_attachments (\
    id INTEGER PRIMARY KEY AUTOINCREMENT, \
    name TEXT NOT NULL, \
    record_type TEXT NOT NULL, \
    record_id TEXT NOT NULL, \
    blob_key TEXT NOT NULL, \
    created_at TEXT NOT NULL)";

const CREATE_VARIANT_RECORDS: &str = "CREATE TABLE IF NOT EXISTS storage_variant_records (\
    id INTEGER PRIMARY KEY AUTOINCREMENT, \
    blob_key TEXT NOT NULL, \
    variation_digest TEXT NOT NULL)";

const INDEXES: &[&str] = &[
    "CREATE UNIQUE INDEX IF NOT EXISTS index_storage_blobs_on_key ON storage_blobs (key)",
    "CREATE INDEX IF NOT EXISTS index_storage_attachments_on_record \
        ON storage_attachments (record_type, record_id, name)",
    "CREATE UNIQUE INDEX IF NOT EXISTS index_storage_variant_records_uniqueness \
        ON storage_variant_records (blob_key, variation_digest)",
];

/// Create the three storage tables and their indexes if they don't already
/// exist. Idempotent.
pub async fn ensure_tables(conn: &DatabaseConnection) -> Result<()> {
    for stmt in [CREATE_BLOBS, CREATE_ATTACHMENTS, CREATE_VARIANT_RECORDS] {
        conn.execute_unprepared(stmt)
            .await
            .map_err(|e| crate::error::StorageError::Db(format!("create table failed: {e}")))?;
    }
    for stmt in INDEXES {
        conn.execute_unprepared(stmt)
            .await
            .map_err(|e| crate::error::StorageError::Db(format!("create index failed: {e}")))?;
    }
    Ok(())
}
