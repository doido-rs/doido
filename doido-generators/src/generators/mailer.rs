use crate::generator::{GeneratedFile, Generator};
use crate::generators::{register_module, to_pascal, to_snake};
use doido_core::Result;

/// Fallback `app/mailers/mod.rs` when the app doesn't have one on disk yet.
const MAILERS_MOD_BASE: &str = include_str!("../../templates/new/app/mailers/mod.rs");
const MAILERS_MOD_PATH: &str = "app/mailers/mod.rs";

pub struct MailerGenerator;

impl Generator for MailerGenerator {
    fn name(&self) -> &str {
        "mailer"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("mailer generator requires a name argument")
        })?;
        let snake = to_snake(name);
        let pascal = to_pascal(name);
        let content =
            crate::templates::get("mailer/mailer.rs.template").replace("{pascal}", &pascal);
        let test = crate::templates::get("mailer/mailer_test.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{snake}", &snake);

        // Register the mailer's module in app/mailers/mod.rs.
        let existing = std::fs::read_to_string(MAILERS_MOD_PATH)
            .unwrap_or_else(|_| MAILERS_MOD_BASE.to_string());
        let mailers_mod =
            register_module(&existing, &format!("{snake}_mailer"), "@generated-mailers");

        Ok(vec![
            GeneratedFile {
                path: format!("app/mailers/{snake}_mailer.rs"),
                content,
            },
            GeneratedFile {
                path: MAILERS_MOD_PATH.to_string(),
                content: mailers_mod,
            },
            GeneratedFile {
                path: format!("tests/{snake}_mailer_test.rs"),
                content: test,
            },
        ])
    }
}
