use doido_auth::generators::migration_support::{
    create_table_imports, create_table_up, register_migration, render_migration_file,
    MIGRATION_MOD_BASE,
};

#[test]
fn register_migration_is_idempotent_on_repeat() {
    let once = register_migration(MIGRATION_MOD_BASE, "m20260101_create_users_table");
    let twice = register_migration(&once, "m20260101_create_users_table");
    assert_eq!(once, twice);
}

#[test]
fn render_migration_file_contains_module_name() {
    let file = render_migration_file(
        "m20260101_create_users_table",
        "use doido::model::migration::{create_table, drop_table};",
        "        create_table(manager, \"users\", |_t| {}).await\n",
        "        drop_table(manager, \"users\").await\n",
    );
    assert!(file.contains("m20260101_create_users_table"));
}

#[test]
fn create_table_imports_omits_add_index_without_index_fields() {
    use doido_auth::generators::Field;
    let fields = Field::parse_all(&["name:string"]).unwrap();
    let imports = create_table_imports(&fields);
    assert!(!imports.contains("add_index"));
    let up = create_table_up("people", &fields);
    assert!(up.contains("t.string(\"name\")"));
}
