use crate::generator::{GeneratedFile, Generator};
use crate::generators::{controller::ControllerGenerator, model::ModelGenerator, to_snake};
use doido_core::Result;

pub struct ScaffoldGenerator;

impl Generator for ScaffoldGenerator {
    fn name(&self) -> &str {
        "scaffold"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("scaffold generator requires a name argument")
        })?;

        let mut files = vec![];
        files.extend(ControllerGenerator.generate(args)?);
        // The model generator also emits the migration and updates the migration
        // crate's lib.rs, so no separate migration generator call is needed.
        files.extend(ModelGenerator.generate(args)?);

        // Route injection marker
        let snake = to_snake(name);
        files.push(GeneratedFile {
            path: "config/routes.rs".to_string(),
            content: format!("// ROUTE_INJECTION: resources!({snake});\n"),
        });

        Ok(files)
    }
}
