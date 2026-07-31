//! Tests that generated `Cargo.toml` dependency lines match the runtime dependency
//! mode (local path vs published version).

use doido_generators::generators::new::ProjectGenerator;
use doido_generators::{DependencyMode, Generator, DOIDO_VERSION};

fn cargo_toml_for(args: &[&str]) -> String {
    ProjectGenerator
        .generate(args)
        .unwrap()
        .into_iter()
        .find(|f| f.path.ends_with("/Cargo.toml") && !f.path.contains("db/migration"))
        .map(|f| f.content)
        .expect("app Cargo.toml")
}

#[test]
fn generated_cargo_toml_parses_as_valid_toml() {
    let content = cargo_toml_for(&["app", "--database=sqlite", "--cache=redis", "--jobs=redis"]);
    content.parse::<toml::Table>().expect("valid Cargo.toml");
}

#[test]
fn generated_deps_match_runtime_mode() {
    let cargo = cargo_toml_for(&["app", "--database=sqlite"]);
    let mode = DependencyMode::resolve();

    if mode.use_path {
        assert!(
            cargo.contains("doido = { path ="),
            "dev checkout binary should emit path dependencies for doido"
        );
        assert!(
            !cargo.contains("doido = { version ="),
            "dev checkout binary should not emit version dependencies for doido"
        );
    } else {
        assert!(
            !cargo.contains("path ="),
            "isolated/published binary should not emit path dependencies"
        );
        assert!(
            cargo.contains(DOIDO_VERSION),
            "isolated/published binary should pin version {DOIDO_VERSION}"
        );
        assert!(
            cargo.contains("doido = { version ="),
            "isolated/published binary should use version = for doido"
        );
    }
}

#[test]
fn cache_redis_features_stay_on_doido_dependency_line() {
    let cargo = cargo_toml_for(&["app", "--database=sqlite", "--cache=redis"]);
    let mode = DependencyMode::resolve();
    assert!(cargo.contains("cache-redis"));

    if mode.use_path {
        assert!(cargo.contains("doido = { path ="));
    } else {
        assert!(cargo.contains("doido = { version ="));
        assert!(!cargo.contains("path ="));
    }

    // Features must not appear as a stray key outside the inline table.
    assert!(!cargo.contains("}, features ="));
}

#[test]
fn jobs_redis_features_stay_on_doido_jobs_dependency_line() {
    let cargo = cargo_toml_for(&["app", "--database=sqlite", "--jobs=redis"]);
    assert!(cargo.contains("jobs-redis"));
    assert!(cargo.contains("doido-jobs = {"));
    assert!(!cargo.contains("}, features ="));
}
