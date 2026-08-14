//! `doido generate auth:install` — the `devise:install` + `devise User` analogue.
//!
//! Emits a User migration + model, an `auth:` config snippet, and injects a bare
//! `auth_routes!(User);` into `config/routes.rs` that targets doido-auth's
//! **built-in** controllers. It does **not** copy any auth controllers or views
//! into the app (run `doido generate auth:controllers` to eject those for
//! customization) and does **not** modify `Cargo.toml`.

use super::migration_support::{
    register_migration, render_migration_file, MIGRATION_LIB_BASE, MIGRATION_SRC_DIR,
};
use super::route_injector::{
    inject_auth_routes, read_models_mod, read_routes, register_model_module, MODELS_MOD_PATH,
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
    let _ = two_factor;
    template("user.rs.template").to_string()
}

fn user_entity(two_factor: bool) -> String {
    let two_factor_fields = if two_factor {
        "    pub two_factor_secret: Option<String>,\n    pub two_factor_enabled: bool,\n"
    } else {
        ""
    };
    template("user_entity.rs.template").replace("{two_factor_fields}", two_factor_fields)
}

fn entities_mod(existing: &str) -> String {
    doido_model::entities::register_entity_module(existing, "users")
}

impl AuthGenerator for AuthInstallGenerator {
    fn name(&self) -> &str {
        "auth:install"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let _api = args.contains(&"--api");
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
        let entities_mod_path = "app/models/_entities/mod.rs";
        let entities_mod_base = std::fs::read_to_string(entities_mod_path).unwrap_or_else(|_| {
            include_str!("../../templates/new/app/models/_entities/mod.rs").to_string()
        });
        let entities_mod = entities_mod(&entities_mod_base);
        // Bare `auth_routes!(User);` targeting the framework's built-in controllers.
        // Controllers/views are NOT copied — run `auth:controllers` to eject them.
        let routes = inject_auth_routes(&read_routes());

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
                path: "app/models/_entities/users.rs".to_string(),
                content: user_entity(two_factor),
            },
            GeneratedFile {
                path: entities_mod_path.to_string(),
                content: entities_mod,
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
                path: ROUTES_PATH.to_string(),
                content: routes,
            },
        ];

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
    fn emits_users_migration_and_bare_builtin_routes() {
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
        // Bare route targeting the framework's built-in controllers.
        assert!(routes.content.contains("auth_routes!(User);"));
        assert!(routes.content.contains("doido::auth::routes!"));
        // `auth_routes!(User)` expands to `AuthSessions::<User>` — the User model
        // must be brought into scope.
        assert!(routes
            .content
            .contains("use crate::models::user::Model as User;"));
        // No local controllers are referenced — nothing was copied into the app.
        assert!(!routes.content.contains("controllers: {"));
        assert!(!routes.content.contains("use crate::controllers::auth;"));

        let user = files
            .iter()
            .find(|f| f.path == "app/models/user.rs")
            .expect("user model");
        assert!(user.content.contains("impl AuthUser for Model"));
    }

    #[test]
    fn install_does_not_copy_controllers_or_views() {
        for args in [&[][..], &["--api"][..], &["--two-factor"][..]] {
            let files = AuthInstallGenerator.generate(args).unwrap();
            assert!(
                !files.iter().any(|f| f.path.contains("app/controllers/auth/")),
                "auth:install must not copy controllers (args {args:?})"
            );
            assert!(
                !files.iter().any(|f| f.path.contains("app/views/auth/")),
                "auth:install must not copy views (args {args:?})"
            );
        }
    }

    #[test]
    fn two_factor_adds_migration_columns() {
        let files = AuthInstallGenerator.generate(&["--two-factor"]).unwrap();
        let migration = files
            .iter()
            .find(|f| f.path.contains("create_users_table"))
            .unwrap();
        assert!(migration.content.contains("two_factor_secret"));
        assert!(migration.content.contains("two_factor_enabled"));
    }
}
