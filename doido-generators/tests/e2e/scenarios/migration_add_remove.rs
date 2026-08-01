//! `doido generate migration` add/remove column migrations.

use crate::common::db;
use crate::common::http;
use crate::common::{AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn migration_add_and_remove_column() {
    let h = AppHarness::new("migration_add_remove", BaseProfile::Default);
    h.generate(&[
        "generate",
        "scaffold",
        "Post",
        "title:string",
        "--api",
    ]);
    h.generate(&[
        "generate",
        "migration",
        "add_views_to_posts",
        "views:integer",
    ]);
    h.configure_server();
    h.build();
    db::prepare_database(&h.bin(), &h.app);
    db::assert_column_exists(&h.app, "posts", "views");

    h.generate(&[
        "generate",
        "migration",
        "remove_views_from_posts",
        "views:integer",
    ]);
    h.build();
    db::prepare_database(&h.bin(), &h.app);
    db::assert_column_absent(&h.app, "posts", "views");

    h.run(|app| {
        http::api_crud_cycle(
            &app.base_url,
            "posts",
            serde_json::json!({ "title": "Still works" }),
            serde_json::json!({ "title": "Updated title" }),
        );
    });
}
