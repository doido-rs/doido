//! `generators::field::Field` — rendering across every column type, including
//! the alter-table lines (`column_def_method`) and sample-form-value helpers
//! used by generated request tests.

use doido_generators::generators::field::Field;

const TYPES: &[&str] = &[
    "a:string",
    "a:text",
    "a:integer",
    "a:bigint",
    "a:float",
    "a:double",
    "a:decimal",
    "a:boolean",
    "a:timestamp",
    "a:date",
    "a:json",
    "a:uuid",
    "a:binary",
    "a:references",
];

#[test]
fn every_type_renders_migration_alter_and_model() {
    for spec in TYPES {
        let f = Field::parse(spec).unwrap_or_else(|e| panic!("parse {spec}: {e}"));
        assert!(f.migration_line().starts_with("t."), "{spec} migration");
        let add = f.alter_add_line();
        assert!(add.contains("t.add_column("), "{spec} alter add");
        assert!(add.contains("|c| {"), "{spec} alter add closure");
        assert!(
            f.alter_drop_line().contains("t.drop_column("),
            "{spec} alter drop"
        );
        assert!(!f.model_field().is_empty());
        assert!(!f.html_input_type().is_empty());
        assert!(!f.params_struct_field().is_empty());
        assert!(!f.html_form_control("thing").is_empty());
    }
}

#[test]
fn sample_form_values_cover_scalar_types_and_skip_binary() {
    // Binary can't be a urlencoded scalar → no pair.
    assert!(Field::parse("blob:binary")
        .unwrap()
        .sample_form_pair()
        .is_none());
    assert!(Field::parse("blob:binary")
        .unwrap()
        .sample_form_value()
        .is_none());

    // Every scalar type yields a `col=value` pair.
    for (spec, expected_value) in [
        ("name:string", "Test"),
        ("n:integer", "1"),
        ("ok:boolean", "true"),
        ("when:timestamp", "2020-01-01T00:00:00"),
        ("day:date", "2020-01-01"),
        ("data:json", "null"),
        ("gid:uuid", "00000000-0000-0000-0000-000000000000"),
        ("owner:references", "1"),
    ] {
        let f = Field::parse(spec).unwrap();
        let pair = f.sample_form_pair().expect("scalar has a form pair");
        assert!(
            pair.ends_with(expected_value),
            "spec {spec} pair `{pair}` should end with {expected_value}"
        );
    }
}

#[test]
fn references_column_is_required_and_id_suffixed() {
    let f = Field::parse("author:references").unwrap();
    assert_eq!(f.column_name(), "author_id");
    assert!(f.is_required());
    // References never add an explicit `.not_null()` in the create-table line.
    assert!(!f.migration_line().contains(".not_null()"));
    // …but the alter add-column line marks it NOT NULL.
    assert!(f.alter_add_line().contains(".not_null()"));
}

#[test]
fn parse_errors_on_bad_type_and_modifier() {
    assert!(Field::parse("x:notatype").is_err());
    assert!(Field::parse("x:string:weirdmod").is_err());
    assert!(Field::parse_all(&["ok:string", "bad:xyz"]).is_err());
}
