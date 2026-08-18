use doido_model::schema::{dump, dump_to_file, load, write_file};
use doido_model::sea_orm::ConnectionTrait;
use doido_model::testing::TestDb;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn dump_then_load_recreates_the_schema() {
    let src = TestDb::new().await.unwrap();
    src.conn()
        .execute_unprepared("CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT)")
        .await
        .unwrap();

    let schema = dump(src.conn()).await.unwrap();
    assert!(schema.contains("CREATE TABLE"), "{schema}");
    assert!(schema.contains("widgets"));

    // Load into a fresh database and use the table.
    let dest = TestDb::new().await.unwrap();
    load(dest.conn(), &schema).await.unwrap();
    dest.conn()
        .execute_unprepared("INSERT INTO widgets (id, name) VALUES (1, 'ok')")
        .await
        .expect("table exists after load");
}

#[test]
fn write_file_creates_schema_when_missing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("db/schema.sql");
    let schema = "CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT);\n";

    write_file(&path, schema).unwrap();

    assert!(path.is_file());
    assert_eq!(fs::read_to_string(&path).unwrap(), schema);
}

#[test]
fn write_file_overwrites_existing_schema_completely() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("db/schema.sql");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "CREATE TABLE stale (id INTEGER PRIMARY KEY);\n").unwrap();

    let schema = "CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT);\n";
    write_file(&path, schema).unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), schema);
    let rendered = fs::read_to_string(&path).unwrap();
    assert!(!rendered.contains("stale"));
}

#[tokio::test]
async fn dump_to_file_creates_or_rewrites_schema_sql() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("db/schema.sql");
    let db = TestDb::new().await.unwrap();
    db.conn()
        .execute_unprepared("CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT)")
        .await
        .unwrap();

    dump_to_file(db.conn(), &path).await.unwrap();
    let first = fs::read_to_string(&path).unwrap();
    assert!(first.contains("CREATE TABLE widgets"));

    db.conn()
        .execute_unprepared("CREATE TABLE gadgets (id INTEGER PRIMARY KEY)")
        .await
        .unwrap();

    dump_to_file(db.conn(), &path).await.unwrap();
    let second = fs::read_to_string(&path).unwrap();
    assert!(second.contains("CREATE TABLE gadgets"));
    assert!(second.contains("CREATE TABLE widgets"));
    assert_ne!(first, second);
}
