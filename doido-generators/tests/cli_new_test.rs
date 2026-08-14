use assert_cmd::Command;
use std::fs;

#[test]
fn test_doido_new_creates_project_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args(["new", "my-app", "--non-interactive", "--database=sqlite"])
        .assert()
        .success();

    assert!(dir.path().join("my-app/Cargo.toml").exists());
    assert!(dir.path().join("my-app/src/main.rs").exists());
    assert!(dir.path().join("my-app/config/application.toml").exists());
    assert!(dir.path().join("my-app/config/routes.rs").exists());
    assert!(dir.path().join("my-app/tests/integration_test.rs").exists());
    assert!(dir.path().join("my-app/.gitignore").exists());
    assert!(dir
        .path()
        .join("my-app/app/controllers/hello_controller.rs")
        .exists());
    assert!(dir.path().join("my-app/app/models/.gitkeep").exists());
    assert!(dir
        .path()
        .join("my-app/app/views/layouts/application.html.tera")
        .exists());
    assert!(dir.path().join("my-app/db/schema/.gitkeep").exists());
    // `db/migration` is a SeaORM migration project rather than an empty folder.
    assert!(dir.path().join("my-app/db/migration/Cargo.toml").exists());
    assert!(dir.path().join("my-app/db/migration/src/lib.rs").exists());
    assert!(dir.path().join("my-app/db/migration/src/main.rs").exists());
    assert!(dir.path().join("my-app/db/seed/Cargo.toml").exists());
    assert!(dir.path().join("my-app/db/seed/src/main.rs").exists());
}

#[test]
fn test_doido_new_cargo_toml_has_correct_name_and_database() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args([
            "new",
            "blog-app",
            "--non-interactive",
            "--database=postgres",
        ])
        .assert()
        .success();

    let cargo_toml = fs::read_to_string(dir.path().join("blog-app/Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("blog-app"));
    assert!(cargo_toml.contains("postgres"));

    let app_config =
        fs::read_to_string(dir.path().join("blog-app/config/application.toml")).unwrap();
    assert!(app_config.contains("postgres://postgres:postgres@localhost:5432/blog-app_development"));
}

#[test]
fn test_doido_new_cargo_toml_has_cache_redis_feature() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args([
            "new",
            "cache-app",
            "--non-interactive",
            "--database=sqlite",
            "--cache=redis",
        ])
        .assert()
        .success();

    let cargo_toml = fs::read_to_string(dir.path().join("cache-app/Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("cache-redis"));

    let dev_yml = fs::read_to_string(dir.path().join("cache-app/config/development.yml")).unwrap();
    assert!(dev_yml.contains("type: redis"));
}

#[test]
fn test_doido_new_auth_uses_builtin_controllers_without_copying() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args([
            "new",
            "auth-app",
            "--non-interactive",
            "--database=sqlite",
            "--auth",
        ])
        .assert()
        .success();

    let app = dir.path().join("auth-app");
    // Built-in-by-default: model + migration, but no controllers/views copied.
    assert!(app.join("app/models/user.rs").exists());
    assert!(!app.join("app/controllers/auth").exists());
    assert!(!app.join("app/views/auth").exists());

    let routes = fs::read_to_string(app.join("config/routes.rs")).unwrap();
    assert!(routes.contains("auth_routes!(User);"));
    assert!(routes.contains("use crate::models::user::Model as User;"));
    assert!(!routes.contains("controllers: {"));
}

#[test]
fn test_doido_new_auth_api_also_uses_builtin_controllers() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args([
            "new",
            "auth-api-app",
            "--non-interactive",
            "--database=sqlite",
            "--auth",
            "--api",
        ])
        .assert()
        .success();

    let app = dir.path().join("auth-api-app");
    assert!(!app.join("app/controllers/auth").exists());
    assert!(!app.join("app/views/auth").exists());
    let routes = fs::read_to_string(app.join("config/routes.rs")).unwrap();
    assert!(routes.contains("auth_routes!(User);"));
}

#[test]
fn test_doido_new_creates_git_repository() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args(["new", "my-app", "--non-interactive", "--database=sqlite"])
        .assert()
        .success();

    assert!(dir.path().join("my-app/.git").exists());
}

#[test]
fn test_doido_generate_model_writes_model_migration_and_lib() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args(["generate", "model", "User"])
        .assert()
        .success();

    // Entity in _entities/; extension stub in app/models/.
    let entity_path = dir.path().join("app/models/_entities/users.rs");
    assert!(entity_path.exists());
    let entity = fs::read_to_string(&entity_path).unwrap();
    assert!(entity.contains("DeriveEntityModel"));

    let model_path = dir.path().join("app/models/user.rs");
    assert!(model_path.exists());
    let content = fs::read_to_string(&model_path).unwrap();
    assert!(content.contains("pub use super::_entities::users::*;"));

    // Migration registered in the migration crate lib.rs.
    let lib = fs::read_to_string(dir.path().join("db/migration/src/lib.rs")).unwrap();
    assert!(lib.contains("_create_users_table::Migration)"));

    // The migration file itself exists in db/migration/src/.
    let migration_exists = fs::read_dir(dir.path().join("db/migration/src"))
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with('m') && name.ends_with("_create_users_table.rs")
        });
    assert!(migration_exists);
}
