//! `doido new --auth` — API and HTML auth flows after install.

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use serde_json::json;

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn auth_install_api_sign_up_and_sign_in() {
    let h = AppHarness::new("auth_install_api", BaseProfile::WithAuthApi);
    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "users");
            assert!(
                std::fs::read_to_string(h.app.join("Cargo.toml"))
                    .unwrap()
                    .contains("doido-auth"),
                "app should depend on doido-auth"
            );
            // Auth is built-in by default: no controllers/views are copied into
            // the app (run `auth:controllers` to eject them). See auth_generators.rs.
            assert!(
                !h.app.join("app/controllers/auth").exists(),
                "auth controllers should be built-in, not copied"
            );
            assert!(
                !h.app.join("app/views/auth/sign_in.html.tera").exists(),
                "API auth should not emit HTML views"
            );

            // `db seed` creates the initial admin user (before the sign-up below).
            h.seed_database();
            crate::common::db::assert_row_exists(&h.app, "users", "email", "admin@example.com");
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

            // The seeded admin (from `db seed`) can also sign in with its credentials.
            let admin = http::post_json_with_response(
                &format!("{}/users/sign_in", app.base_url),
                json!({ "email": "admin@example.com", "password": "password" }),
            );
            assert_eq!(admin.status, 200, "seeded admin should sign in");
            assert!(
                admin
                    .set_cookie
                    .iter()
                    .any(|c| c.contains("_doido_session")),
                "seeded admin sign in should set session cookie"
            );
        },
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn auth_install_html_sign_up_and_sign_in() {
    let h = AppHarness::new("auth_install_html", BaseProfile::WithAuthHtml);
    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "users");
            // Built-in-by-default: HTML auth renders the framework's overridable
            // views without copying anything into the app.
            assert!(
                !h.app.join("app/controllers/auth").exists(),
                "auth controllers should be built-in, not copied"
            );
            assert!(
                !h.app.join("app/views/auth/sign_in.html.tera").exists(),
                "auth views should be built-in, not copied by default"
            );

            // `db seed` creates the initial admin user (before the sign-up below).
            h.seed_database();
            crate::common::db::assert_row_exists(&h.app, "users", "email", "admin@example.com");
        },
        |app| {
            let sign_in_page = http::get_text(&format!("{}/users/sign_in", app.base_url));
            assert!(
                sign_in_page.contains("Sign in"),
                "sign-in page should render HTML form"
            );

            let sign_up_page = http::get_text(&format!("{}/users/sign_up", app.base_url));
            assert!(
                sign_up_page.contains("Sign up"),
                "sign-up page should render HTML form"
            );

            let sign_up = http::post_form_with_response(
                &format!("{}/users/sign_up", app.base_url),
                &[
                    ("email", "alice@example.com"),
                    ("password", "secret"),
                    ("password_confirmation", "secret"),
                ],
            );
            assert!(
                (300..400).contains(&sign_up.status),
                "sign up should redirect, got {}",
                sign_up.status
            );
            assert!(
                sign_up
                    .set_cookie
                    .iter()
                    .any(|c| c.contains("_doido_session")),
                "sign up should set session cookie"
            );

            let sign_out = http::delete_with_response(&format!("{}/users/sign_out", app.base_url));
            assert!(
                (200..400).contains(&sign_out.status),
                "sign out should succeed or redirect, got {}",
                sign_out.status
            );

            let sign_in = http::post_form_with_response(
                &format!("{}/users/sign_in", app.base_url),
                &[("email", "alice@example.com"), ("password", "secret")],
            );
            assert!(
                (300..400).contains(&sign_in.status),
                "sign in should redirect, got {}",
                sign_in.status
            );
            assert!(
                sign_in
                    .set_cookie
                    .iter()
                    .any(|c| c.contains("_doido_session")),
                "sign in should set session cookie"
            );

            // The seeded admin (from `db seed`) can also sign in with its credentials.
            let admin = http::post_form_with_response(
                &format!("{}/users/sign_in", app.base_url),
                &[("email", "admin@example.com"), ("password", "password")],
            );
            assert!(
                (300..400).contains(&admin.status),
                "seeded admin should sign in, got {}",
                admin.status
            );
            assert!(
                admin
                    .set_cookie
                    .iter()
                    .any(|c| c.contains("_doido_session")),
                "seeded admin sign in should set session cookie"
            );
        },
    );
}
