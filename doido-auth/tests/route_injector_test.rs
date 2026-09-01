//! Route injection helpers used by auth and scaffold generators.

use doido_auth::generators::route_injector::{
    inject_action_routes, inject_auth_routes, inject_auth_routes_only, inject_resources,
    read_controllers_mod, read_models_mod, read_routes, register_auth_controllers_mod,
    register_controller, register_model_module, rewire_local_controllers, ROUTES_PATH,
};

const SAMPLE_ROUTES: &str = "\
use doido::controller::{axum, routes};

routes! {
    get!(\"/\", HelloController::index);
}
";

#[test]
fn inject_auth_routes_switches_block_and_adds_user_import() {
    let out = inject_auth_routes(SAMPLE_ROUTES);
    assert!(out.contains("doido::auth::routes!"));
    assert!(out.contains("auth_routes!(User);"));
    assert!(out.contains("use crate::models::user::Model as User;"));
    assert!(!out.contains("use doido::controller::{axum, routes}"));
}

#[test]
fn inject_auth_routes_is_idempotent() {
    let once = inject_auth_routes(SAMPLE_ROUTES);
    assert_eq!(once, inject_auth_routes(&once));
}

#[test]
fn inject_auth_routes_only_restricts_modules() {
    let out = inject_auth_routes_only(SAMPLE_ROUTES, &["sessions", "registrations"]);
    assert!(out.contains("auth_routes!(User, only: [sessions, registrations]);"));
}

#[test]
fn rewire_local_controllers_points_at_app_auth_module() {
    let installed = inject_auth_routes(SAMPLE_ROUTES);
    let out = rewire_local_controllers(&installed, true);
    assert!(out.contains("controllers: {"));
    assert!(out.contains("auth::SessionsController"));
    assert!(out.contains("two_factor: auth::TwoFactorController"));
    assert!(out.contains("use crate::controllers::auth;"));
}

#[test]
fn inject_resources_adds_controller_use_and_route() {
    let out = inject_resources(SAMPLE_ROUTES, "articles", "ArticlesController", false);
    assert!(out.contains("resources!(articles, ArticlesController);"));
    assert!(out.contains("use crate::controllers::ArticlesController;"));
}

#[test]
fn inject_resources_api_skips_form_routes() {
    let out = inject_resources(SAMPLE_ROUTES, "articles", "ArticlesController", true);
    assert!(out.contains("except: [new, edit]"));
}

#[test]
fn inject_action_routes_adds_named_get_routes() {
    let out = inject_action_routes(SAMPLE_ROUTES, "reports", "ReportsController", &["summary"]);
    assert!(out.contains("get!(\"/reports/summary\", ReportsController::summary);"));
}

#[test]
fn register_controller_and_auth_mod_are_idempotent() {
    let base = read_controllers_mod();
    let with_ctrl = register_controller(&base, "posts", "PostsController");
    assert!(with_ctrl.contains("mod posts_controller;"));
    assert_eq!(
        with_ctrl,
        register_controller(&with_ctrl, "posts", "PostsController")
    );

    let with_auth = register_auth_controllers_mod(&base);
    assert!(with_auth.contains("pub mod auth;"));
    assert_eq!(with_auth, register_auth_controllers_mod(&with_auth));
}

#[test]
fn register_model_module_inserts_above_marker() {
    let base = read_models_mod();
    let updated = register_model_module(&base, "article");
    assert!(updated.contains("pub mod article;"));
    assert!(updated.contains("@generated-models"));
}

#[test]
fn read_routes_falls_back_to_template_when_missing() {
    let dir = tempfile::TempDir::new().unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let routes = read_routes();
    assert!(routes.contains("routes!"));
    std::env::set_current_dir(original).unwrap();
    let _ = ROUTES_PATH; // keep import used
}
