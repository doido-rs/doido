use assert_cmd::Command;
use std::fs;

#[test]
fn test_doido_new_creates_project_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args(["new", "my-app", "--database=sqlite"])
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
}

#[test]
fn test_doido_new_cargo_toml_has_correct_name_and_database() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args(["new", "blog-app", "--database=postgres"])
        .assert()
        .success();

    let cargo_toml = fs::read_to_string(dir.path().join("blog-app/Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("blog-app"));
    assert!(cargo_toml.contains("postgres"));

    let app_config =
        fs::read_to_string(dir.path().join("blog-app/config/application.toml")).unwrap();
    assert!(app_config.contains("postgres://localhost/blog-app_development"));
}

#[test]
fn test_doido_new_creates_git_repository() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args(["new", "my-app", "--database=sqlite"])
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

    // Model file.
    let model_path = dir.path().join("app/models/user.rs");
    assert!(model_path.exists());
    let content = fs::read_to_string(&model_path).unwrap();
    assert!(content.contains("DeriveEntityModel"));

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
