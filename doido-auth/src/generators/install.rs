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
    inject_auth_routes, inject_auth_routes_only, read_models_mod, read_routes,
    register_model_module, MODELS_MOD_PATH, ROUTES_PATH,
};
use super::template;
use super::{AuthGenerator, GeneratedFile};
use crate::config::AuthModule;
use chrono::Utc;
use doido_core::Result;

pub struct AuthInstallGenerator;

const IMPORTS: &str = "use doido::model::migration::{create_table, drop_table};";

/// Migration column statements contributed by `module`, one `t.<type>(...)…;`
/// per line (indented for the `create_table` closure body). Behavior-only and
/// base modules contribute nothing here.
fn module_migration_columns(module: AuthModule) -> &'static [&'static str] {
    match module {
        AuthModule::Rememberable => &["            t.timestamp(\"remember_created_at\");"],
        AuthModule::Trackable => &[
            "            t.integer(\"sign_in_count\").not_null().default(0);",
            "            t.timestamp(\"current_sign_in_at\");",
            "            t.timestamp(\"last_sign_in_at\");",
            "            t.string(\"current_sign_in_ip\");",
            "            t.string(\"last_sign_in_ip\");",
        ],
        AuthModule::Recoverable => &[
            "            t.string(\"reset_password_token\");",
            "            t.timestamp(\"reset_password_sent_at\");",
        ],
        AuthModule::Confirmable => &[
            "            t.string(\"confirmation_token\");",
            "            t.timestamp(\"confirmed_at\");",
            "            t.timestamp(\"confirmation_sent_at\");",
            "            t.string(\"unconfirmed_email\");",
        ],
        AuthModule::Lockable => &[
            "            t.integer(\"failed_attempts\").not_null().default(0);",
            "            t.string(\"unlock_token\");",
            "            t.timestamp(\"locked_at\");",
        ],
        AuthModule::TwoFactorAuthenticatable => &[
            "            t.string(\"two_factor_secret\");",
            "            t.boolean(\"two_factor_enabled\").not_null().default(false);",
        ],
        _ => &[],
    }
}

/// SeaORM entity struct fields contributed by `module` (matches the migration
/// columns above). Emitted into `_entities/users.rs` so the app compiles before
/// the first `db migrate` (which then regenerates the entity from the schema).
fn module_entity_fields(module: AuthModule) -> &'static [&'static str] {
    match module {
        AuthModule::Rememberable => &["    pub remember_created_at: Option<DateTimeUtc>,"],
        AuthModule::Trackable => &[
            "    pub sign_in_count: i32,",
            "    pub current_sign_in_at: Option<DateTimeUtc>,",
            "    pub last_sign_in_at: Option<DateTimeUtc>,",
            "    pub current_sign_in_ip: Option<String>,",
            "    pub last_sign_in_ip: Option<String>,",
        ],
        AuthModule::Recoverable => &[
            "    pub reset_password_token: Option<String>,",
            "    pub reset_password_sent_at: Option<DateTimeUtc>,",
        ],
        AuthModule::Confirmable => &[
            "    pub confirmation_token: Option<String>,",
            "    pub confirmed_at: Option<DateTimeUtc>,",
            "    pub confirmation_sent_at: Option<DateTimeUtc>,",
            "    pub unconfirmed_email: Option<String>,",
        ],
        AuthModule::Lockable => &[
            "    pub failed_attempts: i32,",
            "    pub unlock_token: Option<String>,",
            "    pub locked_at: Option<DateTimeUtc>,",
        ],
        AuthModule::TwoFactorAuthenticatable => &[
            "    pub two_factor_secret: Option<String>,",
            "    pub two_factor_enabled: bool,",
        ],
        _ => &[],
    }
}

fn users_up_body(modules: &[AuthModule]) -> String {
    let mut body = String::from(
        "        create_table(manager, \"users\", |t| {\n\
         \x20           t.string(\"email\").not_null().unique_key();\n\
         \x20           t.string(\"password_digest\").not_null();\n",
    );
    for module in AuthModule::ALL {
        if modules.contains(&module) {
            for line in module_migration_columns(module) {
                body.push_str(line);
                body.push('\n');
            }
        }
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

fn modules_yaml(modules: &[AuthModule]) -> String {
    let mut s = String::from("  modules:\n");
    for module in AuthModule::ALL {
        if modules.contains(&module) {
            s.push_str("    - ");
            s.push_str(module.as_str());
            s.push('\n');
        }
    }
    s
}

fn auth_section(modules: &[AuthModule]) -> String {
    let enabled = modules.contains(&AuthModule::TwoFactorAuthenticatable);
    format!(
        "\nauth:\n  user_model: User\n{}  strategies:\n    - cookie\n  two_factor:\n    enabled: {enabled}\n    issuer: MyApp\n  routes:\n    prefix: /users\n",
        modules_yaml(modules)
    )
}

fn config_file(path: &str, modules: &[AuthModule]) -> Option<GeneratedFile> {
    let existing = std::fs::read_to_string(path).ok()?;
    if existing.contains("\nauth:") || existing.starts_with("auth:") {
        return None;
    }
    Some(GeneratedFile {
        path: path.to_string(),
        content: format!("{}{}", existing.trim_end(), auth_section(modules)),
    })
}

fn user_model() -> String {
    template("user.rs.template").to_string()
}

fn user_entity(modules: &[AuthModule]) -> String {
    let mut fields = String::new();
    for module in AuthModule::ALL {
        if modules.contains(&module) {
            for line in module_entity_fields(module) {
                fields.push_str(line);
                fields.push('\n');
            }
        }
    }
    template("user_entity.rs.template").replace("{module_fields}", &fields)
}

/// The module set selected for this install: `--modules=a,b,c` when given (with
/// `database_authenticatable` always ensured), otherwise the default set;
/// `--two-factor` adds `two_factor_authenticatable`.
fn selected_modules(args: &[&str]) -> (Vec<AuthModule>, bool) {
    let explicit = args.iter().find_map(|a| a.strip_prefix("--modules="));
    let mut modules: Vec<AuthModule> = match explicit {
        Some(list) => list
            .split(',')
            .filter_map(|s| AuthModule::from_str(s.trim()))
            .collect(),
        None => crate::config::AuthConfig::default().modules,
    };
    if !modules.contains(&AuthModule::DatabaseAuthenticatable) {
        modules.insert(0, AuthModule::DatabaseAuthenticatable);
    }
    if args.contains(&"--two-factor") && !modules.contains(&AuthModule::TwoFactorAuthenticatable) {
        modules.push(AuthModule::TwoFactorAuthenticatable);
    }
    (modules, explicit.is_some())
}

fn entities_mod(existing: &str) -> String {
    doido_model::entities::register_entity_module(existing, "users")
}

impl AuthGenerator for AuthInstallGenerator {
    fn name(&self) -> &str {
        "auth:install"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let (modules, explicit_modules) = selected_modules(args);

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let migration_module = format!("m{timestamp}_create_users_table");
        let migration = render_migration_file(
            &migration_module,
            IMPORTS,
            &users_up_body(&modules),
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
        // Routes target the framework's built-in controllers (nothing copied —
        // run `auth:controllers` to eject). A default install mounts every module
        // route (bare `auth_routes!(User);`); an explicit `--modules=` selection
        // restricts the mounted groups via `only:`.
        let routes = if explicit_modules {
            let cfg = crate::config::AuthConfig {
                modules: modules.clone(),
                ..Default::default()
            };
            let groups = cfg.enabled_route_groups();
            inject_auth_routes_only(&read_routes(), &groups)
        } else {
            inject_auth_routes(&read_routes())
        };

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
                content: user_entity(&modules),
            },
            GeneratedFile {
                path: entities_mod_path.to_string(),
                content: entities_mod,
            },
            GeneratedFile {
                path: "app/models/user.rs".to_string(),
                content: user_model(),
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

        if let Some(f) = config_file("config/development.yml", &modules) {
            files.push(f);
        }
        if let Some(f) = config_file("config/test.yml", &modules) {
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

    #[test]
    fn default_install_writes_module_list_to_config() {
        // config_file reads existing config off disk; test auth_section directly.
        let modules = crate::config::AuthConfig::default().modules;
        let section = auth_section(&modules);
        assert!(section.contains("modules:"));
        assert!(section.contains("- database_authenticatable"));
        assert!(section.contains("- registerable"));
        assert!(section.contains("- recoverable"));
        assert!(section.contains("- rememberable"));
        assert!(section.contains("- validatable"));
    }

    #[test]
    fn explicit_modules_generate_only_routes_and_columns() {
        let files = AuthInstallGenerator
            .generate(&["--modules=database_authenticatable,trackable,lockable,confirmable"])
            .unwrap();

        let routes = files.iter().find(|f| f.path == ROUTES_PATH).unwrap();
        assert!(routes.content.contains("auth_routes!(User, only: ["));
        assert!(routes.content.contains("sessions"));
        assert!(routes.content.contains("confirmation"));
        assert!(routes.content.contains("unlock"));
        // recoverable / registerable not selected — their route groups are absent.
        assert!(!routes.content.contains("registrations"));
        assert!(!routes.content.contains("passwords"));

        let migration = files
            .iter()
            .find(|f| f.path.contains("create_users_table"))
            .unwrap();
        assert!(migration.content.contains("sign_in_count"));
        assert!(migration.content.contains("failed_attempts"));
        assert!(migration.content.contains("confirmation_token"));

        let entity = files
            .iter()
            .find(|f| f.path == "app/models/_entities/users.rs")
            .unwrap();
        assert!(entity.content.contains("pub sign_in_count: i32,"));
        assert!(entity.content.contains("pub failed_attempts: i32,"));
        assert!(entity
            .content
            .contains("pub confirmation_token: Option<String>,"));
        // No leftover template placeholder.
        assert!(!entity.content.contains("{module_fields}"));
    }
}
