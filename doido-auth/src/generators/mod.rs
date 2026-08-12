//! Shared helpers for auth generators — migration rendering, name inflection,
//! and route/module injection.

pub mod field;
pub mod migration_support;
pub mod names;
pub mod route_injector;

pub use field::Field;
pub use names::{to_pascal, to_snake, to_table_name};

pub mod controller;
pub mod install;
pub mod scaffold;

pub use controller::AuthControllerGenerator;
pub use install::AuthInstallGenerator;
pub use scaffold::AuthScaffoldGenerator;

use doido_core::Result;

/// A file emitted by an auth generator.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// Auth generator contract — mirrors `doido_generators::Generator` without a
/// crate dependency (avoids circular deps with `doido-generators`).
pub trait AuthGenerator: Send + Sync {
    fn name(&self) -> &str;
    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>>;
}

/// Implemented by `doido_generators::GeneratorRegistry` to merge auth generators.
pub trait AuthGeneratorRegistry {
    fn register_auth(&mut self, generator: Box<dyn AuthGenerator>);
}

/// Register all auth generators into `reg`.
pub fn register(reg: &mut impl AuthGeneratorRegistry) {
    reg.register_auth(Box::new(install::AuthInstallGenerator));
    reg.register_auth(Box::new(controller::AuthControllerGenerator));
    reg.register_auth(Box::new(scaffold::AuthScaffoldGenerator));
}

/// Load an embedded template from `doido-auth/templates/`.
pub(crate) fn template(rel: &str) -> &'static str {
    match rel {
        "migration.rs.template" => include_str!("../../templates/migration.rs.template"),
        "user.rs.template" => include_str!("../../templates/user.rs.template"),
        "user_entity.rs.template" => include_str!("../../templates/user_entity.rs.template"),
        "auth/mod.rs.template" => include_str!("../../templates/auth/mod.rs.template"),
        "auth/sessions_controller_html.rs.template" => {
            include_str!("../../templates/auth/sessions_controller_html.rs.template")
        }
        "auth/sessions_controller_api.rs.template" => {
            include_str!("../../templates/auth/sessions_controller_api.rs.template")
        }
        "auth/registrations_controller_html.rs.template" => {
            include_str!("../../templates/auth/registrations_controller_html.rs.template")
        }
        "auth/registrations_controller_api.rs.template" => {
            include_str!("../../templates/auth/registrations_controller_api.rs.template")
        }
        "auth/passwords_controller_html.rs.template" => {
            include_str!("../../templates/auth/passwords_controller_html.rs.template")
        }
        "auth/passwords_controller_api.rs.template" => {
            include_str!("../../templates/auth/passwords_controller_api.rs.template")
        }
        "auth/oauth_controller.rs.template" => {
            include_str!("../../templates/auth/oauth_controller.rs.template")
        }
        "auth/two_factor_controller_html.rs.template" => {
            include_str!("../../templates/auth/two_factor_controller_html.rs.template")
        }
        "auth/two_factor_controller_api.rs.template" => {
            include_str!("../../templates/auth/two_factor_controller_api.rs.template")
        }
        "auth/views/sign_in.html.tera" => {
            include_str!("../../templates/auth/views/sign_in.html.tera")
        }
        "auth/views/sign_up.html.tera" => {
            include_str!("../../templates/auth/views/sign_up.html.tera")
        }
        "auth/views/password_new.html.tera" => {
            include_str!("../../templates/auth/views/password_new.html.tera")
        }
        "auth/views/password_edit.html.tera" => {
            include_str!("../../templates/auth/views/password_edit.html.tera")
        }
        "auth/views/two_factor.html.tera" => {
            include_str!("../../templates/auth/views/two_factor.html.tera")
        }
        "auth_controller.rs.template" => {
            include_str!("../../templates/auth_controller.rs.template")
        }
        "scaffold/controller_html.rs.template" => {
            include_str!("../../templates/scaffold/controller_html.rs.template")
        }
        "scaffold/controller_api.rs.template" => {
            include_str!("../../templates/scaffold/controller_api.rs.template")
        }
        "scaffold/model.rs.template" => include_str!("../../templates/scaffold/model.rs.template"),
        "scaffold/entity.rs.template" => {
            include_str!("../../templates/scaffold/entity.rs.template")
        }
        "scaffold/views/index.html.tera" => {
            include_str!("../../templates/scaffold/views/index.html.tera")
        }
        "scaffold/views/show.html.tera" => {
            include_str!("../../templates/scaffold/views/show.html.tera")
        }
        "scaffold/views/new.html.tera" => {
            include_str!("../../templates/scaffold/views/new.html.tera")
        }
        "scaffold/views/edit.html.tera" => {
            include_str!("../../templates/scaffold/views/edit.html.tera")
        }
        "scaffold/views/_form.html.tera" => {
            include_str!("../../templates/scaffold/views/_form.html.tera")
        }
        other => panic!("unknown auth generator template: {other}"),
    }
}

/// Append `pub mod <module>;` into a directory `mod.rs`, just above `marker`.
pub(crate) fn register_module(existing: &str, module: &str, marker: &str) -> String {
    let decl = format!("pub mod {module};");
    if existing.lines().any(|l| l.trim() == decl) {
        return existing.to_string();
    }
    let mut lines: Vec<String> = existing.lines().map(String::from).collect();
    match lines.iter().position(|l| l.contains(marker)) {
        Some(i) => lines.insert(i, decl),
        None => lines.push(decl),
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
