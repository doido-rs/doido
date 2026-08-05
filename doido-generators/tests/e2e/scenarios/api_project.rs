//! `doido new --api` marks the project `api_only` in `config/application.toml`.
//! The route macros read that marker at compile time and drop the `new`/`edit`
//! form routes from `resources!` — so a **plain** `resources!(posts, Controller)`
//! pointing at the JSON controller (which defines no `new`/`edit` methods) both
//! compiles and serves no form routes. In a non-API project the same line would
//! fail to compile (dangling `PostsController::new`/`::edit` references), so a
//! successful build here is itself proof that the macro honored the marker.

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use std::fs;

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn api_only_project_drops_form_routes() {
    let h = AppHarness::new("api_project", BaseProfile::ApiOnly);

    // Sanity: the api-only project carries the marker the macros read.
    let app_toml = fs::read_to_string(h.app.join("config/application.toml")).unwrap();
    assert!(
        app_toml.contains("api_only = true"),
        "expected api_only marker in application.toml:\n{app_toml}"
    );

    h.generate(&[
        "generate",
        "scaffold",
        "Post",
        "title:string",
        "body:text",
        "--api",
    ]);

    // The api scaffold injects `except: [new, edit]`; rewrite it to the plain
    // form so the build exercises the macro's marker-driven dropping instead.
    let routes_path = h.app.join("config/routes.rs");
    let routes = fs::read_to_string(&routes_path).unwrap();
    let plain = routes.replace(
        "resources!(posts, PostsController, except: [new, edit]);",
        "resources!(posts, PostsController);",
    );
    assert_ne!(
        routes, plain,
        "expected the api scaffold to inject `except: [new, edit]` to rewrite:\n{routes}"
    );
    fs::write(&routes_path, plain).unwrap();

    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "posts");
        },
        |app| {
            // The form routes are absent: `/posts/{id}/edit` is unrouted (404),
            // and `/posts/new` falls through to `show` (no row 0) rather than a
            // `new` action.
            assert_eq!(
                http::get_status_any(&format!("{}/posts/1/edit", app.base_url)),
                404,
                "edit form route must not exist in an api_only project"
            );
            assert_eq!(
                http::get_status_any(&format!("{}/posts/new", app.base_url)),
                404,
                "new form route must not exist in an api_only project"
            );

            // The JSON actions (index/show/create/update/destroy) still work.
            http::api_crud_cycle(
                &app.base_url,
                "posts",
                serde_json::json!({ "title": "Hello", "body": "world" }),
                serde_json::json!({ "title": "Updated", "body": "world" }),
            );
        },
    );
}
