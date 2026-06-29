use crate::generator::{GeneratedFile, Generator};
use crate::templates::builtin_templates;
use doido_core::{anyhow::anyhow, Result};

/// `doido generate templates [name]` — ejects the built-in default templates
/// into the project's `templates/` directory so they can be customized. With a
/// generator name, only that generator's templates are ejected.
pub struct TemplatesGenerator;

impl Generator for TemplatesGenerator {
    fn name(&self) -> &str {
        "templates"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let prefix = match args.first().copied() {
            None => None,
            Some(name) => Some(prefix_for(name)?),
        };

        let files: Vec<GeneratedFile> = builtin_templates()
            .iter()
            .filter(|(rel, _)| prefix.is_none_or(|p| rel.starts_with(p)))
            .map(|(rel, content)| GeneratedFile {
                path: format!("templates/{rel}"),
                content: (*content).to_string(),
            })
            .collect();

        Ok(files)
    }
}

/// Maps a built-in generator name to the `templates/` prefix holding its files.
fn prefix_for(name: &str) -> Result<&'static str> {
    match name {
        "controller" => Ok("controller/"),
        "job" => Ok("job/"),
        "mailer" => Ok("mailer/"),
        "channel" => Ok("channel/"),
        "migration" => Ok("migration/"),
        "model" => Ok("models/"),
        "scaffold" => Ok("scaffold/"),
        other => Err(anyhow!(
            "unknown generator '{other}' (valid: controller, job, mailer, channel, migration, model, scaffold)"
        )),
    }
}
