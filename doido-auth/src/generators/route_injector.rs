//! Injects explicit auth routes understood by the `routes!` macro.

const ROUTES_BASE: &str = include_str!("../../templates/new/config/routes.rs");
const CONTROLLERS_MOD_BASE: &str = include_str!("../../templates/new/app/controllers/mod.rs");
const MODELS_MOD_BASE: &str = include_str!("../../templates/new/app/models/mod.rs");

pub const ROUTES_PATH: &str = "config/routes.rs";
pub const CONTROLLERS_MOD_PATH: &str = "app/controllers/mod.rs";
pub const MODELS_MOD_PATH: &str = "app/models/mod.rs";

/// Injects session/registration/password/OAuth routes for generated auth controllers.
pub fn inject_auth_routes(routes: &str, api: bool) -> String {
    if routes.contains("SessionsController::create") {
        return routes.to_string();
    }

    let mut lines_to_add = vec![
        "post!(\"/users/sign_in\", auth::SessionsController::create);",
        "delete!(\"/users/sign_out\", auth::SessionsController::destroy);",
        "post!(\"/users/sign_up\", auth::RegistrationsController::create);",
        "post!(\"/users/password\", auth::PasswordsController::create);",
        "patch!(\"/users/password\", auth::PasswordsController::update);",
        "get!(\"/auth/{provider}\", auth::OauthController::authorize);",
        "get!(\"/auth/{provider}/callback\", auth::OauthController::callback);",
    ];
    if !api {
        lines_to_add.insert(
            0,
            "get!(\"/users/sign_up\", auth::RegistrationsController::new);",
        );
        lines_to_add.insert(
            0,
            "get!(\"/users/sign_in\", auth::SessionsController::new);",
        );
    }

    let mut lines: Vec<String> = routes.lines().map(String::from).collect();

    let use_auth = "use crate::controllers::auth;";
    if !routes.contains(use_auth) {
        let pos = lines
            .iter()
            .rposition(|l| l.starts_with("use "))
            .map(|i| i + 1)
            .unwrap_or(0);
        lines.insert(pos, use_auth.to_string());
    }

    if let Some(open) = lines.iter().position(|l| {
        let t = l.trim();
        t.starts_with("routes!") && t.contains('{')
    }) {
        if let Some(close_rel) = lines[open..].iter().position(|l| l.trim() == "}") {
            let close = open + close_rel;
            for route in &lines_to_add {
                lines.insert(close, format!("        {route}"));
            }
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Injects `resources!(…)` for a scaffold/resource. Idempotent.
pub fn inject_resources(routes: &str, plural: &str, controller: &str, api: bool) -> String {
    let resources = if api {
        format!("resources!({plural}, {controller}, except: [new, edit]);")
    } else {
        format!("resources!({plural}, {controller});")
    };
    if routes.contains(&resources) {
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

    if let Some(open) = lines.iter().position(|l| {
        let t = l.trim();
        t.starts_with("routes!") && t.contains('{')
    }) {
        if let Some(close_rel) = lines[open..].iter().position(|l| l.trim() == "}") {
            let close = open + close_rel;
            lines.insert(close, format!("        {resources}"));
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Injects custom GET routes for named controller actions.
pub fn inject_action_routes(
    routes: &str,
    snake: &str,
    controller: &str,
    actions: &[&str],
) -> String {
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

    if let Some(open) = lines.iter().position(|l| {
        let t = l.trim();
        t.starts_with("routes!") && t.contains('{')
    }) {
        if let Some(close_rel) = lines[open..].iter().position(|l| l.trim() == "}") {
            let close = open + close_rel;
            for action in actions {
                let path = format!("/{snake}/{action}");
                let route = format!("get!(\"{path}\", {controller}::{action});");
                if !routes.contains(&route) {
                    lines.insert(close, format!("        {route}"));
                }
            }
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Appends `mod <name>_controller;` + `pub use …` to `app/controllers/mod.rs`.
pub fn register_controller(controllers_mod: &str, plural: &str, controller: &str) -> String {
    let module = format!("{plural}_controller");
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

/// Registers the `auth` controller submodule in `app/controllers/mod.rs`.
pub fn register_auth_controllers_mod(controllers_mod: &str) -> String {
    let decl = "pub mod auth;";
    if controllers_mod.contains(decl) {
        return controllers_mod.to_string();
    }
    let mut out = controllers_mod.trim_end().to_string();
    out.push('\n');
    out.push_str(&format!("{decl}\n"));
    out
}

/// Inserts `pub mod <module>;` into `app/models/mod.rs` above the marker.
pub fn register_model_module(models_mod: &str, module: &str) -> String {
    super::register_module(models_mod, module, "@generated-models")
}

pub fn read_routes() -> String {
    std::fs::read_to_string(ROUTES_PATH).unwrap_or_else(|_| ROUTES_BASE.to_string())
}

pub fn read_controllers_mod() -> String {
    std::fs::read_to_string(CONTROLLERS_MOD_PATH)
        .unwrap_or_else(|_| CONTROLLERS_MOD_BASE.to_string())
}

pub fn read_models_mod() -> String {
    std::fs::read_to_string(MODELS_MOD_PATH).unwrap_or_else(|_| MODELS_MOD_BASE.to_string())
}
