//! Initial-user seed: an app generated with `doido new --auth` ships a
//! `db/seed` that inserts an admin user (admin@example.com / password). This
//! scenario proves the seeded user is a *real, usable* account:
//!
//! - the seed compiles `app/models/user.rs` and inserts the row (idempotently),
//! - a brand-new user can still register (sign-up),
//! - the seeded credentials sign in (access) and a wrong password is rejected
//!   (credential verification),
//! - and the seeded user's session unlocks a protected route.

use crate::common::{db, http, AppHarness, BaseProfile};
use serde_json::json;
use std::fs;
use std::path::Path;

/// A session-guarded controller: 200 for a signed-in user, 401 otherwise.
const DASHBOARD_CONTROLLER: &str = r#"use doido::controller::{controller, Response};
use serde_json::json;

pub struct DashboardController;

#[controller]
impl DashboardController {
    /// GET /dashboard — only reachable with a signed-in session.
    pub async fn index(mut ctx: doido::controller::Context) -> Response {
        let user_id = ctx.session().get::<i64>("user_id");
        match user_id {
            Some(id) => ctx.json(json!({ "user_id": id })),
            None => ctx.status(401),
        }
    }
}
"#;

/// Adds a session-guarded `GET /dashboard` route so we can exercise authenticated
/// access with the seeded user's session.
fn add_protected_dashboard(app: &Path) {
    fs::write(
        app.join("app/controllers/dashboard_controller.rs"),
        DASHBOARD_CONTROLLER,
    )
    .expect("write dashboard controller");

    let mod_path = app.join("app/controllers/mod.rs");
    let mut mods = fs::read_to_string(&mod_path).expect("read controllers mod");
    mods.push_str(
        "\nmod dashboard_controller;\npub use dashboard_controller::DashboardController;\n",
    );
    fs::write(&mod_path, mods).expect("write controllers mod");

    let routes_path = app.join("config/routes.rs");
    let routes = fs::read_to_string(&routes_path).expect("read routes");
    let routes = routes.replace(
        "use crate::controllers::HelloController;",
        "use crate::controllers::HelloController;\nuse crate::controllers::DashboardController;",
    );
    let routes = routes.replace(
        "get!(\"/\", HelloController::index);",
        "get!(\"/\", HelloController::index);\n        get!(\"/dashboard\", DashboardController::index);",
    );
    fs::write(&routes_path, routes).expect("write routes");
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn seed_creates_initial_user() {
    let h = AppHarness::new("seed_initial_user", BaseProfile::WithAuthApi);
    add_protected_dashboard(&h.app);

    h.run_with_db(
        |h| {
            db::assert_table_exists(&h.app, "users");

            // The generated seed inserts the initial user.
            h.seed_database();
            db::assert_row_count(&h.app, "users", 1);
            db::assert_row_exists(&h.app, "users", "email", "admin@example.com");

            // Idempotent: re-running the seed does not duplicate the user.
            h.seed_database();
            db::assert_row_count(&h.app, "users", 1);
        },
        |app| {
            let base = &app.base_url;

            // 1. A brand-new user can register.
            let sign_up = http::post_json_with_response(
                &format!("{base}/users/sign_up"),
                json!({
                    "email": "newuser@example.com",
                    "password": "secret123",
                    "password_confirmation": "secret123"
                }),
            );
            assert_eq!(sign_up.status, 200, "sign up should succeed");

            // 2. The protected route rejects anonymous access.
            assert_eq!(
                http::get_status_any(&format!("{base}/dashboard")),
                401,
                "protected route should reject anonymous access"
            );

            // 3. The seeded admin signs in with the seeded credentials (access
            //    + correct-password verification).
            let sign_in = http::post_json_status_any(
                &format!("{base}/users/sign_in"),
                json!({ "email": "admin@example.com", "password": "password" }),
            );
            assert_eq!(sign_in.status, 200, "seeded admin should sign in");
            let cookie = http::session_cookie(&sign_in.set_cookie);
            assert!(
                cookie.contains("_doido_session"),
                "sign in should establish a session"
            );

            // 4. A wrong password for the seeded admin is rejected (verification).
            let bad = http::post_json_status_any(
                &format!("{base}/users/sign_in"),
                json!({ "email": "admin@example.com", "password": "wrong-password" }),
            );
            assert_eq!(bad.status, 401, "wrong password should be rejected");

            // 5. With the seeded admin's session, the protected route grants access.
            assert_eq!(
                http::get_status_with_cookie(&format!("{base}/dashboard"), &cookie),
                200,
                "protected route should allow the signed-in seeded user"
            );
        },
    );
}
