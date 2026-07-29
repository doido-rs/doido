use doido_model::create_database;
use tempfile::TempDir;

#[tokio::test]
async fn create_sqlite_database_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("db/dev.db");
    let url = format!("sqlite://{}", path.display());
    create_database(&url).await.unwrap();
    assert!(path.exists());
    create_database(&url).await.unwrap();
}

#[tokio::test]
async fn create_in_memory_sqlite_succeeds() {
    create_database("sqlite::memory:").await.unwrap();
}

#[tokio::test]
async fn create_database_rejects_invalid_url() {
    let err = create_database("not-a-url").await;
    assert!(err.is_err());
}
