//! Injects `auth_routes!(User);` understood by the `routes!` macro.

const ROUTES_BASE: &str = include_str!("../../templates/new/config/routes.rs");
const CONTROLLERS_MOD_BASE: &str = include_str!("../../templates/new/app/controllers/mod.rs");
const MODELS_MOD_BASE: &str = include_str!("../../templates/new/app/models/mod.rs");

pub const ROUTES_PATH: &str = "config/routes.rs";
pub const CONTROLLERS_MOD_PATH: &str = "app/controllers/mod.rs";
pub const MODELS_MOD_PATH: &str = "app/models/mod.rs";

fn is_routes_block_open(line: &str) -> bool {
    let t = line.trim();
    (t.starts_with("routes!") || t.starts_with("doido::auth::routes!")) && t.contains('{')
}

/// Import that brings the app's `User` model type into scope for a bare
/// `auth_routes!(User);`, which expands to `AuthSessions::<User>` etc.
const USER_IMPORT: &str = "use crate::models::user::Model as User;";
/// Import that brings the app's ejected `auth` controllers module into scope.
const AUTH_MOD_IMPORT: &str = "use crate::controllers::auth;";

/// Injects the framework's built-in auth routes: switches the app's plain
/// `routes!` block to the auth-aware `doido::auth::routes!` variant, brings the
/// `User` model into scope, and adds a bare `auth_routes!(User);` that targets
/// doido-auth's **built-in** controllers (nothing copied into the app). Idempotent.
pub fn inject_auth_routes(routes: &str) -> String {
    if routes.contains("auth_routes!(User") {
        return routes.to_string();
    }
    ensure_auth_routes_block(routes, "auth_routes!(User);", &[USER_IMPORT])
}

/// Like [`inject_auth_routes`] but restricts the mounted routes to the given
/// route groups (`auth_routes!(User, only: [sessions, registrations, …]);`).
/// Used when an explicit module set is chosen at install time. Idempotent.
pub fn inject_auth_routes_only(routes: &str, groups: &[&str]) -> String {
    if routes.contains("auth_routes!(User") {
        return routes.to_string();
    }
    let line = format!("auth_routes!(User, only: [{}]);", groups.join(", "));
    ensure_auth_routes_block(routes, &line, &[USER_IMPORT])
}

/// Builds the `auth_routes!` line that points every enabled route module at the
/// app's local (ejected) controllers. When all modules are overridden the macro
/// never references `User`, so the `User` import is dropped by the caller.
fn local_controllers_line(two_factor: bool) -> String {
    let mut overrides = vec![
        "sessions: auth::SessionsController",
        "registrations: auth::RegistrationsController",
        "passwords: auth::PasswordsController",
        "oauth: auth::OauthController",
    ];
    if two_factor {
        overrides.push("two_factor: auth::TwoFactorController");
    }
    format!("auth_routes!(User, controllers: {{ {} }});", overrides.join(", "))
}

/// Rewrites an installed bare `auth_routes!(User);` to reference the app's local
/// (ejected) auth controllers, adding `use crate::controllers::auth;`. The
/// `User` import stays: modules that aren't overridden (e.g. `confirmation`) still
/// expand to built-in `AuthXxx::<User>` handlers. Used by the `auth:controllers`
/// generator. Idempotent: if a `controllers:` override is already present, the
/// input is returned unchanged.
pub fn rewire_local_controllers(routes: &str, two_factor: bool) -> String {
    if routes.contains("controllers: {") {
        return routes.to_string();
    }
    let line = local_controllers_line(two_factor);
    // Common case: `auth:install` already left a bare `auth_routes!(User);`.
    if routes.contains("auth_routes!(User);") {
        let replaced = routes.replacen("auth_routes!(User);", &line, 1);
        return ensure_use(&replaced, AUTH_MOD_IMPORT);
    }
    // No auth routes yet — inject the block directly with local controllers.
    ensure_auth_routes_block(routes, &line, &[USER_IMPORT, AUTH_MOD_IMPORT])
}

/// Switches the app's `routes!` block to `doido::auth::routes!`, drops the now
/// unused `routes` import, adds each `import` (if missing), and inserts
/// `auth_line` before the block's closing brace.
fn ensure_auth_routes_block(routes: &str, auth_line: &str, imports: &[&str]) -> String {
    let auth_block = "    doido::auth::routes! {";
    let mut lines: Vec<String> = routes.lines().map(String::from).collect();

    if routes.contains("routes! {") && !routes.contains("doido::auth::routes!") {
        for line in &mut lines {
            if line.trim().starts_with("routes! {") {
                *line = auth_block.to_string();
            }
        }
    }

    if routes.contains("use doido::controller::{axum, routes}") {
        for line in &mut lines {
            if line.contains("use doido::controller::{axum, routes}") {
                *line = "use doido::controller::axum;".to_string();
            }
        }
    }

    for import in imports {
        insert_use(&mut lines, import);
    }

    if let Some(open) = lines.iter().position(|l| is_routes_block_open(l)) {
        if let Some(close_rel) = lines[open..].iter().position(|l| l.trim() == "}") {
            let close = open + close_rel;
            lines.insert(close, format!("        {auth_line}"));
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Ensures `import` is present, returning the (possibly modified) source.
fn ensure_use(routes: &str, import: &str) -> String {
    if routes.contains(import) {
        return routes.to_string();
    }
    let mut lines: Vec<String> = routes.lines().map(String::from).collect();
    insert_use(&mut lines, import);
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Inserts `import` after the last `use crate::...` line (or the last `use` line,
/// or at the top) when it is not already present.
fn insert_use(lines: &mut Vec<String>, import: &str) {
    if lines.iter().any(|l| l.trim() == import) {
        return;
    }
    let pos = lines
        .iter()
        .rposition(|l| l.contains("use crate::"))
        .map(|i| i + 1)
        .or_else(|| {
            lines
                .iter()
                .rposition(|l| l.trim_start().starts_with("use "))
                .map(|i| i + 1)
        })
        .unwrap_or(0);
    lines.insert(pos, import.to_string());
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

    if let Some(open) = lines.iter().position(|l| is_routes_block_open(l)) {
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

    if let Some(open) = lines.iter().position(|l| is_routes_block_open(l)) {
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
