//! CLI discovery tests for conditional payments generators.

use doido_generators::commands::generate::{
    project_has_doido_payments_at, registry_for_project_at,
};
use doido_generators::default_registry;
use doido_generators::payments_registry;

#[test]
fn default_registry_excludes_payments_generators() {
    let reg = default_registry();
    for name in payments_registry::payments_generator_names() {
        assert!(
            !reg.list().contains(name),
            "{name} must not be in default_registry"
        );
    }
}

#[test]
fn payments_generators_absent_without_cargo_dep() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"
[dependencies]
doido = "0.0.9"
"#,
    )
    .unwrap();
    assert!(!project_has_doido_payments_at(dir.path()));
    let reg = registry_for_project_at(dir.path());
    assert!(!reg.list().contains(&"payments:install"));
}

#[test]
fn payments_generators_present_with_doido_payments_dep() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"
[dependencies]
doido-payments = "0.0.1"
"#,
    )
    .unwrap();
    assert!(project_has_doido_payments_at(dir.path()));
    let reg = registry_for_project_at(dir.path());
    assert!(reg.list().contains(&"payments:install"));
    assert!(reg.list().contains(&"payments:scaffold"));
}
