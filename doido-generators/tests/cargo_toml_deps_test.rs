//! Tests that generated `Cargo.toml` dependency lines match the build mode
//! (local path vs published version).

use doido_generators::generators::new::ProjectGenerator;
use doido_generators::{Generator, DOIDO_VERSION, TEMPLATE_USE_PATH_DEPS};

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
fn generated_deps_match_build_mode() {
    let cargo = cargo_toml_for(&["app", "--database=sqlite"]);

    if TEMPLATE_USE_PATH_DEPS {
        assert!(
            cargo.contains("doido = { path ="),
            "workspace build should emit path dependencies for doido"
        );
        assert!(
            !cargo.contains("doido = { version ="),
            "workspace build should not emit version dependencies for doido"
        );
    } else {
        assert!(
            !cargo.contains("path ="),
            "published build should not emit path dependencies"
        );
        assert!(
            cargo.contains(DOIDO_VERSION),
            "published build should pin crates.io version {DOIDO_VERSION}"
        );
        assert!(
            cargo.contains("doido = { version ="),
            "published build should use version = for doido"
        );
    }
}

#[test]
fn cache_redis_features_stay_on_doido_dependency_line() {
    let cargo = cargo_toml_for(&["app", "--database=sqlite", "--cache=redis"]);
    assert!(cargo.contains("cache-redis"));

    if TEMPLATE_USE_PATH_DEPS {
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
