//! CLI discovery tests for conditional auth generators.

use doido_generators::auth_registry;
use doido_generators::commands::generate::{project_has_doido_auth_at, registry_for_project_at};
use doido_generators::default_registry;

#[test]
fn default_registry_excludes_auth_generators() {
    let reg = default_registry();
    for name in auth_registry::auth_generator_names() {
        assert!(
            !reg.list().contains(name),
            "{name} must not be in default_registry"
        );
    }
}

#[test]
fn auth_generators_absent_without_cargo_dep() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"
[dependencies]
doido = "0.0.9"
"#,
    )
    .unwrap();
    assert!(!project_has_doido_auth_at(dir.path()));
    let reg = registry_for_project_at(dir.path());
    assert!(!reg.list().contains(&"auth:install"));
}

#[test]
fn auth_generators_present_with_doido_auth_dep() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"
[dependencies]
doido-auth = "0.0.9"
"#,
    )
    .unwrap();
    assert!(project_has_doido_auth_at(dir.path()));
    let reg = registry_for_project_at(dir.path());
    assert!(reg.list().contains(&"auth:install"));
    assert!(reg.list().contains(&"auth:scaffold"));
}
