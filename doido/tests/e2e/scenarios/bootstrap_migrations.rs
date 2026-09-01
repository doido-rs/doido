//! Bootstrap migrations emitted by `doido new`: storage (always) and jobs (with `--jobs=db`).

use crate::common::db;
use crate::common::{AppHarness, BaseProfile};

const STORAGE_MIGRATION: &str = "m20260101000000_create_storage_tables";
const JOBS_MIGRATION: &str = "m20260101000001_create_doido_jobs_table";

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn bootstrap_storage_migration_creates_tables() {
    let h = AppHarness::new("bootstrap_storage", BaseProfile::Default);
    h.run_with_db(
        |h| {
            db::assert_migrator_scaffolded(&h.app);
            db::assert_seeds_scaffolded(&h.app);
            db::assert_migration_source_exists(&h.app, STORAGE_MIGRATION);
            db::assert_mod_registers_migration(&h.app, STORAGE_MIGRATION);
            db::assert_table_exists(&h.app, "storage_blobs");
            db::assert_table_exists(&h.app, "storage_attachments");
            db::assert_table_exists(&h.app, "storage_variant_records");
            let dev = std::fs::read_to_string(h.app.join("config/development.yml")).unwrap();
            assert!(
                dev.contains("storage:"),
                "development.yml should wire storage"
            );
        },
        |_app| {},
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn bootstrap_jobs_db_migration_creates_doido_jobs_table() {
    let h = AppHarness::new("bootstrap_jobs_db", BaseProfile::WithJobsDb);
    h.run_with_db(
        |h| {
            db::assert_migrator_scaffolded(&h.app);
            db::assert_seeds_scaffolded(&h.app);
            db::assert_migration_source_exists(&h.app, JOBS_MIGRATION);
            db::assert_mod_registers_migration(&h.app, JOBS_MIGRATION);
            db::assert_table_exists(&h.app, "doido_jobs");
            // Storage bootstrap is independent of the jobs backend.
            db::assert_table_exists(&h.app, "storage_blobs");
        },
        |_app| {},
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn bootstrap_jobs_memory_omits_doido_jobs_table() {
    let h = AppHarness::new("bootstrap_jobs_memory", BaseProfile::Default);
    h.run_with_db(
        |h| {
            db::assert_migrator_scaffolded(&h.app);
            db::assert_migration_source_absent(&h.app, JOBS_MIGRATION);
            let mod_rs = std::fs::read_to_string(h.app.join("db/migration/mod.rs")).unwrap();
            assert!(
                !mod_rs.contains(JOBS_MIGRATION),
                "mod.rs must not register jobs migration for memory backend"
            );
            db::assert_table_absent(&h.app, "doido_jobs");
            // Storage bootstrap is still applied.
            db::assert_table_exists(&h.app, "storage_blobs");
        },
        |_app| {},
    );
}
