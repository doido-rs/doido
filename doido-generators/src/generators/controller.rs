use crate::generator::{GeneratedFile, Generator};
use crate::generators::helper::{
    helper_names, helpers_mod_path, read_helpers_mod, register_helper_in_mod, render_helper_content,
};
use crate::generators::{to_pascal, to_snake};
use doido_core::Result;

/// Fallback `app/controllers/mod.rs` when the app doesn't have one on disk yet.
const CONTROLLERS_MOD_BASE: &str = include_str!("../../templates/new/app/controllers/mod.rs");
pub(crate) const CONTROLLERS_MOD_PATH: &str = "app/controllers/mod.rs";

/// Fallback `config/routes.rs` when the app doesn't have one on disk yet.
const ROUTES_BASE: &str = include_str!("../../templates/new/config/routes.rs");
const ROUTES_PATH: &str = "config/routes.rs";

pub struct ControllerGenerator;

pub(crate) fn read_routes() -> String {
    std::fs::read_to_string(ROUTES_PATH).unwrap_or_else(|_| ROUTES_BASE.to_string())
}

/// Injects `use crate::controllers::<Controller>;` and a `get!(…)` route into
/// `config/routes.rs`, preserving existing routes. Idempotent on the route line.
pub(crate) fn inject_controller_route(routes: &str, snake: &str, controller: &str) -> String {
    let route = format!("get!(\"/{snake}\", {controller}::index);");
    if routes.contains(&route) {
        return routes.to_string();
    }

    let use_line = format!("use crate::controllers::{controller};");
    let mut lines: Vec<String> = routes.lines().map(String::from).collect();

    if !routes.contains(&use_line) {
        let pos = lines
            .iter()
            .rposition(|l| l.contains("use crate::controllers"))
            .map(|i| i + 1)
            .unwrap_or(0);
        lines.insert(pos, use_line);
    }

    if let Some(open) = lines.iter().position(|l| l.contains("routes!")) {
        if let Some(close_rel) = lines[open..].iter().position(|l| l.trim() == "}") {
            let close = open + close_rel;
            lines.insert(close, format!("        {route}"));
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

pub(crate) fn read_controllers_mod() -> String {
    std::fs::read_to_string(CONTROLLERS_MOD_PATH)
        .unwrap_or_else(|_| CONTROLLERS_MOD_BASE.to_string())
}

/// Appends `mod <stem>_controller;` + `pub use …` to `app/controllers/mod.rs`.
/// Idempotent: skips when the module is already declared.
pub(crate) fn register_controller_in_mod(
    controllers_mod: &str,
    module_stem: &str,
    controller: &str,
) -> String {
    let module = format!("{module_stem}_controller");
    let decl = format!("mod {module};");
    if controllers_mod.lines().any(|l| l.trim() == decl) {
        return controllers_mod.to_string();
    }
    let mut out = controllers_mod.trim_end().to_string();
    out.push('\n');
    out.push_str(&format!("mod {module};\n"));
    out.push_str(&format!("pub use {module}::{controller};\n"));
    out
}

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
        let controller_type = format!("{pascal}Controller");
        let controllers_mod =
            register_controller_in_mod(&read_controllers_mod(), &snake, &controller_type);
        let routes = inject_controller_route(&read_routes(), &snake, &controller_type);
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
                path: format!("app/controllers/{snake}_controller.rs"),
                content,
            },
            GeneratedFile {
                path: CONTROLLERS_MOD_PATH.to_string(),
                content: controllers_mod,
            },
            GeneratedFile {
                path: ROUTES_PATH.to_string(),
                content: routes,
            },
            GeneratedFile {
                path: format!("tests/{snake}_controller_test.rs"),
                content: test,
            },
        ])
    }
}
