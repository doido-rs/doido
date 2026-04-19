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
        Ok(vec![GeneratedFile {
            path: format!("app/mailers/{}_mailer.rs", snake),
            content: format!(
                "use doido_mailer::{{mailer, Mail}};\n\n#[mailer]\npub struct {pascal}Mailer;\n\nimpl {pascal}Mailer {{\n    pub fn welcome(to: &str) -> Mail {{\n        Mail::new().to(to).subject(\"Welcome!\").body_text(\"Welcome to the platform.\")\n    }}\n}}\n",
            ),
        }])
    }
}
