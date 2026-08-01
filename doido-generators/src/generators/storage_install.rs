//! `doido generate storage:install` — the `active_storage:install` analogue.
//!
//! Emits a migration creating the `storage_blobs`, `storage_attachments` and
//! `storage_variant_records` tables (via the `doido_model::migration` builders),
//! registers it in `db/migration/src/lib.rs`, and appends a `storage:` section to
//! `config/development.yml` and `config/test.yml` when those files exist.

use crate::generator::{GeneratedFile, Generator};
use crate::generators::migration_support::{
    register_migration, render_migration_file, MIGRATION_LIB_BASE, MIGRATION_SRC_DIR,
};
use chrono::Utc;
use doido_core::Result;

pub struct StorageInstallGenerator;

const IMPORTS: &str = "use doido::model::migration::{add_index, create_table, drop_table};";

const UP_BODY: &str = r#"        create_table(manager, "storage_blobs", |t| {
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
"#;

const DOWN_BODY: &str = r#"        drop_table(manager, "storage_variant_records").await?;
        drop_table(manager, "storage_attachments").await?;
        drop_table(manager, "storage_blobs").await
"#;

/// The `storage:` config block appended to a `config/<env>.yml`. `active` is the
/// service selected for that environment.
fn storage_section(active: &str) -> String {
    format!(
        "\nstorage:\n  service: {active}\n  services:\n    local:\n      type: disk\n      root: storage\n    test:\n      type: memory\n"
    )
}

/// Append the storage section to `path` if the file exists and has none yet.
fn config_file(path: &str, active: &str) -> Option<GeneratedFile> {
    let existing = std::fs::read_to_string(path).ok()?;
    if existing.contains("storage:") {
        return None;
    }
    Some(GeneratedFile {
        path: path.to_string(),
        content: format!("{}{}", existing.trim_end(), storage_section(active)),
    })
}

impl Generator for StorageInstallGenerator {
    fn name(&self) -> &str {
        "storage:install"
    }

    fn generate(&self, _args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let migration_module = format!("m{timestamp}_create_storage_tables");
        let migration = render_migration_file(&migration_module, IMPORTS, UP_BODY, DOWN_BODY);

        let lib_path = format!("{MIGRATION_SRC_DIR}/lib.rs");
        let existing =
            std::fs::read_to_string(&lib_path).unwrap_or_else(|_| MIGRATION_LIB_BASE.to_string());
        let lib = register_migration(&existing, &migration_module);

        let mut files = vec![
            GeneratedFile {
                path: format!("{MIGRATION_SRC_DIR}/{migration_module}.rs"),
                content: migration,
            },
            GeneratedFile {
                path: lib_path,
                content: lib,
            },
        ];

        // Best-effort: wire the `storage` config section into existing env files.
        if let Some(f) = config_file("config/development.yml", "local") {
            files.push(f);
        }
        if let Some(f) = config_file("config/test.yml", "test") {
            files.push(f);
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_migration_and_registers_it() {
        let files = StorageInstallGenerator.generate(&[]).unwrap();
        // A migration file plus the updated lib.rs (config files depend on cwd).
        let migration = files
            .iter()
            .find(|f| f.path.contains("create_storage_tables") && f.path.ends_with(".rs"))
            .expect("migration file emitted");
        assert!(migration.content.contains("storage_blobs"));
        assert!(migration.content.contains("storage_attachments"));
        assert!(migration.content.contains("storage_variant_records"));
        assert!(migration.content.contains("impl MigrationName for Migration"));
        assert!(!migration.content.contains("DeriveMigrationName"));
        let module = migration
            .path
            .strip_prefix("db/migration/src/")
            .unwrap()
            .strip_suffix(".rs")
            .unwrap();
        assert!(migration.content.contains(&format!("\"{module}\"")));

        let lib = files
            .iter()
            .find(|f| f.path.ends_with("lib.rs"))
            .expect("lib.rs emitted");
        assert!(lib.content.contains("create_storage_tables"));
    }
}
