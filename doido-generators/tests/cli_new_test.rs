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
    assert!(dir.path().join("my-app/db/migration/.gitkeep").exists());
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
fn test_doido_generate_model_writes_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido-generators").unwrap();
    cmd.current_dir(dir.path())
        .args(["generate", "model", "User"])
        .assert()
        .success();

    assert!(dir.path().join("src/models/user.rs").exists());
    let content = fs::read_to_string(dir.path().join("src/models/user.rs")).unwrap();
    assert!(content.contains("DeriveEntityModel"));
}
