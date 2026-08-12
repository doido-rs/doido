use doido_model::entities::{
    entity_modules, extension_stub, register_entity_module, register_model_module,
    sync_extension_stubs,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn entity_modules_parses_mod_declarations() {
    let content = r#"pub mod prelude;
pub mod post;
pub mod user;
pub mod sea_orm_active_enums;
"#;
    assert_eq!(
        entity_modules(content),
        vec!["post".to_string(), "user".to_string()]
    );
}

#[test]
fn extension_stub_reexports_entity() {
    let stub = extension_stub("post");
    assert!(stub.contains("pub use super::_entities::post::*;"));
    assert!(stub.contains("never overwritten"));
}

#[test]
fn register_model_module_is_idempotent() {
    let base = "pub mod _entities;\n\n// @generated-models\n";
    let once = register_model_module(base, "post");
    assert!(once.contains("pub mod post;"));
    let twice = register_model_module(&once, "post");
    assert_eq!(once, twice);
}

#[test]
fn register_entity_module_is_idempotent() {
    let base = "// @generated-entities\n";
    let once = register_entity_module(base, "post");
    assert!(once.contains("pub mod post;"));
    let twice = register_entity_module(&once, "post");
    assert_eq!(once, twice);
}

#[test]
fn sync_extension_stubs_creates_missing_files_and_registers_mod() {
    let dir = tempdir().unwrap();
    let entities = dir.path().join("_entities");
    let models = dir.path().join("models");
    fs::create_dir_all(&entities).unwrap();
    fs::create_dir_all(&models).unwrap();

    fs::write(
        entities.join("mod.rs"),
        "pub mod article;\n// @generated-entities\n",
    )
    .unwrap();
    fs::write(
        models.join("mod.rs"),
        "pub mod _entities;\n\n// @generated-models\n",
    )
    .unwrap();

    sync_extension_stubs(&entities, &models).unwrap();

    let article = fs::read_to_string(models.join("article.rs")).unwrap();
    assert!(article.contains("pub use super::_entities::article::*;"));

    let models_mod = fs::read_to_string(models.join("mod.rs")).unwrap();
    assert!(models_mod.contains("pub mod article;"));
}
