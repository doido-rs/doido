//! `doido generate resource` — API resource without views.

use crate::common::db;
use crate::common::http;
use crate::common::{AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn resource_api_crud_over_http() {
    let h = AppHarness::new("resource", BaseProfile::Default);
    h.generate(&[
        "generate",
        "resource",
        "Product",
        "name:string:not_null",
        "price:integer",
    ]);
    h.run_with_db(
        |h| {
            db::assert_table_exists(&h.app, "products");
            db::assert_column_exists(&h.app, "products", "name");
        },
        |app| {
            http::api_crud_cycle(
                &app.base_url,
                "products",
                serde_json::json!({ "name": "Widget", "price": 9 }),
                serde_json::json!({ "name": "Updated", "price": 10 }),
            );
        },
    );
}
