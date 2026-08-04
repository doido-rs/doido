//! `doido generate auth:install` — the `devise:install` + `devise User` analogue.
//!
//! Emits a User migration and model, auth controllers, views (HTML mode),
//! config snippets, and injects `auth_routes!(User);` into `config/routes.rs`.
//! Does **not** modify `Cargo.toml`.

use super::migration_support::{
    register_migration, render_migration_file, MIGRATION_LIB_BASE, MIGRATION_SRC_DIR,
};
use super::route_injector::{
    inject_auth_routes, read_controllers_mod, read_models_mod, read_routes,
    register_auth_controllers_mod, register_model_module, CONTROLLERS_MOD_PATH, MODELS_MOD_PATH,
    ROUTES_PATH,
};
use super::template;
use super::{AuthGenerator, GeneratedFile};
use chrono::Utc;
use doido_core::Result;

pub struct AuthInstallGenerator;

const IMPORTS: &str = "use doido::model::migration::{create_table, drop_table};";

fn users_up_body(two_factor: bool) -> String {
    let mut body = String::from(
        "        create_table(manager, \"users\", |t| {\n\
         \x20           t.string(\"email\").not_null().unique_key();\n\
         \x20           t.string(\"password_digest\").not_null();\n",
    );
    if two_factor {
        body.push_str("            t.string(\"two_factor_secret\");\n");
        body.push_str("            t.boolean(\"two_factor_enabled\").not_null();\n");
    }
    body.push_str(
        "            t.timestamp(\"created_at\").not_null();\n\
         \x20           t.timestamp(\"updated_at\").not_null();\n\
         \x20       })\n\
         \x20       .await\n",
    );
    body
}

const DOWN_BODY: &str = "        drop_table(manager, \"users\").await\n";

fn auth_section(two_factor: bool) -> String {
    let enabled = if two_factor { "true" } else { "false" };
    format!(
        "\nauth:\n  user_model: User\n  strategies:\n    - cookie\n  two_factor:\n    enabled: {enabled}\n    issuer: MyApp\n  routes:\n    prefix: /users\n"
    )
}

fn config_file(path: &str, two_factor: bool) -> Option<GeneratedFile> {
    let existing = std::fs::read_to_string(path).ok()?;
    if existing.contains("\nauth:") || existing.starts_with("auth:") {
        return None;
    }
    Some(GeneratedFile {
        path: path.to_string(),
        content: format!("{}{}", existing.trim_end(), auth_section(two_factor)),
    })
}

fn user_model(two_factor: bool) -> String {
    let two_factor_fields = if two_factor {
        "    pub two_factor_secret: Option<String>,\n    pub two_factor_enabled: bool,\n"
    } else {
        ""
    };
    template("user.rs.template").replace("{two_factor_fields}", two_factor_fields)
}

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

impl AuthGenerator for AuthInstallGenerator {
    fn name(&self) -> &str {
        "auth:install"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let api = args.contains(&"--api");
        let two_factor = args.contains(&"--two-factor");

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let migration_module = format!("m{timestamp}_create_users_table");
        let migration = render_migration_file(
            &migration_module,
            IMPORTS,
            &users_up_body(two_factor),
            DOWN_BODY,
        );

        let lib_path = format!("{MIGRATION_SRC_DIR}/lib.rs");
        let existing =
            std::fs::read_to_string(&lib_path).unwrap_or_else(|_| MIGRATION_LIB_BASE.to_string());
        let lib = register_migration(&existing, &migration_module);

        let models_mod = register_model_module(&read_models_mod(), "user");
        let controllers_mod = register_auth_controllers_mod(&read_controllers_mod());
        let routes = inject_auth_routes(&read_routes(), api);

        let suffix = if api { "api" } else { "html" };

        let mut files = vec![
            GeneratedFile {
                path: format!("{MIGRATION_SRC_DIR}/{migration_module}.rs"),
                content: migration,
            },
            GeneratedFile {
                path: lib_path,
                content: lib,
            },
            GeneratedFile {
                path: "app/models/user.rs".to_string(),
                content: user_model(two_factor),
            },
            GeneratedFile {
                path: MODELS_MOD_PATH.to_string(),
                content: models_mod,
            },
            GeneratedFile {
                path: "app/controllers/auth/mod.rs".to_string(),
                content: auth_mod(two_factor),
            },
            GeneratedFile {
                path: "app/controllers/auth/sessions_controller.rs".to_string(),
                content: template(&format!("auth/sessions_controller_{suffix}.rs.template"))
                    .to_string(),
            },
            GeneratedFile {
                path: "app/controllers/auth/registrations_controller.rs".to_string(),
                content: template(&format!(
                    "auth/registrations_controller_{suffix}.rs.template"
                ))
                .to_string(),
            },
            GeneratedFile {
                path: "app/controllers/auth/passwords_controller.rs".to_string(),
                content: template(&format!("auth/passwords_controller_{suffix}.rs.template"))
                    .to_string(),
            },
            GeneratedFile {
                path: "app/controllers/auth/oauth_controller.rs".to_string(),
                content: template("auth/oauth_controller.rs.template").to_string(),
            },
            GeneratedFile {
                path: CONTROLLERS_MOD_PATH.to_string(),
                content: controllers_mod,
            },
            GeneratedFile {
                path: ROUTES_PATH.to_string(),
                content: routes,
            },
        ];

        if two_factor {
            files.push(GeneratedFile {
                path: "app/controllers/auth/two_factor_controller.rs".to_string(),
                content: template(&format!("auth/two_factor_controller_{suffix}.rs.template"))
                    .to_string(),
            });
        }

        if !api {
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

        if let Some(f) = config_file("config/development.yml", two_factor) {
            files.push(f);
        }
        if let Some(f) = config_file("config/test.yml", two_factor) {
            files.push(f);
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_users_migration_and_routes() {
        let files = AuthInstallGenerator.generate(&[]).unwrap();
        let migration = files
            .iter()
            .find(|f| f.path.contains("create_users_table"))
            .expect("users migration");
        assert!(migration.content.contains("password_digest"));
        assert!(migration
            .content
            .contains("impl MigrationName for Migration"));

        let routes = files
            .iter()
            .find(|f| f.path == ROUTES_PATH)
            .expect("routes.rs");
        assert!(routes.content.contains("SessionsController::create"));
        assert!(routes.content.contains("use crate::controllers::auth;"));

        let user = files
            .iter()
            .find(|f| f.path == "app/models/user.rs")
            .expect("user model");
        assert!(user.content.contains("impl AuthUser for Model"));
    }

    #[test]
    fn two_factor_adds_columns_and_controller() {
        let files = AuthInstallGenerator.generate(&["--two-factor"]).unwrap();
        let migration = files
            .iter()
            .find(|f| f.path.contains("create_users_table"))
            .unwrap();
        assert!(migration.content.contains("two_factor_secret"));
        assert!(files
            .iter()
            .any(|f| f.path.ends_with("two_factor_controller.rs")));
    }

    #[test]
    fn api_mode_skips_html_views() {
        let files = AuthInstallGenerator.generate(&["--api"]).unwrap();
        assert!(!files.iter().any(|f| f.path.contains("app/views/auth/")));
        let sessions = files
            .iter()
            .find(|f| f.path.ends_with("sessions_controller.rs"))
            .unwrap();
        assert!(sessions.content.contains("body_json"));
    }
}
