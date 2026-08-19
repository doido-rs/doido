//! `doido generate storage:install` — storage tables migrated, server responds.

use crate::common::db;
use crate::common::http;
use crate::common::{AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn storage_install_creates_tables() {
    let h = AppHarness::new("storage_install", BaseProfile::Default);
    h.generate(&["generate", "storage:install"]);
    h.run_with_db(
        |h| {
            db::assert_table_exists(&h.app, "storage_blobs");
            db::assert_table_exists(&h.app, "storage_attachments");
            db::assert_table_exists(&h.app, "storage_variant_records");
            let dev = std::fs::read_to_string(h.app.join("config/development.yml")).unwrap();
            assert!(
                dev.contains("storage:"),
                "development.yml should wire storage"
            );
        },
        |app| {
            assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200);
        },
    );
}
