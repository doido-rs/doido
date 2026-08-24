//! Development error pages over a real generated app (Rails DebugExceptions analogue).

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use std::fs;
use std::path::Path;

fn wire_errors_controller(app: &Path) {
    fs::write(
        app.join("app/controllers/errors_controller.rs"),
        r#"use doido::controller::controller;

pub struct ErrorsController;

#[controller]
impl ErrorsController {
    pub async fn boom(ctx: doido::controller::Context) -> doido::controller::Response {
        let _ = ctx;
        panic!("e2e intentional failure");
    }
}
"#,
    )
    .expect("write errors_controller.rs");

    fs::write(
        app.join("app/controllers/mod.rs"),
        r#"mod errors_controller;
mod hello_controller;

pub use errors_controller::ErrorsController;
pub use hello_controller::HelloController;
"#,
    )
    .expect("write controllers/mod.rs");

    fs::write(
        app.join("config/routes.rs"),
        r#"use crate::controllers::{ErrorsController, HelloController};
use doido::controller::{axum, routes};

pub fn router() -> axum::Router {
    routes! {
        get!("/", HelloController::index);
        get!("/dev/error", ErrorsController::boom);
    }
}
"#,
    )
    .expect("write config/routes.rs");
}

fn assert_development_error_page(status: u16, body: &str) {
    assert_eq!(status, 500, "expected 500 from action error");
    assert!(
        body.contains("<!DOCTYPE html>"),
        "development errors should render HTML, got: {}",
        &body[..body.len().min(200)]
    );
    assert!(body.contains("500"), "page should show status code");
    assert!(
        body.contains("Internal Server Error"),
        "page should show status title"
    );
    assert!(
        body.contains("e2e intentional failure") || body.contains("panic:"),
        "page should include panic message"
    );
    assert!(
        body.contains("Backtrace"),
        "page should include backtrace section"
    );
    assert!(body.contains("doido"), "page should use framework branding");
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn html_app_shows_development_error_page_for_404_and_500() {
    let h = AppHarness::new("development_errors_html", BaseProfile::Default);
    wire_errors_controller(&h.app);

    h.run_with_db(
        |_| {},
        |app| {
            let (status, body) = http::get_body_any(
                &format!("{}/no-such-route", app.base_url),
                Some("text/html"),
            );
            assert_eq!(status, 404);
            assert!(
                body.contains("<!DOCTYPE html>"),
                "404 should render HTML in development"
            );
            assert!(body.contains("404"));
            assert!(body.contains("GET /no-such-route"));
            assert!(body.contains("doido"));

            let (status, body) =
                http::get_body_any(&format!("{}/dev/error", app.base_url), Some("text/html"));
            assert_development_error_page(status, &body);

            let (status, body) = http::get_body_any(
                &format!("{}/dev/error", app.base_url),
                Some("application/json"),
            );
            assert_eq!(status, 500);
            assert_eq!(body, "Internal Server Error");
            assert!(
                !body.contains("<!DOCTYPE html>"),
                "JSON clients should not receive the HTML diagnostic page"
            );
        },
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn api_only_skips_development_error_page() {
    let h = AppHarness::new("development_errors_api", BaseProfile::ApiOnly);
    wire_errors_controller(&h.app);

    h.run_with_db(
        |_| {},
        |app| {
            let (status, body) = http::get_body_any(
                &format!("{}/no-such-route", app.base_url),
                Some("text/html"),
            );
            assert_eq!(status, 404);
            assert!(
                !body.contains("<!DOCTYPE html>"),
                "api_only must skip the HTML diagnostic page"
            );

            let (status, body) =
                http::get_body_any(&format!("{}/dev/error", app.base_url), Some("text/html"));
            assert_eq!(status, 500);
            assert!(
                !body.contains("<!DOCTYPE html>"),
                "api_only must skip the HTML diagnostic page on panics too"
            );
        },
    );
}
