//! Polymorphic attachments — the `has_one_attached` / `has_many_attached`
//! analogue, exposed as helper functions over `(record_type, record_id, name)`.
//!
//! `record_type` is the application model name (e.g. `"User"`), `record_id` its
//! primary key rendered as text, and `name` the attachment slot (e.g. `"avatar"`).
//! Rows link to blobs by the blob `key`.
//!
//! A future proc-macro (`#[has_one_attached]`) can generate typed accessors on
//! sea-orm models on top of these functions.

use crate::blob::{self, Blob};
use crate::error::StorageError;
use doido_core::Result;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value as DbValue,
};

fn db_err(e: impl std::fmt::Display) -> StorageError {
    StorageError::Db(e.to_string())
}

/// Attach the blob `blob_key` to `record` under `name`. For a `has_one` slot,
/// callers should [`detach`] first (see [`replace_one`]).
pub async fn attach(
    conn: &DatabaseConnection,
    name: &str,
    record_type: &str,
    record_id: &str,
    blob_key: &str,
) -> Result<()> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO storage_attachments (name, record_type, record_id, blob_key, created_at) \
         VALUES (?, ?, ?, ?, ?)",
        [
            DbValue::from(name),
            DbValue::from(record_type),
            DbValue::from(record_id),
            DbValue::from(blob_key),
            DbValue::from(chrono::Utc::now().to_rfc3339()),
        ],
    );
    conn.execute_raw(stmt).await.map_err(db_err)?;
    Ok(())
}

/// `has_one_attached`: replace any existing attachment in the `name` slot with
/// `blob_key`.
pub async fn replace_one(
    conn: &DatabaseConnection,
    name: &str,
    record_type: &str,
    record_id: &str,
    blob_key: &str,
) -> Result<()> {
    detach(conn, name, record_type, record_id).await?;
    attach(conn, name, record_type, record_id, blob_key).await
}

/// The blob keys attached to `record` under `name`, oldest first.
pub async fn attached_keys(
    conn: &DatabaseConnection,
    name: &str,
    record_type: &str,
    record_id: &str,
) -> Result<Vec<String>> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "SELECT blob_key FROM storage_attachments \
         WHERE name = ? AND record_type = ? AND record_id = ? ORDER BY id ASC",
        [
            DbValue::from(name),
            DbValue::from(record_type),
            DbValue::from(record_id),
        ],
    );
    let rows = conn.query_all_raw(stmt).await.map_err(db_err)?;
    rows.into_iter()
        .map(|r| {
            r.try_get::<String>("", "blob_key")
                .map_err(|e| db_err(e).into())
        })
        .collect()
}

/// `has_one_attached`: the single attached blob, if any.
pub async fn one(
    conn: &DatabaseConnection,
    name: &str,
    record_type: &str,
    record_id: &str,
) -> Result<Option<Blob>> {
    let keys = attached_keys(conn, name, record_type, record_id).await?;
    match keys.first() {
        Some(key) => blob::find(conn, key).await,
        None => Ok(None),
    }
}

/// `has_many_attached`: all attached blobs.
pub async fn many(
    conn: &DatabaseConnection,
    name: &str,
    record_type: &str,
    record_id: &str,
) -> Result<Vec<Blob>> {
    let keys = attached_keys(conn, name, record_type, record_id).await?;
    let mut blobs = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(b) = blob::find(conn, &key).await? {
            blobs.push(b);
        }
    }
    Ok(blobs)
}

/// Remove the attachment rows in the `name` slot (does not delete the blobs).
pub async fn detach(
    conn: &DatabaseConnection,
    name: &str,
    record_type: &str,
    record_id: &str,
) -> Result<()> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "DELETE FROM storage_attachments WHERE name = ? AND record_type = ? AND record_id = ?",
        [
            DbValue::from(name),
            DbValue::from(record_type),
            DbValue::from(record_id),
        ],
    );
    conn.execute_raw(stmt).await.map_err(db_err)?;
    Ok(())
}

/// All blob keys attached to a record across every slot — used by
/// `purge_for_record` to purge dependent blobs when a record is destroyed.
pub async fn all_keys_for_record(
    conn: &DatabaseConnection,
    record_type: &str,
    record_id: &str,
) -> Result<Vec<String>> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "SELECT blob_key FROM storage_attachments WHERE record_type = ? AND record_id = ?",
        [DbValue::from(record_type), DbValue::from(record_id)],
    );
    let rows = conn.query_all_raw(stmt).await.map_err(db_err)?;
    rows.into_iter()
        .map(|r| {
            r.try_get::<String>("", "blob_key")
                .map_err(|e| db_err(e).into())
        })
        .collect()
}

/// Delete every attachment row pointing at `blob_key` (used when purging a blob).
pub async fn delete_for_blob(conn: &DatabaseConnection, blob_key: &str) -> Result<()> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "DELETE FROM storage_attachments WHERE blob_key = ?",
        [DbValue::from(blob_key)],
    );
    conn.execute_raw(stmt).await.map_err(db_err)?;
    Ok(())
}

/// Delete a single attachment row linking `record`/`name` to `blob_key`.
pub async fn detach_blob(
    conn: &DatabaseConnection,
    name: &str,
    record_type: &str,
    record_id: &str,
    blob_key: &str,
) -> Result<()> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "DELETE FROM storage_attachments \
         WHERE name = ? AND record_type = ? AND record_id = ? AND blob_key = ?",
        [
            DbValue::from(name),
            DbValue::from(record_type),
            DbValue::from(record_id),
            DbValue::from(blob_key),
        ],
    );
    conn.execute_raw(stmt).await.map_err(db_err)?;
    Ok(())
}
