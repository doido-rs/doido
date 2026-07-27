//! ensure_tables creates the three storage tables and is idempotent.

use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement};

async fn table_exists(conn: &doido_model::sea_orm::DatabaseConnection, name: &str) -> bool {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "SELECT name FROM sqlite_master WHERE type='table' AND name = ?",
        [name.into()],
    );
    conn.query_one_raw(stmt).await.unwrap().is_some()
}

#[tokio::test]
async fn creates_tables_idempotently() {
    let conn = doido_model::sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap();

    // Running twice must not error.
    doido_storage::ensure_tables(&conn).await.unwrap();
    doido_storage::ensure_tables(&conn).await.unwrap();

    assert!(table_exists(&conn, "storage_blobs").await);
    assert!(table_exists(&conn, "storage_attachments").await);
    assert!(table_exists(&conn, "storage_variant_records").await);
}
