//! `doido_auth::generators::field::Field` parsing + rendering across every
//! column type, modifier, and error path.

use doido_auth::generators::field::{ColumnType, Field};

/// (spec, expected builder method, expected html input type)
const TYPES: &[(&str, &str, &str)] = &[
    ("a:string", "string", "text"),
    ("a:text", "text", "textarea"),
    ("a:integer", "integer", "number"),
    ("a:int", "integer", "number"),
    ("a:bigint", "big_integer", "number"),
    ("a:long", "big_integer", "number"),
    ("a:float", "float", "number"),
    ("a:double", "double", "number"),
    ("a:decimal", "decimal", "number"),
    ("a:numeric", "decimal", "number"),
    ("a:boolean", "boolean", "checkbox"),
    ("a:bool", "boolean", "checkbox"),
    ("a:timestamp", "timestamp", "datetime-local"),
    ("a:datetime", "timestamp", "datetime-local"),
    ("a:date", "date", "date"),
    ("a:json", "json", "text"),
    ("a:jsonb", "json", "text"),
    ("a:uuid", "uuid", "text"),
    ("a:binary", "binary", "text"),
    ("a:blob", "binary", "text"),
    ("a:references", "references", "number"),
    ("a:belongs_to", "references", "number"),
];

#[test]
fn every_column_type_renders() {
    for (spec, builder, input) in TYPES {
        let field = Field::parse(spec).unwrap_or_else(|e| panic!("parse {spec}: {e}"));
        assert!(
            field.migration_line().contains(&format!("t.{builder}(")),
            "spec {spec} should build via t.{builder}(...)"
        );
        assert_eq!(field.html_input_type(), *input, "input type for {spec}");
        // Rendering helpers must all produce non-empty output.
        assert!(!field.params_struct_field().is_empty());
        assert!(!field.model_field().is_empty());
        assert!(!field.active_model_set().is_empty());
        assert!(!field.active_model_assign().is_empty());
        assert!(!field.html_form_control("widget").is_empty());
    }
}

#[test]
fn defaults_to_string_when_type_omitted() {
    let field = Field::parse("title").unwrap();
    assert_eq!(field.column_name(), "title");
    assert!(field.migration_line().contains("t.string(\"title\")"));
}

#[test]
fn references_column_is_suffixed_and_required() {
    let field = Field::parse("author:references").unwrap();
    assert_eq!(field.column_name(), "author_id");
    assert!(field.is_required());
    assert!(!field.is_user_reference());
    assert!(field.model_field().contains("author_id: i64"));

    let user_ref = Field::parse("user:references").unwrap();
    assert!(user_ref.is_user_reference());
}

#[test]
fn modifiers_control_null_unique_and_index() {
    let field = Field::parse("email:string:required:unique:index").unwrap();
    assert!(field.is_required());
    assert!(field.wants_index());
    let line = field.migration_line();
    assert!(line.contains(".not_null()"));
    assert!(line.contains(".unique_key()"));

    // Optional (nullable) plain column: Option<T>, no not_null().
    let optional = Field::parse("nickname:string").unwrap();
    assert!(!optional.is_required());
    assert!(optional.model_field().contains("Option<String>"));
    assert!(!optional.migration_line().contains(".not_null()"));
}

#[test]
fn required_boolean_uses_serde_default_in_params() {
    let field = Field::parse("active:boolean:required").unwrap();
    assert!(field.params_struct_field().contains("#[serde(default)]"));
    // References never emit `.not_null()` (the FK column carries the constraint).
    let reference = Field::parse("team:references").unwrap();
    assert!(!reference.migration_line().contains(".not_null()"));
}

#[test]
fn column_type_parse_is_case_insensitive() {
    assert!(Field::parse("A:STRING").is_ok());
    assert!(Field::parse("A:Integer:REQUIRED").unwrap().is_required());
}

#[test]
fn parse_all_reports_errors() {
    assert!(Field::parse_all(&["ok:string", "bad:notatype"]).is_err());
    assert!(Field::parse("").is_err());
    assert!(Field::parse("x:string:whatmod").is_err());
    // A sanity check on the exported enum type.
    let _ = ColumnType::String;
}
