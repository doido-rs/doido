//! `doido generate locale [code]` — a starter i18n locale file
//! (`config/locales/<code>.yml`, default `en`), Rails-style.

use crate::generator::{GeneratedFile, Generator};
use doido_core::Result;

pub struct LocaleGenerator;

impl Generator for LocaleGenerator {
    fn name(&self) -> &str {
        "locale"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let code = args.first().copied().unwrap_or("en");
        let content = format!(
            "{code}:\n  hello: \"Hello world\"\n  # Add translations here, e.g.:\n  # greeting: \"Welcome, %{{name}}!\"\n"
        );
        Ok(vec![GeneratedFile {
            path: format!("config/locales/{code}.yml"),
            content,
        }])
    }
}
