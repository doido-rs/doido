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

const JOBS_IMPORTS: &str = "use doido::model::migration::drop_table;";

const JOBS_UP_BODY: &str = r#"        // `doido_jobs` uses a TEXT primary key (job id), not the implicit bigint `id`
        // that `create_table` adds, so the schema is emitted as raw SQL.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS doido_jobs (
                    id TEXT PRIMARY KEY,
                    queue TEXT NOT NULL,
                    status TEXT NOT NULL,
                    priority INTEGER NOT NULL DEFAULT 0,
                    run_at INTEGER NOT NULL,
                    locked_at INTEGER,
                    data TEXT NOT NULL
                )",
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_doido_jobs_reserve
                    ON doido_jobs (queue, status, run_at)",
            )
            .await?;
        Ok(())
"#;

const JOBS_DOWN_BODY: &str = r#"        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_doido_jobs_reserve")
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

/// Returns `(updated lib.rs, migration module name, migration source)` when the
/// module is not already registered.
fn register_if_absent(lib: &str, m: &BootstrapMigration) -> (String, Option<(String, String)>) {
    if lib.contains(m.module) {
        return (lib.to_string(), None);
    }
    let content = render_bootstrap_migration(m);
    let lib = register_migration(lib, m.module);
    (lib, Some((m.module.to_string(), content)))
}

/// Registers bootstrap migrations into `lib.rs` and returns any new migration
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

/// True when `lib.rs` already references the storage bootstrap migration.
pub fn storage_migration_installed(lib: &str) -> bool {
    lib.contains(STORAGE_MIGRATION_MODULE) || lib.contains("create_storage_tables")
}

/// The `storage:` config block for `config/<env>.yml`.
pub fn storage_config_section(active: &str) -> String {
    format!(
        "storage:\n  service: {active}\n  services:\n    local:\n      type: disk\n      root: storage\n    test:\n      type: memory\n"
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
        assert!(jobs.1.contains("idx_doido_jobs_reserve"));
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
