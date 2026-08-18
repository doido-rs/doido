//! App-installed custom generator: a `doido::Generator` defined in the app and
//! installed via the `doido::Doido` builder is dispatched by the app's own CLI
//! (`cargo doido generate <name>`), alongside the framework built-ins.

use crate::common::cli;
use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use std::fs;
use std::path::Path;

fn assert_file(app: &Path, rel: &str) {
    assert!(app.join(rel).is_file(), "expected generated file `{rel}`");
}

/// Rewrite the app's `src/main.rs` to define a generator that lives in the app
/// (not in `doido-generators`) and install it on the `Doido` builder. The module
/// includes match the generated skeleton so the app still compiles and serves.
fn install_greeter_generator(app: &Path) {
    let main_rs = r#"#[path = "../app/controllers/mod.rs"]
mod controllers;

#[path = "../app/models/mod.rs"]
mod models;

#[path = "../app/jobs/mod.rs"]
mod jobs;

#[path = "../app/mailers/mod.rs"]
mod mailers;

#[path = "../app/helpers/mod.rs"]
mod helpers;

#[path = "../config/routes.rs"]
mod routes;

use doido::{GeneratedFile, Generator};

/// A generator defined in the application itself, not in `doido-generators`.
struct GreeterGenerator;

impl Generator for GreeterGenerator {
    fn name(&self) -> &str {
        "greeter"
    }

    fn generate(&self, args: &[&str]) -> doido::core::Result<Vec<GeneratedFile>> {
        let name = args.first().copied().unwrap_or("World");
        Ok(vec![GeneratedFile {
            path: format!("greetings/{}.rs", name.to_lowercase()),
            content: format!("// hello {name}\n"),
        }])
    }
}

#[tokio::main]
async fn main() {
    doido::Doido::new()
        .router(routes::router())
        .register_generator(Box::new(GreeterGenerator))
        .run()
        .await;
}
"#;
    fs::write(app.join("src/main.rs"), main_rs).expect("write app src/main.rs");
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn custom_generator_installed_via_builder() {
    let h = AppHarness::new("custom_generator", BaseProfile::Default);
    install_greeter_generator(&h.app);

    // `run_with_db` builds the app (with the custom generator) first, so the
    // app binary that dispatches `greeter` is the one we invoke below.
    h.run_with_db(
        |h| {
            cli::run_app(&h.bin(), &h.app, &["generate", "greeter", "Hello"]);
            assert_file(&h.app, "greetings/hello.rs");
        },
        |app| assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200),
    );
}
