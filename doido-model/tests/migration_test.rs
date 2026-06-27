use doido_model::migration::{Column, Index, Table};
use doido_model::sea_orm::ConnectionTrait;
use doido_model::testing::TestDb;

#[tokio::test]
async fn create_table_insert_then_drop() {
    let db = TestDb::new().await.unwrap();
    let conn = db.conn();

    Table::create(conn, "users", |t| {
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

    Table::drop(conn, "users").await.unwrap();
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

    Table::create(conn, "posts", |t| {
        t.string("title");
    })
    .await
    .unwrap();

    Column::add(conn, "posts", "views", |c| {
        c.integer();
    })
    .await
    .unwrap();
    conn.execute_unprepared("INSERT INTO posts (title, views) VALUES ('hi', 5)")
        .await
        .unwrap();

    Column::remove(conn, "posts", "views").await.unwrap();
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

    Table::create(conn, "items", |t| {
        t.string("sku");
    })
    .await
    .unwrap();

    Index::add(conn, "items", &["sku"]).await.unwrap();
    Column::rename(conn, "items", "sku", "code").await.unwrap();
    Table::rename(conn, "items", "products").await.unwrap();

    // New name + new column work; old column name no longer exists.
    conn.execute_unprepared("INSERT INTO products (code) VALUES ('x')")
        .await
        .unwrap();
    assert!(conn
        .execute_unprepared("SELECT sku FROM products")
        .await
        .is_err());
}
