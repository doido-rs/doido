//! Helpers for audit/version shadow tables (`{source}_versions`).

use super::{add_index, create_table, drop_table};
use crate::sea_orm::DbErr;
use crate::sea_orm_migration::SchemaManager;

/// Creates `{source_table}_versions` (or `version_table` when overridden) with a
/// paper-trail-style column set: `item_id`, `event`, `object` (JSONB), `whodunnit`,
/// `created_at`, plus indexes on `item_id` and `(item_id, created_at)`.
pub async fn create_audit_version_table(
    manager: &SchemaManager<'_>,
    source_table: &str,
    version_table: Option<&str>,
) -> Result<(), DbErr> {
    let table = version_table
        .map(str::to_string)
        .unwrap_or_else(|| format!("{source_table}_versions"));
    create_table(manager, &table, |t| {
        t.without_implicit_id();
        t.serial_id();
        t.integer("item_id").not_null();
        t.string("event").not_null();
        t.json_binary("object");
        t.string("whodunnit");
        t.timestamp_with_time_zone("created_at").not_null();
    })
    .await?;

    add_index(manager, &table, &["item_id"]).await?;
    add_index(manager, &table, &["item_id", "created_at"]).await
}

/// Drops the version table if present.
pub async fn drop_audit_version_table(
    manager: &SchemaManager<'_>,
    source_table: &str,
    version_table: Option<&str>,
) -> Result<(), DbErr> {
    let table = version_table
        .map(str::to_string)
        .unwrap_or_else(|| format!("{source_table}_versions"));
    drop_table(manager, &table).await
}
