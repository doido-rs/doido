//! `generators::migration_support` — the index-present and empty-table branches
//! of `create_table_up`/`create_table_imports` plus `drop_table_down`.

use doido_auth::generators::field::Field;
use doido_auth::generators::migration_support::{
    create_table_imports, create_table_up, drop_table_down, render_migration_file,
};

#[test]
fn create_table_up_with_index_emits_add_index_and_ok() {
    let fields = Field::parse_all(&["email:string:unique:index", "name:string"]).unwrap();
    let up = create_table_up("users", &fields);
    assert!(up.contains("create_table(manager, \"users\""));
    assert!(up.contains("add_index(manager, \"users\", &[\"email\"])"));
    assert!(up.contains(".await?;"));
    assert!(up.contains("Ok(())"));

    // The imports line must pull in `add_index` when an index is present.
    assert!(create_table_imports(&fields).contains("add_index"));
}

#[test]
fn create_table_up_without_index_uses_plain_await() {
    let fields = Field::parse_all(&["title:string"]).unwrap();
    let up = create_table_up("posts", &fields);
    assert!(up.trim_end().ends_with(".await"));
    assert!(!up.contains("add_index"));
    assert!(!create_table_imports(&fields).contains("add_index"));
}

#[test]
fn create_table_up_empty_fields_emits_id_only_closure() {
    let up = create_table_up("things", &[]);
    assert!(up.contains("create_table(manager, \"things\", |_t| {})"));
    assert!(up.contains("auto-incrementing `id`"));
}

#[test]
fn drop_table_down_and_render_migration_file() {
    assert!(drop_table_down("widgets").contains("drop_table(manager, \"widgets\")"));
    let file = render_migration_file(
        "m20260101_000000_create_widgets_table",
        "use doido::model::migration::{create_table, drop_table};",
        &create_table_up("widgets", &Field::parse_all(&["label:string"]).unwrap()),
        &drop_table_down("widgets"),
    );
    assert!(file.contains("m20260101_000000_create_widgets_table"));
    assert!(file.contains("create_table(manager, \"widgets\""));
}
