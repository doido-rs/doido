//! `doido new --auth` — auth tables migrated, sign-up and sign-in work.

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use serde_json::json;

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn auth_install_sign_up_and_sign_in() {
    let h = AppHarness::new("auth_install", BaseProfile::WithAuth);
    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "users");
            assert!(
                std::fs::read_to_string(h.app.join("Cargo.toml"))
                    .unwrap()
                    .contains("doido-auth"),
                "app should depend on doido-auth"
            );
        },
        |app| {
            let sign_up = http::post_json_with_response(
                &format!("{}/users/sign_up", app.base_url),
                json!({
                    "email": "alice@example.com",
                    "password": "secret",
                    "password_confirmation": "secret"
                }),
            );
            assert_eq!(sign_up.status, 200, "sign up should succeed");

            let sign_in = http::post_json_with_response(
                &format!("{}/users/sign_in", app.base_url),
                json!({
                    "email": "alice@example.com",
                    "password": "secret"
                }),
            );
            assert_eq!(sign_in.status, 200, "sign in should succeed");
            assert!(
                sign_in
                    .set_cookie
                    .iter()
                    .any(|c| c.contains("_doido_session")),
                "sign in should set session cookie"
            );
        },
    );
}
