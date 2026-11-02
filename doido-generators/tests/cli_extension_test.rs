use assert_cmd::Command;
use std::fs;

#[test]
fn test_doido_extension_creates_crate_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido").unwrap();
    cmd.current_dir(dir.path())
        .args(["extension", "payments"])
        .assert()
        .success();

    assert!(dir.path().join("doido-payments/Cargo.toml").exists());
    assert!(dir.path().join("doido-payments/src/lib.rs").exists());
    assert!(dir
        .path()
        .join("doido-payments/src/generators/mod.rs")
        .exists());
    assert!(dir
        .path()
        .join("doido-payments/src/generators/install.rs")
        .exists());
    assert!(dir
        .path()
        .join("doido-payments/tests/generators_test.rs")
        .exists());
    assert!(dir.path().join("doido-payments/.gitignore").exists());
}

#[test]
fn test_doido_extension_cargo_toml_has_doido_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido").unwrap();
    cmd.current_dir(dir.path())
        .args(["extension", "Widget"])
        .assert()
        .success();

    let cargo_toml = fs::read_to_string(dir.path().join("doido-widget/Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("name = \"doido-widget\""));
    assert!(cargo_toml.contains("doido-core"));
    assert!(cargo_toml.contains("doido-controller"));
    assert!(cargo_toml.contains("doido ="));
}

#[test]
fn test_doido_extension_generators_test_uses_crate_ident() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido").unwrap();
    cmd.current_dir(dir.path())
        .args(["extension", "analytics"])
        .assert()
        .success();

    let test_rs =
        fs::read_to_string(dir.path().join("doido-analytics/tests/generators_test.rs")).unwrap();
    assert!(test_rs.contains("use doido_analytics::generators"));
    assert!(test_rs.contains("analytics:install"));
}
