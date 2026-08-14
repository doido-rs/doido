//! `doido new --auth` no longer copies auth controllers/views: auth runs on the
//! framework's built-in controllers + overridable framework views. The
//! `auth:controllers` generator ejects them into the app for customization.
//!
//! Test 1 proves the no-copy default works end-to-end over HTTP.
//! Test 2 proves ejecting produces local files, rewires routes, and that a
//! customized ejected view is actually served (i.e. overrides the framework one).

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use std::fs;

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn new_auth_uses_builtin_controllers_without_copying_files() {
    let h = AppHarness::new("auth_builtin_html", BaseProfile::WithAuthHtml);

    // `doido new --auth` must not have copied any auth controllers or views.
    assert!(
        !h.app.join("app/controllers/auth").exists(),
        "no auth controllers should be copied into the app"
    );
    assert!(
        !h.app.join("app/views/auth/sign_in.html.tera").exists(),
        "no auth views should be copied into the app"
    );
    let routes = fs::read_to_string(h.app.join("config/routes.rs")).unwrap();
    assert!(
        routes.contains("auth_routes!(User)"),
        "routes should mount auth via the macro"
    );
    assert!(
        !routes.contains("controllers: {"),
        "routes should target built-in controllers (no local override)"
    );

    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "users");
            h.seed_database();
            crate::common::db::assert_row_exists(&h.app, "users", "email", "admin@example.com");
        },
        |app| {
            // The built-in HTML sign-in controller renders the *framework* view
            // even though nothing was copied into app/views.
            let sign_in_page = http::get_text(&format!("{}/users/sign_in", app.base_url));
            assert!(
                sign_in_page.contains("Sign in"),
                "built-in sign-in view should render, got: {sign_in_page}"
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

            // Sign in with "remember me" — the `rememberable` module (a default
            // module) issues a persistent signed cookie alongside the session.
            let sign_in = http::post_form_with_response(
                &format!("{}/users/sign_in", app.base_url),
                &[
                    ("email", "alice@example.com"),
                    ("password", "secret"),
                    ("remember", "1"),
                ],
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
                "sign in should set the session cookie"
            );
            assert!(
                sign_in
                    .set_cookie
                    .iter()
                    .any(|c| c.contains("_doido_remember") && c.contains("Max-Age")),
                "remember me should set a persistent remember cookie"
            );

            // `recoverable` (a default module): requesting a reset runs without
            // error against the generated reset-password columns.
            let reset = http::post_form_with_response(
                &format!("{}/users/password", app.base_url),
                &[("email", "admin@example.com")],
            );
            assert!(
                reset.status < 500,
                "password reset request should not error, got {}",
                reset.status
            );
        },
    );
}

const EJECT_MARKER: &str = "EJECTED-CUSTOM-SIGN-IN-MARKER";

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn auth_controllers_generator_ejects_and_customizes() {
    let h = AppHarness::new("auth_eject_html", BaseProfile::WithAuthHtml);

    // Eject the built-in controllers + views into the app.
    h.generate(&["generate", "auth:controllers"]);

    let sessions = h.app.join("app/controllers/auth/sessions_controller.rs");
    let view = h.app.join("app/views/auth/sign_in.html.tera");
    assert!(sessions.is_file(), "auth:controllers should eject the sessions controller");
    assert!(view.is_file(), "auth:controllers should eject the sign-in view");

    let routes = fs::read_to_string(h.app.join("config/routes.rs")).unwrap();
    assert!(
        routes.contains("controllers: {"),
        "routes should be rewired to the local controllers"
    );
    assert!(
        routes.contains("sessions: auth::SessionsController"),
        "routes should reference the ejected SessionsController"
    );

    // Customize the ejected view; the customization must be what's served.
    let original = fs::read_to_string(&view).unwrap();
    fs::write(
        &view,
        original.replace("<h1>Sign in</h1>", &format!("<h1>Sign in</h1><p>{EJECT_MARKER}</p>")),
    )
    .unwrap();

    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "users");
            h.seed_database();
        },
        |app| {
            let page = http::get_text(&format!("{}/users/sign_in", app.base_url));
            assert!(
                page.contains(EJECT_MARKER),
                "customized ejected view should be served, got: {page}"
            );
        },
    );
}
