//! Additional migration DSL coverage: column types, indexes, foreign keys.

use doido_model::migration::{
    add_column, add_index, create_table, remove_column, remove_index, rename_column,
};
use doido_model::sea_orm::ConnectionTrait;
use doido_model::testing::TestDb;
use doido_model::SchemaManager;

#[tokio::test]
async fn create_table_supports_all_common_column_types() {
    let db = TestDb::new().await.unwrap();
    let manager = SchemaManager::new(db.conn());

    create_table(&manager, "widgets", |t| {
        t.string("name").not_null();
        t.text("body");
        t.integer("count");
        t.big_integer("big");
        t.float("rate");
        t.double("precise");
        t.decimal("price");
        t.boolean("active");
        t.timestamp("seen_at");
        t.date("born_on");
        t.json("meta");
        t.uuid("token");
        t.binary("blob");
        t.references("owner");
        t.timestamps();
    })
    .await
    .unwrap();

    db.conn()
        .execute_unprepared(
            "INSERT INTO widgets (name, count, active, owner_id, created_at, updated_at) \
             VALUES ('x', 1, 1, 1, datetime('now'), datetime('now'))",
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn column_helpers_add_remove_and_rename() {
    let db = TestDb::new().await.unwrap();
    let manager = SchemaManager::new(db.conn());

    create_table(&manager, "notes", |t| {
        t.string("title");
    })
    .await
    .unwrap();

    add_column(&manager, "notes", "priority", |c| {
        c.integer();
    })
    .await
    .unwrap();

    rename_column(&manager, "notes", "title", "heading")
        .await
        .unwrap();

    remove_column(&manager, "notes", "priority").await.unwrap();

    db.conn()
        .execute_unprepared("INSERT INTO notes (heading) VALUES ('hi')")
        .await
        .unwrap();
}

#[tokio::test]
async fn index_helpers_add_and_remove() {
    let db = TestDb::new().await.unwrap();
    let manager = SchemaManager::new(db.conn());

    create_table(&manager, "tags", |t| {
        t.string("label");
    })
    .await
    .unwrap();

    add_index(&manager, "tags", &["label"]).await.unwrap();
    remove_index(&manager, "tags", &["label"]).await.unwrap();
}

#[tokio::test]
async fn alter_table_batches_multiple_changes() {
    use doido_model::migration::alter_table;

    let db = TestDb::new().await.unwrap();
    let manager = SchemaManager::new(db.conn());

    create_table(&manager, "items", |t| {
        t.string("sku");
    })
    .await
    .unwrap();

    alter_table(&manager, "items", |t| {
        t.add_column("qty", |c| {
            c.integer();
        });
        t.rename_column("sku", "code");
        t.drop_column("qty");
    })
    .await
    .unwrap();

    db.conn()
        .execute_unprepared("INSERT INTO items (code) VALUES ('abc')")
        .await
        .unwrap();
}
