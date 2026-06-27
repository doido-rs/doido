use doido_model::migration::{add_index, alter_table, create_table, drop_table, rename_table};
use doido_model::sea_orm::ConnectionTrait;
use doido_model::testing::TestDb;
use doido_model::SchemaManager;

#[tokio::test]
async fn create_table_insert_then_drop() {
    let db = TestDb::new().await.unwrap();
    let conn = db.conn();
    let manager = SchemaManager::new(conn);

    create_table(&manager, "users", |t| {
        t.string("email").not_null();
        t.integer("age");
        t.timestamps();
    })
    .await
    .unwrap();

    // The implicit `id` plus the declared columns all exist.
    conn.execute_unprepared("INSERT INTO users (email, age) VALUES ('a@b.c', 30)")
        .await
        .unwrap();
    conn.execute_unprepared("SELECT id, email, age, created_at, updated_at FROM users")
        .await
        .unwrap();

    drop_table(&manager, "users").await.unwrap();
    // Table is gone.
    assert!(conn
        .execute_unprepared("SELECT 1 FROM users")
        .await
        .is_err());
}

#[tokio::test]
async fn add_then_remove_column() {
    let db = TestDb::new().await.unwrap();
    let conn = db.conn();
    let manager = SchemaManager::new(conn);

    create_table(&manager, "posts", |t| {
        t.string("title");
    })
    .await
    .unwrap();

    // `alter_table` batches column changes, each applied as its own statement.
    alter_table(&manager, "posts", |t| {
        t.add_column("views", |c| {
            c.integer();
        });
    })
    .await
    .unwrap();
    conn.execute_unprepared("INSERT INTO posts (title, views) VALUES ('hi', 5)")
        .await
        .unwrap();

    alter_table(&manager, "posts", |t| {
        t.drop_column("views");
    })
    .await
    .unwrap();
    // The column is gone.
    assert!(conn
        .execute_unprepared("SELECT views FROM posts")
        .await
        .is_err());
}

#[tokio::test]
async fn rename_column_table_and_add_index() {
    let db = TestDb::new().await.unwrap();
    let conn = db.conn();
    let manager = SchemaManager::new(conn);

    create_table(&manager, "items", |t| {
        t.string("sku");
    })
    .await
    .unwrap();

    add_index(&manager, "items", &["sku"]).await.unwrap();
    alter_table(&manager, "items", |t| {
        t.rename_column("sku", "code");
    })
    .await
    .unwrap();
    rename_table(&manager, "items", "products").await.unwrap();

    // New name + new column work; old column name no longer exists.
    conn.execute_unprepared("INSERT INTO products (code) VALUES ('x')")
        .await
        .unwrap();
    assert!(conn
        .execute_unprepared("SELECT sku FROM products")
        .await
        .is_err());
}
