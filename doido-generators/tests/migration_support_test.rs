use doido_generators::generators::field::Field;
use doido_generators::generators::migration_support::{
    create_table_imports, create_table_up, drop_table_down, register_migration,
    render_migration_file, MIGRATION_MOD_BASE,
};

#[test]
fn register_migration_inserts_mod_and_list_entries() {
    let updated = register_migration(MIGRATION_MOD_BASE, "m20260101_create_posts_table");
    assert!(updated.contains("mod m20260101_create_posts_table;"));
    assert!(updated.contains("Box::new(m20260101_create_posts_table::Migration),"));
    assert!(updated.contains("@generated-migrations-mod"));
}

#[test]
fn render_migration_file_wires_name_and_bodies() {
    let body = render_migration_file(
        "m20260101_create_posts",
        "use doido::model::migration::{create_table, drop_table};",
        "        create_table(manager, \"posts\", |_t| {}).await\n",
        "        drop_table(manager, \"posts\").await\n",
    );
    assert!(body.contains("m20260101_create_posts"));
    assert!(body.contains("impl MigrationTrait for Migration"));
    assert!(body.contains("create_table(manager, \"posts\""));
}

#[test]
fn create_table_up_adds_index_lines_when_requested() {
    let fields = Field::parse_all(&["email:string:unique:index"]).unwrap();
    let imports = create_table_imports(&fields);
    assert!(imports.contains("add_index"));
    let up = create_table_up("users", &fields);
    assert!(up.contains("add_index(manager, \"users\""));
}

#[test]
fn create_table_up_empty_fields_emits_hint() {
    let up = create_table_up("widgets", &[]);
    assert!(up.contains("create_table(manager, \"widgets\""));
}

#[test]
fn drop_table_down_targets_table_name() {
    let down = drop_table_down("widgets");
    assert!(down.contains("drop_table(manager, \"widgets\")"));
}
