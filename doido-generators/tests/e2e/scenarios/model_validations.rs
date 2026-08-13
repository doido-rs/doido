//! Model validations — extension stubs survive entity re-export on migrate.

use crate::common::cli;
use crate::common::{AppHarness, BaseProfile};

const ARTICLE_EXTENSION: &str = r#"//! Article extensions with Active Record-style validations.
#![allow(dead_code, unused_imports)]
pub use super::_entities::articles::*;

use doido::model::validation::{Errors, Validate};

impl Validate for Model {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e.length("body", self.body.as_deref().unwrap_or(""), Some(5), Some(100));
        e
    }
}
"#;

const VALIDATION_TEST: &str = r#"#[path = "../app/models/mod.rs"]
mod models;

use doido::model::validation::Validate;
use models::article::Model;

#[test]
fn valid_article_passes() {
    let article = Model {
        id: 1,
        title: "Hello".into(),
        body: Some("long enough body".into()),
    };
    assert!(article.is_valid());
}

#[test]
fn blank_title_fails_presence() {
    let article = Model {
        id: 1,
        title: "   ".into(),
        body: Some("long enough body".into()),
    };
    let errors = article.validate();
    assert_eq!(errors.on("title"), vec!["can't be blank"]);
}

#[test]
fn short_body_fails_length() {
    let article = Model {
        id: 1,
        title: "Hello".into(),
        body: Some("tiny".into()),
    };
    assert!(!article.is_valid());
    assert!(article
        .validate()
        .full_messages()
        .iter()
        .any(|m| m.contains("body is too short")));
}

#[test]
fn custom_base_error_is_counted() {
    use doido::model::validation::Errors;
    let mut e = Errors::new();
    e.add("base", "something went wrong");
    assert_eq!(e.len(), 1);
}
"#;

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn model_validations_survive_entity_export_on_migrate() {
    let h = AppHarness::new("model_validations", BaseProfile::Default);
    h.generate(&[
        "generate",
        "model",
        "Article",
        "title:string:not_null",
        "body:text",
    ]);

    std::fs::write(h.app.join("app/models/article.rs"), ARTICLE_EXTENSION).unwrap();
    std::fs::write(
        h.app.join("tests/article_validation_test.rs"),
        VALIDATION_TEST,
    )
    .unwrap();

    h.build();
    h.prepare_database();

    let entity_path = h.app.join("app/models/_entities/articles.rs");
    let entities = std::fs::read_to_string(&entity_path).unwrap();
    assert!(
        entities.contains("DeriveEntityModel"),
        "migrate should export entity into _entities/"
    );
    assert!(
        entities.contains("table_name = \"articles\""),
        "entity module should follow the table name"
    );

    let extension = std::fs::read_to_string(h.app.join("app/models/article.rs")).unwrap();
    assert!(
        extension.contains("impl Validate for Model"),
        "extension stub must not be overwritten by migrate export"
    );
    assert!(
        !extension.contains("DeriveEntityModel"),
        "entity definition must stay in _entities/"
    );

    let status = cli::cargo_test(
        &h.app,
        &["--test", "article_validation_test", "--", "--nocapture"],
    );
    assert!(status.success(), "validation tests failed after migrate");
}
