use crate::generator::{GeneratedFile, Generator};
use crate::generators::helper::{
    helper_names, helpers_mod_path, read_helpers_mod, register_helper_in_mod, render_helper_content,
};
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
        let (_, helper_type) = helper_names(name);
        let content = crate::templates::get("controller/controller.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{snake}", &snake)
            .replace("{Helper}", &helper_type);
        let test = crate::templates::get("controller/controller_test.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{snake}", &snake);
        let (helper_snake, _) = helper_names(name);
        let helpers_mod = register_helper_in_mod(&read_helpers_mod(), name);
        Ok(vec![
            GeneratedFile {
                path: format!("app/helpers/{helper_snake}.rs"),
                content: render_helper_content(name, &snake),
            },
            GeneratedFile {
                path: helpers_mod_path().to_string(),
                content: helpers_mod,
            },
            GeneratedFile {
                path: format!("src/controllers/{snake}_controller.rs"),
                content,
            },
            GeneratedFile {
                path: format!("tests/{snake}_controller_test.rs"),
                content: test,
            },
        ])
    }
}
