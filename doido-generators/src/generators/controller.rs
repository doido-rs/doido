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
        Ok(vec![GeneratedFile {
            path: format!("src/controllers/{}_controller.rs", snake),
            content: format!(
                "use doido_controller::controller;\n\n#[controller]\npub struct {pascal}Controller;\n\nimpl {pascal}Controller {{\n    pub async fn index(&self) -> &'static str {{\n        \"ok\"\n    }}\n}}\n",
            ),
        }])
    }
}
