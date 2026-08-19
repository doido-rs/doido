//! Smoke scenarios for generators without a dedicated HTTP surface.

use crate::common::cli;
use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use std::path::Path;

fn assert_file(app: &Path, rel: &str) {
    assert!(app.join(rel).is_file(), "expected generated file `{rel}`");
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn job_generator_app_boots() {
    let h = AppHarness::new("job", BaseProfile::Default);
    h.generate(&["generate", "job", "SendEmail"]);
    h.run_with_db(
        |h| assert_file(&h.app, "app/jobs/send_email_job.rs"),
        |app| assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200),
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn mailer_generator_app_boots() {
    let h = AppHarness::new("mailer", BaseProfile::Default);
    h.generate(&["generate", "mailer", "UserMailer"]);
    h.run_with_db(
        |h| assert_file(&h.app, "app/mailers/user_mailer.rs"),
        |app| assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200),
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn locale_generator_writes_file() {
    let h = AppHarness::new("locale", BaseProfile::Default);
    h.generate(&["generate", "locale", "pt"]);
    h.run_with_db(
        |h| assert_file(&h.app, "config/locales/pt.yml"),
        |app| assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200),
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn templates_generator_ejects_files() {
    let h = AppHarness::new("templates", BaseProfile::Default);
    h.generate(&["generate", "templates", "scaffold"]);
    h.run_with_db(
        |h| assert_file(&h.app, "templates/scaffold/views/index.html.tera"),
        |app| assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200),
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn generator_generator_scaffolds_and_registers_generator() {
    let h = AppHarness::new("generator_meta", BaseProfile::Default);
    // Scaffold an app-owned generator: writes app/generators/greeter.rs, updates
    // app/generators/mod.rs, and registers it on the Doido builder in main.rs.
    h.generate(&["generate", "generator", "greeter"]);
    h.run_with_db(
        |h| {
            assert_file(&h.app, "app/generators/greeter.rs");
            // App is rebuilt with the generator compiled + registered, so the
            // app's own binary now dispatches `greeter`.
            cli::run_app(&h.bin(), &h.app, &["generate", "greeter", "Hello"]);
            assert_file(&h.app, "app/greeters/hello.rs");
        },
        |app| assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200),
    );
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn controller_generator_does_not_break_app() {
    let h = AppHarness::new("controller", BaseProfile::Default);
    h.generate(&["generate", "controller", "Pages"]);
    h.run(|app| assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200));
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn storage_adapter_generator_compiles() {
    let h = AppHarness::new("storage_adapter", BaseProfile::Default);
    h.generate(&["generate", "storage:adapter", "Cloud"]);
    h.run_with_db(
        |h| assert_file(&h.app, "app/storage/cloud_service.rs"),
        |app| assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200),
    );
}
