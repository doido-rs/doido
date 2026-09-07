//! `auth:scaffold` wires `current_user` / `signed_in` into every rendered view
//! via a generated `load_current_user` before_action. A signed-in visitor sees
//! their data on the scaffolded page; anonymous visitors are redirected to
//! sign-in; and the password digest never leaks into the template context.

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use std::fs;

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn current_user_is_available_in_scaffolded_views() {
    let h = AppHarness::new("current_user_view", BaseProfile::WithAuthHtml);

    // Auth-aware resource: the generated controller carries
    // `#[before_action(load_current_user)]` on its rendering actions.
    h.generate(&[
        "generate",
        "auth:scaffold",
        "Note",
        "title:string:not_null",
    ]);

    // Customize the generated index view to show the exposed user. Rendering the
    // whole `current_user` via `json_encode` also proves the password digest is
    // not serialized into the template context. The snippet goes *inside* the
    // content block — `{% extends %}` must stay the first tag. Mirrors what an app
    // author writes.
    let index_view = h.app.join("app/views/notes/index.html.tera");
    let original = fs::read_to_string(&index_view).expect("generated notes index view");
    fs::write(
        &index_view,
        original.replace(
            "{% block content %}",
            "{% block content %}\n{% if signed_in %}<pre id=\"who\">{{ current_user | json_encode() }}</pre>{% endif %}",
        ),
    )
    .unwrap();

    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "notes");
            h.seed_database();
        },
        |app| {
            // Anonymous visitors are redirected to sign-in by `require_user`.
            let anon = http::get_text(&format!("{}/notes", app.base_url));
            assert!(
                anon.contains("Sign in"),
                "anonymous access to /notes should land on sign-in, got: {anon}"
            );

            // Register + sign in to obtain a session cookie.
            http::post_form_with_response(
                &format!("{}/users/sign_up", app.base_url),
                &[
                    ("email", "alice@example.com"),
                    ("password", "secret"),
                    ("password_confirmation", "secret"),
                ],
            );
            let sign_in = http::post_form_with_response(
                &format!("{}/users/sign_in", app.base_url),
                &[("email", "alice@example.com"), ("password", "secret")],
            );
            let cookie = http::session_cookie(&sign_in.set_cookie);
            assert!(!cookie.is_empty(), "sign in should set a session cookie");

            // The signed-in visitor sees `current_user` in the scaffolded view.
            let page = http::get_text_with_cookie(&format!("{}/notes", app.base_url), &cookie);
            assert!(
                page.contains("alice@example.com"),
                "current_user should be available in the view, got: {page}"
            );
            // The password digest must never reach the template context / HTML.
            assert!(
                !page.contains("password_digest") && !page.contains("$2b$"),
                "password digest must not leak into the rendered page, got: {page}"
            );
        },
    );
}
