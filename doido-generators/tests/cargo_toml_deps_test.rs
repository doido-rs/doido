//! Tests that generated `Cargo.toml` dependency lines match the runtime dependency
//! mode (local path vs published version).

use doido_generators::generators::new::ProjectGenerator;
use doido_generators::{DependencyMode, Generator, DOIDO_VERSION};

fn cargo_toml_for(args: &[&str]) -> String {
    ProjectGenerator
        .generate(args)
        .unwrap()
        .into_iter()
        .find(|f| {
            f.path.ends_with("/Cargo.toml")
                && !f.path.contains("db/migration")
                && !f.path.contains("db/seed")
        })
        .map(|f| f.content)
        .expect("app Cargo.toml")
}

fn migration_cargo_toml_for(args: &[&str]) -> String {
    ProjectGenerator
        .generate(args)
        .unwrap()
        .into_iter()
        .find(|f| f.path.contains("db/migration/Cargo.toml"))
        .map(|f| f.content)
        .expect("migration Cargo.toml")
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
    assert!(cargo.contains("default-features = false"));
    assert!(cargo.contains("features = [\"sqlite\", \"cache-redis\"]"));

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

#[test]
fn jobs_db_features_include_database_driver() {
    let cargo = cargo_toml_for(&["app", "--database=postgres", "--jobs=db"]);
    assert!(cargo.contains("jobs-db"));
    assert!(cargo.contains("postgres"));
    assert!(cargo.contains("doido-jobs = {"));
    assert!(cargo.contains("features = [\"jobs-db\", \"postgres\"]"));
    assert!(!cargo.contains("}, features ="));
}

#[test]
fn doido_model_database_features_stay_on_app_dependency_line() {
    let cases = [
        ("sqlite", "features = [\"sqlite\"]"),
        ("postgres", "features = [\"postgres\"]"),
        ("mysql", "features = [\"mysql\"]"),
    ];
    for (database, feature) in cases {
        let cargo = cargo_toml_for(&["app", &format!("--database={database}")]);
        cargo.parse::<toml::Table>().expect("valid Cargo.toml");
        assert!(
            cargo.contains("doido-model = {"),
            "{database}: app must declare doido-model"
        );
        assert!(
            cargo.contains(feature),
            "{database}: app doido-model line must include {feature}"
        );
    }
}

#[test]
fn app_depends_on_the_local_migration_crate() {
    // The app links the `migration` crate so `doido db migrate` runs the
    // `Migrator` in-process; the seeder lives in `db/seeds.rs`, not a crate.
    let cargo = cargo_toml_for(&["app", "--database=sqlite"]);
    assert!(
        cargo.contains("migration = { path = \"db/migration\" }"),
        "app Cargo.toml must depend on the local migration crate"
    );
    assert!(
        cargo.contains("members = [\"db/migration\"]"),
        "workspace must list only db/migration (the seed crate is gone)"
    );
    assert!(
        !cargo.contains("db/seed"),
        "the db/seed crate must no longer be referenced"
    );
}

#[test]
fn doido_model_database_features_stay_on_migration_dependency_line() {
    let cases = [
        ("sqlite", "features = [\"sqlite\"]"),
        ("postgres", "features = [\"postgres\"]"),
        ("mysql", "features = [\"mysql\"]"),
    ];
    for (database, feature) in cases {
        let cargo = migration_cargo_toml_for(&["app", &format!("--database={database}")]);
        cargo
            .parse::<toml::Table>()
            .expect("valid migration Cargo.toml");
        assert!(
            cargo.contains("doido = {"),
            "{database}: migration crate must declare doido meta crate"
        );
        assert!(
            cargo.contains(feature),
            "{database}: doido line must include {feature}"
        );
        assert!(
            !cargo.contains("}, features ="),
            "{database}: features must stay inside the inline table"
        );
    }
}
