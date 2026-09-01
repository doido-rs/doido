use doido_model::entities::{
    dedupe_model_extension_stubs, ensure_active_model_behavior_in_extensions,
    ensure_model_extension_stubs, entity_has_model_extension, entity_modules, extension_stub,
    model_extension_covers_entity, model_modules, reexported_entity_module, register_entity_module,
    register_model_module, rewrite_generated_imports, write_active_model_behavior_module,
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
    assert!(content.contains("use doido::model::sea_orm;"));
    assert!(content.contains("use doido::model::sea_orm::entity::prelude::*;"));
    assert!(content.contains("use doido::model::sea_orm::Set;"));
}

#[test]
fn rewrite_generated_imports_adds_sea_orm_import_for_attributes() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("storage_blobs.rs"),
        "use sea_orm::entity::prelude::*;\n\n#[sea_orm(table_name = \"storage_blobs\")]\n",
    )
    .unwrap();
    rewrite_generated_imports(dir.path()).unwrap();
    let content = fs::read_to_string(dir.path().join("storage_blobs.rs")).unwrap();
    assert!(content.contains("use doido::model::sea_orm;"));
    assert!(content.contains("use doido::model::sea_orm::entity::prelude::*;"));
    assert!(content.contains("#![allow(dead_code, unused_imports)]"));
}

#[test]
fn rewrite_generated_imports_is_idempotent_for_doido_imports() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("users.rs"),
        "//! Generated user entity\n\
         #![allow(dead_code)]\n\n\
         use doido::model::sea_orm;\n\
         use doido::model::sea_orm::entity::prelude::*;\n",
    )
    .unwrap();
    rewrite_generated_imports(dir.path()).unwrap();
    rewrite_generated_imports(dir.path()).unwrap();
    let content = fs::read_to_string(dir.path().join("users.rs")).unwrap();
    assert!(
        content
            .find("#![allow(dead_code, unused_imports)]")
            .unwrap()
            < content.find("use doido::model::sea_orm;").unwrap()
    );
    assert_eq!(content.matches("use doido::model::sea_orm;").count(), 1);
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
fn ensure_model_extension_stubs_creates_missing_stubs() {
    let dir = tempdir().unwrap();
    let entities = dir.path().join("_entities");
    let models = dir.path().join("models");
    fs::create_dir_all(&entities).unwrap();
    fs::create_dir_all(&models).unwrap();
    fs::write(
        entities.join("mod.rs"),
        "pub mod storage_blobs;\n// @generated-entities\n",
    )
    .unwrap();
    fs::write(
        models.join("mod.rs"),
        "pub mod _entities;\n// @generated-models\n",
    )
    .unwrap();

    ensure_model_extension_stubs(&entities, &models).unwrap();

    let stub = fs::read_to_string(models.join("storage_blob.rs")).unwrap();
    assert!(stub.contains("pub use super::_entities::storage_blobs::*;"));
    assert!(stub.contains("impl ActiveModelBehavior for ActiveModel {}"));
    let models_mod = fs::read_to_string(models.join("mod.rs")).unwrap();
    assert!(models_mod.contains("pub mod storage_blob;"));
}

#[test]
fn ensure_active_model_behavior_in_extensions_patches_reexports() {
    let dir = tempdir().unwrap();
    let models = dir.path().join("models");
    fs::create_dir_all(&models).unwrap();
    fs::write(
        models.join("user.rs"),
        "pub use super::_entities::users::*;\n",
    )
    .unwrap();

    ensure_active_model_behavior_in_extensions(&models).unwrap();

    let content = fs::read_to_string(models.join("user.rs")).unwrap();
    assert!(content.contains("impl ActiveModelBehavior for ActiveModel {}"));
}

#[test]
fn write_active_model_behavior_module_covers_orphan_entities() {
    let dir = tempdir().unwrap();
    let entities = dir.path().join("_entities");
    let models = dir.path().join("models");
    fs::create_dir_all(&entities).unwrap();
    fs::create_dir_all(&models).unwrap();
    fs::write(
        entities.join("mod.rs"),
        "pub mod posts;\n// @generated-entities\n",
    )
    .unwrap();
    fs::write(
        models.join("post.rs"),
        "#![allow(dead_code)]\n// inline tutorial model\n",
    )
    .unwrap();

    write_active_model_behavior_module(&entities, &models).unwrap();

    let behavior = fs::read_to_string(entities.join("active_model_behavior.rs")).unwrap();
    assert!(behavior.contains("impl ActiveModelBehavior for super::posts::ActiveModel {}"));
    let entities_mod = fs::read_to_string(entities.join("mod.rs")).unwrap();
    assert!(entities_mod.contains("pub mod active_model_behavior;"));
}

#[test]
fn model_extension_covers_entity_when_reexport_and_impl_present() {
    let content =
        "pub use super::_entities::users::*;\nimpl ActiveModelBehavior for ActiveModel {}\n";
    assert!(model_extension_covers_entity(content, "users"));
    assert!(!model_extension_covers_entity(
        "pub use super::_entities::users::*;\n",
        "users"
    ));
}

#[test]
fn model_extension_covers_entity_when_full_impl_present() {
    let content = "pub use super::_entities::addresses::*;\n\
                   #[async_trait::async_trait]\n\
                   impl ActiveModelBehavior for ActiveModel {\n\
                       async fn before_save<C>(self, _db: &C, _insert: bool) -> Result<Self, DbErr> {\n\
                           Ok(self)\n\
                       }\n\
                   }\n";
    assert!(model_extension_covers_entity(content, "addresses"));
}

#[test]
fn ensure_active_model_behavior_skips_when_full_impl_present() {
    let dir = tempdir().unwrap();
    let models = dir.path().join("models");
    fs::create_dir_all(&models).unwrap();
    let original = "pub use super::_entities::addresses::*;\n\
                    #[async_trait::async_trait]\n\
                    impl ActiveModelBehavior for ActiveModel {\n\
                        async fn before_save<C>(self, _db: &C, _insert: bool) -> Result<Self, DbErr> {\n\
                            Ok(self)\n\
                        }\n\
                    }\n";
    fs::write(models.join("address.rs"), original).unwrap();

    ensure_active_model_behavior_in_extensions(&models).unwrap();

    let content = fs::read_to_string(models.join("address.rs")).unwrap();
    assert_eq!(content, original);
    assert_eq!(
        content
            .matches("impl ActiveModelBehavior for ActiveModel")
            .count(),
        1
    );
}

#[test]
fn ensure_model_extension_stubs_skips_when_entity_already_reexported() {
    let dir = tempdir().unwrap();
    let entities = dir.path().join("_entities");
    let models = dir.path().join("models");
    fs::create_dir_all(&entities).unwrap();
    fs::create_dir_all(&models).unwrap();
    fs::write(entities.join("mod.rs"), "pub mod skus;\n").unwrap();
    fs::write(
        models.join("mod.rs"),
        "pub mod _entities;\npub mod sku;\n// @generated-models\n",
    )
    .unwrap();
    fs::write(
        models.join("sku.rs"),
        "pub use super::_entities::skus::*;\nimpl ActiveModelBehavior for ActiveModel {}\n",
    )
    .unwrap();

    ensure_model_extension_stubs(&entities, &models).unwrap();

    assert!(!models.join("skus.rs").exists());
    assert!(entity_has_model_extension(&models, "skus"));
}

#[test]
fn dedupe_model_extension_stubs_removes_plural_duplicate() {
    let dir = tempdir().unwrap();
    let models = dir.path().join("models");
    fs::create_dir_all(&models).unwrap();
    fs::write(
        models.join("mod.rs"),
        "pub mod _entities;\npub mod sku;\npub mod skus;\n// @generated-models\n",
    )
    .unwrap();
    fs::write(
        models.join("sku.rs"),
        "pub use super::_entities::skus::*;\n",
    )
    .unwrap();
    fs::write(
        models.join("skus.rs"),
        "pub use super::_entities::skus::*;\n",
    )
    .unwrap();

    dedupe_model_extension_stubs(&models).unwrap();

    assert!(models.join("sku.rs").exists());
    assert!(!models.join("skus.rs").exists());
    let models_mod = fs::read_to_string(models.join("mod.rs")).unwrap();
    assert!(models_mod.contains("pub mod sku;"));
    assert!(!models_mod.contains("pub mod skus;"));
}

#[test]
fn reexported_entity_module_parses_pub_use_line() {
    let content = "pub use super::_entities::skus::*;\n";
    assert_eq!(reexported_entity_module(content), Some("skus".to_string()));
}

#[test]
fn ensure_model_extension_stubs_skips_existing_model_modules() {
    let dir = tempdir().unwrap();
    let entities = dir.path().join("_entities");
    let models = dir.path().join("models");
    fs::create_dir_all(&entities).unwrap();
    fs::create_dir_all(&models).unwrap();
    fs::write(entities.join("mod.rs"), "pub mod users;\n").unwrap();
    fs::write(
        models.join("mod.rs"),
        "pub mod _entities;\npub mod user;\n// @generated-models\n",
    )
    .unwrap();
    fs::write(models.join("user.rs"), "// custom user model\n").unwrap();

    ensure_model_extension_stubs(&entities, &models).unwrap();

    assert_eq!(
        fs::read_to_string(models.join("user.rs")).unwrap(),
        "// custom user model\n"
    );
    assert!(!models.join("users.rs").exists());
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

#[test]
fn reexported_entity_module_skips_doc_comments_before_pub_use() {
    let content = "//! Model extensions\n\
                   #![allow(dead_code)]\n\n\
                   pub use super::_entities::skus::*;\n";
    assert_eq!(reexported_entity_module(content), Some("skus".to_string()));
}
