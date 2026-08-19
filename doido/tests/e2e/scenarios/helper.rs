//! `doido generate helper` — generated helper wired into a controller and exercised over HTTP.

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use std::fs;
use std::path::Path;

fn assert_file(app: &Path, rel: &str) {
    assert!(app.join(rel).is_file(), "expected generated file `{rel}`");
}

/// Adds a real method to the generated helper and routes `/` through it.
fn wire_posts_helper_into_hello(app: &Path) {
    fs::write(
        app.join("app/helpers/mod.rs"),
        r#"//! Controller helpers (doido-controller).

pub mod posts_helper;
pub use posts_helper::PostsHelper;

// @generated-helpers
"#,
    )
    .expect("write app/helpers/mod.rs");

    fs::write(
        app.join("app/helpers/posts_helper.rs"),
        r#"//! Generated controller helper — exercised by release e2e.
#![allow(dead_code)]

use doido::controller::helper;

#[helper]
pub struct PostsHelper;

impl PostsHelper {
    pub fn format_title(title: &str) -> String {
        title.trim().to_uppercase()
    }
}
"#,
    )
    .expect("write posts_helper.rs");

    fs::write(
        app.join("app/controllers/hello_controller.rs"),
        r#"use crate::helpers::PostsHelper;
use doido::controller::controller;
use serde_json::json;

pub struct HelloController;

#[controller]
impl HelloController {
    pub async fn index(
        ctx: doido::controller::Context,
    ) -> doido::controller::Response {
        ctx.json(json!({
            "title": PostsHelper::format_title("  e2e  ")
        }))
    }
}
"#,
    )
    .expect("write hello_controller.rs");
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn helper_generator_executes_method_over_http() {
    let h = AppHarness::new("helper", BaseProfile::Default);
    h.generate(&["generate", "helper", "Posts"]);
    wire_posts_helper_into_hello(&h.app);

    h.run_with_db(
        |h| {
            assert_file(&h.app, "app/helpers/posts_helper.rs");
            let helpers_mod = fs::read_to_string(h.app.join("app/helpers/mod.rs"))
                .expect("read app/helpers/mod.rs");
            assert!(
                helpers_mod.contains("pub mod posts_helper;"),
                "posts_helper must be registered in app/helpers/mod.rs"
            );
            assert!(
                helpers_mod.contains("pub use posts_helper::PostsHelper;"),
                "PostsHelper must be re-exported from app/helpers/mod.rs"
            );
        },
        |app| {
            let body = http::get_json(&format!("{}/", app.base_url));
            assert_eq!(body["title"], "E2E");
        },
    );
}
