use doido::model::migration::{add_index, create_table, drop_table};
use doido::model::sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260101000000_create_storage_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_table(manager, "storage_blobs", |t| {
            t.string("key").not_null().unique_key();
            t.string("filename").not_null();
            t.string("content_type");
            t.text("metadata");
            t.string("service_name").not_null();
            t.big_integer("byte_size").not_null();
            t.string("checksum");
            t.timestamp("created_at").not_null();
        })
        .await?;
        create_table(manager, "storage_attachments", |t| {
            t.string("name").not_null();
            t.string("record_type").not_null();
            t.string("record_id").not_null();
            t.string("blob_key").not_null();
            t.timestamp("created_at").not_null();
        })
        .await?;
        create_table(manager, "storage_variant_records", |t| {
            t.string("blob_key").not_null();
            t.string("variation_digest").not_null();
        })
        .await?;
        add_index(
            manager,
            "storage_attachments",
            &["record_type", "record_id", "name"],
        )
        .await?;
        add_index(
            manager,
            "storage_variant_records",
            &["blob_key", "variation_digest"],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_table(manager, "storage_variant_records").await?;
        drop_table(manager, "storage_attachments").await?;
        drop_table(manager, "storage_blobs").await
    }
}
