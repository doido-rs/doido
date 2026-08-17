//! CORS on API projects: opt-in via `config/<env>.yml` `[middleware.cors]`.

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use std::fs;
use std::path::Path;

const ALLOWED_ORIGIN: &str = "https://app.example";
const BLOCKED_ORIGIN: &str = "https://evil.example";

fn append_cors_config(app: &Path, origins: &[&str], methods: &[&str]) {
    let path = app.join("config/development.yml");
    let mut yaml = fs::read_to_string(&path).unwrap();
    yaml.push_str("\nmiddleware:\n  cors:\n    enabled: true\n    allowed_origins:\n");
    for origin in origins {
        yaml.push_str(&format!("      - \"{origin}\"\n"));
    }
    yaml.push_str("    allowed_methods:\n");
    for method in methods {
        yaml.push_str(&format!("      - \"{method}\"\n"));
    }
    fs::write(path, yaml).unwrap();
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn api_cors_disabled_by_default() {
    let h = AppHarness::new("cors_api_disabled", BaseProfile::ApiOnly);
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
        },
        |app| {
            let cors = http::get_with_origin(&format!("{}/posts", app.base_url), ALLOWED_ORIGIN);
            assert_eq!(
                cors.status, 200,
                "index should succeed without CORS enabled"
            );
            assert!(
                cors.allow_origin.is_none(),
                "CORS is opt-in: no allow-origin without middleware.cors.enabled"
            );
        },
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn api_cors_honors_development_yml() {
    let h = AppHarness::new("cors_api_config", BaseProfile::ApiOnly);
    h.generate(&[
        "generate",
        "scaffold",
        "Post",
        "title:string",
        "body:text",
        "--api",
    ]);
    append_cors_config(
        &h.app,
        &[ALLOWED_ORIGIN],
        &["GET", "POST", "PATCH", "DELETE", "OPTIONS"],
    );

    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "posts");
        },
        |app| {
            let posts = format!("{}/posts", app.base_url);

            let allowed = http::get_with_origin(&posts, ALLOWED_ORIGIN);
            assert_eq!(allowed.status, 200);
            assert_eq!(
                allowed.allow_origin.as_deref(),
                Some(ALLOWED_ORIGIN),
                "allowed origin should echo in Access-Control-Allow-Origin"
            );

            let blocked = http::get_with_origin(&posts, BLOCKED_ORIGIN);
            assert_eq!(blocked.status, 200);
            assert!(
                blocked.allow_origin.is_none(),
                "disallowed origin must not receive Access-Control-Allow-Origin"
            );

            let preflight = http::options_preflight(&posts, ALLOWED_ORIGIN, "POST");
            assert!(
                preflight.status == 200 || preflight.status == 204,
                "preflight should succeed, got {}",
                preflight.status
            );
            assert_eq!(
                preflight.allow_origin.as_deref(),
                Some(ALLOWED_ORIGIN),
                "preflight should allow configured origin"
            );
            assert!(
                preflight
                    .allow_methods
                    .as_deref()
                    .is_some_and(|m| m.contains("POST")),
                "preflight should advertise POST in Allow-Methods, got {:?}",
                preflight.allow_methods
            );

            http::api_crud_cycle(
                &app.base_url,
                "posts",
                serde_json::json!({ "title": "CORS", "body": "ok" }),
                serde_json::json!({ "title": "Updated", "body": "ok" }),
            );
        },
    );
}
