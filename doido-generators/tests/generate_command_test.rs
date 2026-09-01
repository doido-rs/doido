use doido_generators::commands::generate::{registry_for_project_at, run_with};
use std::fs;
use tempfile::TempDir;

#[test]
fn run_with_help_lists_generators() {
    run_with(&["help".to_string()], Vec::new());
}

#[test]
fn registry_for_empty_project_has_builtins_only() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let reg = registry_for_project_at(dir.path());
    assert!(reg.list().contains(&"model"));
}
