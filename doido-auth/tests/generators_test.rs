//! Integration tests for auth generators.

use doido_auth::generators::{
    register, AuthControllerGenerator, AuthGenerator, AuthGeneratorRegistry, AuthInstallGenerator,
    AuthScaffoldGenerator,
};

struct TestRegistry {
    names: Vec<String>,
}

impl AuthGeneratorRegistry for TestRegistry {
    fn register_auth(&mut self, generator: Box<dyn AuthGenerator>) {
        self.names.push(generator.name().to_string());
    }
}

#[test]
fn register_exports_all_generators() {
    let mut reg = TestRegistry { names: Vec::new() };
    register(&mut reg);
    assert!(reg.names.contains(&"auth:install".to_string()));
    assert!(reg.names.contains(&"auth:controllers".to_string()));
    assert!(reg.names.contains(&"auth:controller".to_string()));
    assert!(reg.names.contains(&"auth:scaffold".to_string()));
    assert_eq!(reg.names.len(), 4);
}

#[test]
fn auth_install_emits_user_migration_and_builtin_routes() {
    let files = AuthInstallGenerator.generate(&[]).unwrap();

    assert!(files.iter().any(|f| f.path.contains("create_users_table")));
    assert!(files.iter().any(|f| f.path == "app/models/user.rs"));

    // Built-in-by-default: no auth controllers/views are copied into the app.
    assert!(
        !files
            .iter()
            .any(|f| f.path.starts_with("app/controllers/auth/")),
        "auth:install must not copy controllers"
    );
    assert!(
        !files.iter().any(|f| f.path.starts_with("app/views/auth/")),
        "auth:install must not copy views"
    );

    let routes = files
        .iter()
        .find(|f| f.path == "config/routes.rs")
        .expect("routes.rs emitted");
    // Bare route targeting doido-auth's built-in controllers.
    assert!(routes.content.contains("auth_routes!(User);"));
    assert!(!routes.content.contains("controllers: {"));
}

#[test]
fn auth_install_route_injection_is_idempotent() {
    let first = AuthInstallGenerator.generate(&[]).unwrap();
    let routes = first
        .iter()
        .find(|f| f.path == "config/routes.rs")
        .unwrap()
        .content
        .clone();

    std::fs::create_dir_all("config").ok();
    std::fs::write("config/routes.rs", &routes).unwrap();

    let second = AuthInstallGenerator.generate(&[]).unwrap();
    let routes2 = second
        .iter()
        .find(|f| f.path == "config/routes.rs")
        .unwrap();
    assert_eq!(routes.matches("auth_routes!(User").count(), 1);
    assert_eq!(routes2.content.matches("auth_routes!(User").count(), 1);

    let _ = std::fs::remove_file("config/routes.rs");
}

#[test]
fn auth_install_api_flag_skips_views() {
    let files = AuthInstallGenerator.generate(&["--api"]).unwrap();
    assert!(!files.iter().any(|f| f.path.starts_with("app/views/auth/")));
}

#[test]
fn auth_install_two_factor_flag_adds_columns() {
    let files = AuthInstallGenerator.generate(&["--two-factor"]).unwrap();
    let migration = files
        .iter()
        .find(|f| f.path.contains("create_users_table"))
        .unwrap();
    assert!(migration.content.contains("two_factor_secret"));
    assert!(migration.content.contains("two_factor_enabled"));
}

#[test]
fn auth_controller_emits_require_user_guard() {
    let files = AuthControllerGenerator
        .generate(&["Projects", "index"])
        .unwrap();
    let controller = files
        .iter()
        .find(|f| f.path == "app/controllers/projects_controller.rs")
        .unwrap();
    assert!(controller
        .content
        .contains("#[before_action(require_user)]"));
    assert!(controller.content.contains("CurrentUser<User>"));
}

#[test]
fn auth_scaffold_adds_user_reference_and_auth_guards() {
    let files = AuthScaffoldGenerator
        .generate(&["Article", "title:string"])
        .unwrap();

    let migration = files
        .iter()
        .find(|f| f.path.contains("create_articles_table"))
        .expect("articles migration");
    assert!(migration.content.contains("references(\"user\")"));

    let controller = files
        .iter()
        .find(|f| f.path.ends_with("articles_controller.rs"))
        .unwrap();
    assert!(controller.content.contains("require_user"));
    assert!(controller.content.contains("user_id: Set(user.id())"));

    let routes = files.iter().find(|f| f.path == "config/routes.rs").unwrap();
    assert!(routes
        .content
        .contains("resources!(articles, ArticlesController);"));
}

#[test]
fn auth_scaffold_runs_install_when_user_model_missing() {
    let user_path = std::path::Path::new("app/models/user.rs");
    let had_user = user_path.exists();
    if had_user {
        let _ = std::fs::rename(user_path, "app/models/user.rs.bak");
    }

    let files = AuthScaffoldGenerator
        .generate(&["Note", "body:text"])
        .unwrap();
    assert!(files.iter().any(|f| f.path == "app/models/user.rs"));

    if had_user {
        let _ = std::fs::rename("app/models/user.rs.bak", user_path);
    } else {
        let _ = std::fs::remove_file(user_path);
    }
}
