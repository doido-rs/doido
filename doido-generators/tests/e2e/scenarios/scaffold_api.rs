//! `doido generate scaffold --api` — JSON CRUD over HTTP with migrations applied.

use crate::common::{AppHarness, BaseProfile};
use crate::common::http;

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn scaffold_api_crud_over_http() {
    let h = AppHarness::new("scaffold_api", BaseProfile::Default);
    h.generate(&[
        "generate",
        "scaffold",
        "Post",
        "title:string",
        "body:text",
        "--api",
    ]);
    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "posts");
            crate::common::db::assert_column_exists(&h.app, "posts", "title");
        },
        |app| {
            http::api_crud_cycle(
                &app.base_url,
                "posts",
                serde_json::json!({ "title": "Hello", "body": "world" }),
            );
        },
    );
}
