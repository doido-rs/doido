use crate::generator::{GeneratedFile, Generator};
use crate::generators::{to_pascal, to_snake};
use doido_core::Result;

pub struct ControllerGenerator;

impl Generator for ControllerGenerator {
    fn name(&self) -> &str {
        "controller"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("controller generator requires a name argument")
        })?;
        let snake = to_snake(name);
        let pascal = to_pascal(name);
        let content =
            crate::templates::get("controller/controller.rs.template").replace("{pascal}", &pascal);
        Ok(vec![GeneratedFile {
            path: format!("src/controllers/{}_controller.rs", snake),
            content,
        }])
    }
}
