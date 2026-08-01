use crate::generator::{GeneratedFile, Generator};
use crate::generators::{register_module, to_pascal, to_snake};
use doido_core::Result;

/// Fallback `app/mailers/mod.rs` when the app doesn't have one on disk yet.
const MAILERS_MOD_BASE: &str = include_str!("../../templates/new/app/mailers/mod.rs");
const MAILERS_MOD_PATH: &str = "app/mailers/mod.rs";

pub struct MailerGenerator;

/// `Welcome` → `WelcomeMailer` / `welcome_mailer`; `UserMailer` stays as-is.
fn mailer_type_name(name: &str) -> String {
    let pascal = to_pascal(name);
    if pascal.ends_with("Mailer") {
        pascal
    } else {
        format!("{pascal}Mailer")
    }
}

/// Module/file stem: `welcome_mailer`, not `user_mailer_mailer`.
fn mailer_module_snake(name: &str) -> String {
    let snake = to_snake(name);
    if snake.ends_with("_mailer") {
        snake
    } else {
        format!("{snake}_mailer")
    }
}

impl Generator for MailerGenerator {
    fn name(&self) -> &str {
        "mailer"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("mailer generator requires a name argument")
        })?;
        let snake = mailer_module_snake(name);
        let pascal = mailer_type_name(name);
        let content =
            crate::templates::get("mailer/mailer.rs.template").replace("{pascal}", &pascal);
        let test = crate::templates::get("mailer/mailer_test.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{snake}", &snake);

        // Register the mailer's module in app/mailers/mod.rs.
        let existing = std::fs::read_to_string(MAILERS_MOD_PATH)
            .unwrap_or_else(|_| MAILERS_MOD_BASE.to_string());
        let mailers_mod = register_module(&existing, &snake, "@generated-mailers");

        Ok(vec![
            GeneratedFile {
                path: format!("app/mailers/{snake}.rs"),
                content,
            },
            GeneratedFile {
                path: MAILERS_MOD_PATH.to_string(),
                content: mailers_mod,
            },
            GeneratedFile {
                path: format!("tests/{snake}_test.rs"),
                content: test,
            },
        ])
    }
}
