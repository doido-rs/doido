use doido_generators::commands::destroy::destroyable_paths;
use doido_generators::{default_registry, Generator, InitializerGenerator};

#[test]
fn initializer_generator_is_registered_in_default_registry() {
    let reg = default_registry();
    assert!(reg.list().contains(&"initializer"));
}

#[test]
fn initializer_generator_requires_name() {
    let result = InitializerGenerator.generate(&[]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("initializer generator requires a name argument"));
}

#[test]
fn initializer_generator_produces_expected_files() {
    let files = InitializerGenerator.generate(&["cors"]).unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    assert!(paths.contains(&"config/initializers/cors.rs"));
    assert!(paths.contains(&"config/initializers/mod.rs"));
    assert!(paths.contains(&"tests/cors_initializer_test.rs"));

    let init_file = files
        .iter()
        .find(|f| f.path == "config/initializers/cors.rs")
        .unwrap();
    assert!(init_file.content.contains("pub fn init() -> Result<()>"));
    assert!(init_file.content.contains("Application initializer `cors`"));

    let mod_file = files
        .iter()
        .find(|f| f.path == "config/initializers/mod.rs")
        .unwrap();
    assert!(mod_file.content.contains("pub mod cors;"));
    assert!(mod_file
        .content
        .contains("inits.register(\"cors\", cors::init);"));
    assert!(mod_file.content.contains("pub fn init() -> Result<()>"));
    assert!(mod_file.content.contains("pub fn run() -> Result<()>"));
    assert!(mod_file
        .content
        .contains("pub fn registry() -> Initializers"));

    let test_file = files
        .iter()
        .find(|f| f.path == "tests/cors_initializer_test.rs")
        .unwrap();
    assert!(test_file
        .content
        .contains("#[path = \"../config/initializers/cors.rs\"]"));
    assert!(test_file.content.contains("mod cors;"));
    assert!(test_file.content.contains("assert!(cors::init().is_ok());"));
}

#[test]
fn initializer_generator_normalizes_names() {
    let files = InitializerGenerator
        .generate(&["custom-auth-setup"])
        .unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    assert!(paths.contains(&"config/initializers/custom_auth_setup.rs"));
    assert!(paths.contains(&"config/initializers/mod.rs"));
    assert!(paths.contains(&"tests/custom_auth_setup_initializer_test.rs"));

    let mod_file = files
        .iter()
        .find(|f| f.path == "config/initializers/mod.rs")
        .unwrap();
    assert!(mod_file.content.contains("pub mod custom_auth_setup;"));
    assert!(mod_file
        .content
        .contains("inits.register(\"custom_auth_setup\", custom_auth_setup::init);"));
}

#[test]
fn initializer_registration_is_idempotent() {
    let initial_mod = r#"use doido::controller::initializers::Initializers;
use doido::Result;

// @generated-modules

pub fn init() -> Result<()> {
    registry().run_all()
}

pub fn registry() -> Initializers {
    let mut inits = Initializers::new();
    register_initializers(&mut inits);
    inits
}

fn register_initializers(inits: &mut Initializers) {
    // @generated-initializers
}
"#;

    let updated_once = doido_generators::generators::initializer::register_initializer_in_mod(
        initial_mod,
        "database_logger",
    );
    assert_eq!(updated_once.matches("pub mod database_logger;").count(), 1);
    assert_eq!(
        updated_once
            .matches("inits.register(\"database_logger\", database_logger::init);")
            .count(),
        1
    );

    let updated_twice = doido_generators::generators::initializer::register_initializer_in_mod(
        &updated_once,
        "database_logger",
    );
    assert_eq!(updated_twice.matches("pub mod database_logger;").count(), 1);
    assert_eq!(
        updated_twice
            .matches("inits.register(\"database_logger\", database_logger::init);")
            .count(),
        1
    );
}

#[test]
fn initializer_destroyable_paths_skips_shared_mod() {
    let files = InitializerGenerator.generate(&["telemetry"]).unwrap();
    let destroyable = destroyable_paths(&files);

    assert_eq!(destroyable.len(), 2);
    assert!(destroyable.contains(&"config/initializers/telemetry.rs".to_string()));
    assert!(destroyable.contains(&"tests/telemetry_initializer_test.rs".to_string()));
    assert!(!destroyable.contains(&"config/initializers/mod.rs".to_string()));
}
