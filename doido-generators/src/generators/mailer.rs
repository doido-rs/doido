use crate::generator::{GeneratedFile, Generator};
use crate::generators::{to_pascal, to_snake};
use doido_core::Result;

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
        Ok(vec![
            GeneratedFile {
                path: format!("app/mailers/{snake}_mailer.rs"),
                content,
            },
            GeneratedFile {
                path: format!("tests/{snake}_mailer_test.rs"),
                content: test,
            },
        ])
    }
}
