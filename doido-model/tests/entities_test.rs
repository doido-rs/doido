use doido_model::entities::{
    entity_modules, extension_stub, model_modules, register_entity_module, register_model_module,
    rewrite_generated_imports,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn entity_modules_parses_mod_declarations() {
    let content = r#"pub mod prelude;
pub mod posts;
pub mod users;
pub mod sea_orm_active_enums;
"#;
    assert_eq!(
        entity_modules(content),
        vec!["posts".to_string(), "users".to_string()]
    );
}

#[test]
fn model_modules_skips_entities_registry() {
    let content = "pub mod _entities;\npub mod post;\n// @generated-models\n";
    assert_eq!(model_modules(content), vec!["post".to_string()]);
}

#[test]
fn extension_stub_reexports_entity_table_module() {
    let stub = extension_stub("post", "posts");
    assert!(stub.contains("pub use super::_entities::posts::*;"));
    assert!(stub.contains("impl ActiveModelBehavior for ActiveModel {}"));
    assert!(stub.contains("#![allow(dead_code, unused_imports)]"));
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
    let once = register_entity_module(base, "posts");
    assert!(once.contains("pub mod posts;"));
    let twice = register_entity_module(&once, "posts");
    assert_eq!(once, twice);
}

#[test]
fn rewrite_generated_imports_replaces_sea_orm_paths() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("posts.rs"),
        "use sea_orm::entity::prelude::*;\nuse sea_orm::Set;\n",
    )
    .unwrap();
    rewrite_generated_imports(dir.path()).unwrap();
    let content = fs::read_to_string(dir.path().join("posts.rs")).unwrap();
    assert!(content.contains("use doido::model::sea_orm as sea_orm;"));
    assert!(content.contains("use doido::model::sea_orm::entity::prelude::*;"));
    assert!(content.contains("use doido::model::sea_orm::Set;"));
}

#[test]
fn rewrite_generated_imports_adds_alias_for_sea_orm_attributes() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("storage_blobs.rs"),
        "use sea_orm::entity::prelude::*;\n\n#[sea_orm(table_name = \"storage_blobs\")]\n",
    )
    .unwrap();
    rewrite_generated_imports(dir.path()).unwrap();
    let content = fs::read_to_string(dir.path().join("storage_blobs.rs")).unwrap();
    assert!(content.contains("use doido::model::sea_orm as sea_orm;"));
    assert!(content.contains("use doido::model::sea_orm::entity::prelude::*;"));
    assert!(content.contains("#![allow(dead_code, unused_imports)]"));
}

#[test]
fn rewrite_generated_imports_strips_active_model_behavior_impl() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("posts.rs"),
        "use sea_orm::entity::prelude::*;\n\n\
         #[derive(DeriveEntityModel)]\n\
         pub struct Model {}\n\n\
         impl ActiveModelBehavior for ActiveModel {}\n",
    )
    .unwrap();
    rewrite_generated_imports(dir.path()).unwrap();
    let content = fs::read_to_string(dir.path().join("posts.rs")).unwrap();
    assert!(!content.contains("impl ActiveModelBehavior for ActiveModel {}"));
}

#[test]
fn rewrite_generated_imports_adds_lint_allows_to_prelude() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("prelude.rs"),
        "pub use super::posts::Entity as Posts;\n",
    )
    .unwrap();
    rewrite_generated_imports(dir.path()).unwrap();
    let content = fs::read_to_string(dir.path().join("prelude.rs")).unwrap();
    assert!(content.contains("#![allow(unused_imports)]"));
}
