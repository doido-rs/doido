//! `doido generate auth:controllers` — the `devise:controllers` + `devise:views`
//! analogue.
//!
//! By default, auth works through doido-auth's **built-in** controllers and
//! views (nothing is copied into the app). This generator *ejects* those into
//! the project so they can be customized: it writes
//! `app/controllers/auth/*_controller.rs` + `app/views/auth/*.html.tera`,
//! registers the `auth` controllers module, and rewires `config/routes.rs` to
//! point `auth_routes!` at the local controllers.
//!
//! Flags:
//! - `--api` — controllers only, JSON responses (no HTML views).
//! - `--two-factor` — also eject the 2FA controller/view.
//! - `--controllers-only` — eject controllers + rewire routes, skip views.
//! - `--views-only` — eject only the views (built-in controllers keep serving,
//!   now rendering the app's overriding templates); routes are left unchanged.

use super::route_injector::{
    read_controllers_mod, read_routes, register_auth_controllers_mod, rewire_local_controllers,
    CONTROLLERS_MOD_PATH, ROUTES_PATH,
};
use super::template;
use super::{AuthGenerator, GeneratedFile};
use doido_core::Result;

pub struct AuthControllersGenerator;

fn auth_mod(two_factor: bool) -> String {
    let oauth_module = "mod oauth_controller;\n";
    let oauth_use = "pub use oauth_controller::OauthController;\n";
    let (two_factor_module, two_factor_use) = if two_factor {
        (
            "mod two_factor_controller;\n",
            "pub use two_factor_controller::TwoFactorController;\n",
        )
    } else {
        ("", "")
    };
    template("auth/mod.rs.template")
        .replace("{oauth_module}", oauth_module)
        .replace("{oauth_use}", oauth_use)
        .replace("{two_factor_module}", two_factor_module)
        .replace("{two_factor_use}", two_factor_use)
}

impl AuthGenerator for AuthControllersGenerator {
    fn name(&self) -> &str {
        "auth:controllers"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let api = args.contains(&"--api");
        let two_factor = args.contains(&"--two-factor");
        let controllers_only = args.contains(&"--controllers-only");
        let views_only = args.contains(&"--views-only");

        let emit_controllers = !views_only;
        // API auth has no HTML views to eject.
        let emit_views = !controllers_only && !api;

        let suffix = if api { "api" } else { "html" };
        let mut files = Vec::new();

        if emit_controllers {
            files.push(GeneratedFile {
                path: "app/controllers/auth/mod.rs".to_string(),
                content: auth_mod(two_factor),
            });
            files.push(GeneratedFile {
                path: "app/controllers/auth/sessions_controller.rs".to_string(),
                content: template(&format!("auth/sessions_controller_{suffix}.rs.template"))
                    .to_string(),
            });
            files.push(GeneratedFile {
                path: "app/controllers/auth/registrations_controller.rs".to_string(),
                content: template(&format!(
                    "auth/registrations_controller_{suffix}.rs.template"
                ))
                .to_string(),
            });
            files.push(GeneratedFile {
                path: "app/controllers/auth/passwords_controller.rs".to_string(),
                content: template(&format!("auth/passwords_controller_{suffix}.rs.template"))
                    .to_string(),
            });
            files.push(GeneratedFile {
                path: "app/controllers/auth/oauth_controller.rs".to_string(),
                content: template("auth/oauth_controller.rs.template").to_string(),
            });
            if two_factor {
                files.push(GeneratedFile {
                    path: "app/controllers/auth/two_factor_controller.rs".to_string(),
                    content: template(&format!("auth/two_factor_controller_{suffix}.rs.template"))
                        .to_string(),
                });
            }
            files.push(GeneratedFile {
                path: CONTROLLERS_MOD_PATH.to_string(),
                content: register_auth_controllers_mod(&read_controllers_mod()),
            });
            files.push(GeneratedFile {
                path: ROUTES_PATH.to_string(),
                content: rewire_local_controllers(&read_routes(), two_factor),
            });
        }

        if emit_views {
            for (file, rel) in [
                ("sign_in", "auth/views/sign_in.html.tera"),
                ("sign_up", "auth/views/sign_up.html.tera"),
                ("password_new", "auth/views/password_new.html.tera"),
                ("password_edit", "auth/views/password_edit.html.tera"),
            ] {
                files.push(GeneratedFile {
                    path: format!("app/views/auth/{file}.html.tera"),
                    content: template(rel).to_string(),
                });
            }
            if two_factor {
                files.push(GeneratedFile {
                    path: "app/views/auth/two_factor.html.tera".to_string(),
                    content: template("auth/views/two_factor.html.tera").to_string(),
                });
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ejects_controllers_views_and_rewires_routes() {
        let files = AuthControllersGenerator.generate(&[]).unwrap();
        for path in [
            "app/controllers/auth/mod.rs",
            "app/controllers/auth/sessions_controller.rs",
            "app/controllers/auth/registrations_controller.rs",
            "app/controllers/auth/passwords_controller.rs",
            "app/controllers/auth/oauth_controller.rs",
            "app/views/auth/sign_in.html.tera",
            "app/views/auth/sign_up.html.tera",
        ] {
            assert!(
                files.iter().any(|f| f.path == path),
                "expected ejected file {path}"
            );
        }
        let controllers_mod = files
            .iter()
            .find(|f| f.path == CONTROLLERS_MOD_PATH)
            .unwrap();
        assert!(controllers_mod.content.contains("pub mod auth;"));
    }

    #[test]
    fn rewires_installed_bare_route_to_local_controllers() {
        // Simulate an app that already ran `auth:install`: bare route + User import.
        std::fs::create_dir_all("config").ok();
        std::fs::write(
            ROUTES_PATH,
            "use doido::controller::axum;\nuse crate::models::user::Model as User;\n\npub fn router() -> axum::Router {\n    doido::auth::routes! {\n        auth_routes!(User);\n    }\n}\n",
        )
        .unwrap();

        let files = AuthControllersGenerator.generate(&[]).unwrap();
        let routes = files.iter().find(|f| f.path == ROUTES_PATH).unwrap();
        assert!(routes.content.contains("controllers: {"));
        assert!(routes.content.contains("sessions: auth::SessionsController"));
        assert!(routes.content.contains("use crate::controllers::auth;"));
        assert!(!routes.content.contains("auth_routes!(User);"));
        // With every module overridden, `User` is no longer referenced — its import
        // must be dropped so the ejected routes compile under `-D warnings`.
        assert!(!routes.content.contains("use crate::models::user::Model as User;"));

        let _ = std::fs::remove_file(ROUTES_PATH);
    }

    #[test]
    fn api_flag_skips_views() {
        let files = AuthControllersGenerator.generate(&["--api"]).unwrap();
        assert!(!files.iter().any(|f| f.path.starts_with("app/views/auth/")));
        assert!(files
            .iter()
            .any(|f| f.path == "app/controllers/auth/sessions_controller.rs"));
    }

    #[test]
    fn views_only_skips_controllers_and_routes() {
        let files = AuthControllersGenerator
            .generate(&["--views-only"])
            .unwrap();
        assert!(!files
            .iter()
            .any(|f| f.path.starts_with("app/controllers/auth/")));
        assert!(!files.iter().any(|f| f.path == ROUTES_PATH));
        assert!(files
            .iter()
            .any(|f| f.path == "app/views/auth/sign_in.html.tera"));
    }
}
