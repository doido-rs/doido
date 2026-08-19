//! Field specs on `model`/`resource`: types, nullability, references, indexes.

use crate::common::db;
use crate::common::http;
use crate::common::{AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn model_field_specs_persist_and_serve() {
    let h = AppHarness::new("model_fields", BaseProfile::Default);
    h.generate(&[
        "generate",
        "resource",
        "Sku",
        "code:string:unique:not_null",
        "qty:integer:index",
        "active:boolean",
        "vendor:references",
    ]);
    h.run_with_db(
        |h| {
            db::assert_table_exists(&h.app, "skus");
            db::assert_column_exists(&h.app, "skus", "code");
            db::assert_column_exists(&h.app, "skus", "vendor_id");
            db::assert_column_exists(&h.app, "skus", "qty");
            db::assert_column_exists(&h.app, "skus", "active");
        },
        |app| {
            let created = http::post_json(
                &format!("{}/skus", app.base_url),
                serde_json::json!({
                    "code": "ABC-1",
                    "qty": 3,
                    "active": true,
                    "vendor_id": 42
                }),
            );
            assert_eq!(created["code"], "ABC-1");
            assert_eq!(http::get_status(&format!("{}/skus", app.base_url)), 200);
        },
    );
}
