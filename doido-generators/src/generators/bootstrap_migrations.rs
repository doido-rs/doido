//! Framework bootstrap migrations injected into new apps and reused by
//! `storage:install` (storage tables only).

use crate::generators::migration_support::{register_migration, render_migration_file};

/// Stable module name for the storage bootstrap migration (deterministic `doido new` output).
pub const STORAGE_MIGRATION_MODULE: &str = "m20260101000000_create_storage_tables";

/// Stable module name for the database-backed jobs queue table.
pub const JOBS_MIGRATION_MODULE: &str = "m20260101000001_create_doido_jobs_table";

const STORAGE_IMPORTS: &str = "use doido::model::migration::{add_index, create_table, drop_table};";

const STORAGE_UP_BODY: &str = r#"        create_table(manager, "storage_blobs", |t| {
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

const STORAGE_DOWN_BODY: &str = r#"        drop_table(manager, "storage_variant_records").await?;
        drop_table(manager, "storage_attachments").await?;
        drop_table(manager, "storage_blobs").await
"#;

const JOBS_IMPORTS: &str =
    "use doido::model::migration::{add_index, create_table, drop_table, remove_index};";

const JOBS_UP_BODY: &str = r#"        create_table(manager, "doido_jobs", |t| {
            t.string("job_id").not_null().unique_key();
            t.string("queue").not_null();
            t.string("status").not_null();
            t.big_integer("priority");
            t.big_integer("run_at").not_null();
            t.big_integer("locked_at");
            t.text("data").not_null();
            t.text("last_error");
        })
        .await?;
        add_index(
            manager,
            "doido_jobs",
            &["queue", "status", "run_at"],
        )
        .await?;
        Ok(())
"#;

const JOBS_DOWN_BODY: &str = r#"        remove_index(
            manager,
            "doido_jobs",
            &["queue", "status", "run_at"],
        )
        .await?;
        drop_table(manager, "doido_jobs").await
"#;

struct BootstrapMigration {
    module: &'static str,
    imports: &'static str,
    up_body: &'static str,
    down_body: &'static str,
}

const STORAGE_MIGRATION: BootstrapMigration = BootstrapMigration {
    module: STORAGE_MIGRATION_MODULE,
    imports: STORAGE_IMPORTS,
    up_body: STORAGE_UP_BODY,
    down_body: STORAGE_DOWN_BODY,
};

const JOBS_MIGRATION: BootstrapMigration = BootstrapMigration {
    module: JOBS_MIGRATION_MODULE,
    imports: JOBS_IMPORTS,
    up_body: JOBS_UP_BODY,
    down_body: JOBS_DOWN_BODY,
};

fn render_bootstrap_migration(m: &BootstrapMigration) -> String {
    render_migration_file(m.module, m.imports, m.up_body, m.down_body)
}

/// Returns `(updated mod.rs, migration module name, migration source)` when the
/// module is not already registered.
fn register_if_absent(lib: &str, m: &BootstrapMigration) -> (String, Option<(String, String)>) {
    if lib.contains(m.module) {
        return (lib.to_string(), None);
    }
    let content = render_bootstrap_migration(m);
    let lib = register_migration(lib, m.module);
    (lib, Some((m.module.to_string(), content)))
}

/// Registers bootstrap migrations into `mod.rs` and returns any new migration
/// source files to emit. Storage is always included; the jobs table is added only
/// when `jobs_db` is true.
pub fn apply_bootstrap_migrations(lib: &str, jobs_db: bool) -> (String, Vec<(String, String)>) {
    let mut files = Vec::new();
    let (lib, storage) = register_if_absent(lib, &STORAGE_MIGRATION);
    if let Some((module, content)) = storage {
        files.push((module, content));
    }
    let (lib, jobs) = if jobs_db {
        register_if_absent(&lib, &JOBS_MIGRATION)
    } else {
        (lib, None)
    };
    if let Some((module, content)) = jobs {
        files.push((module, content));
    }
    (lib, files)
}

/// True when `mod.rs` already references the storage bootstrap migration.
pub fn storage_migration_installed(lib: &str) -> bool {
    lib.contains(STORAGE_MIGRATION_MODULE) || lib.contains("create_storage_tables")
}

/// The `storage:` config block for `config/<env>.yml`.
pub fn storage_config_section(active: &str) -> String {
    format!(
        "storage:\n  driver: {active}\n  drivers:\n    local:\n      type: disk\n      root: storage\n    test:\n      type: memory\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::migration_support::MIGRATION_LIB_BASE;

    #[test]
    fn apply_injects_storage_and_jobs_when_requested() {
        let (lib, files) = apply_bootstrap_migrations(MIGRATION_LIB_BASE, true);
        assert!(lib.contains(STORAGE_MIGRATION_MODULE));
        assert!(lib.contains(JOBS_MIGRATION_MODULE));
        assert_eq!(files.len(), 2);
        let storage = files
            .iter()
            .find(|(m, _)| m == STORAGE_MIGRATION_MODULE)
            .unwrap();
        assert!(storage.1.contains("storage_blobs"));
        let jobs = files
            .iter()
            .find(|(m, _)| m == JOBS_MIGRATION_MODULE)
            .unwrap();
        assert!(jobs.1.contains("doido_jobs"));
        assert!(jobs.1.contains("job_id"));
        assert!(jobs.1.contains("create_table"));
        assert!(jobs.1.contains("add_index"));
    }

    #[test]
    fn apply_skips_jobs_migration_when_not_db_backend() {
        let (lib, files) = apply_bootstrap_migrations(MIGRATION_LIB_BASE, false);
        assert!(lib.contains(STORAGE_MIGRATION_MODULE));
        assert!(!lib.contains(JOBS_MIGRATION_MODULE));
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn apply_is_idempotent() {
        let (lib, files) = apply_bootstrap_migrations(MIGRATION_LIB_BASE, true);
        let (lib2, files2) = apply_bootstrap_migrations(&lib, true);
        assert_eq!(lib, lib2);
        assert!(files2.is_empty());
        assert_eq!(files.len(), 2);
    }
}
