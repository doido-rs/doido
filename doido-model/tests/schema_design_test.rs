//! Schema design introspection and HTML ER diagram export.

use doido_model::schema_design::{
    export_html, introspect_from_url, resolve_ignore_tables, SchemaDesign, TableDesign,
};
use doido_model::sea_orm::ConnectionTrait;

const BLOG_SCHEMA: &str = r#"
CREATE TABLE authors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT NOT NULL
);
CREATE UNIQUE INDEX index_authors_on_email ON authors(email);
CREATE TABLE posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    author_id INTEGER NOT NULL REFERENCES authors(id)
);
CREATE INDEX index_posts_on_author_id ON posts(author_id);
"#;

async fn load_blog_schema(name: &str) -> String {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join(format!("{name}.db"));
    let url = format!("sqlite://{}", path.display());
    doido_model::create_database(&url).await.unwrap();
    let conn = doido_model::sea_orm::Database::connect(&url).await.unwrap();
    for statement in BLOG_SCHEMA.split(';') {
        let sql = statement.trim();
        if sql.is_empty() {
            continue;
        }
        conn.execute_unprepared(sql).await.unwrap();
    }
    conn.close().await.unwrap();
    std::mem::forget(dir);
    url
}

fn table<'a>(schema: &'a SchemaDesign, name: &str) -> &'a TableDesign {
    schema
        .tables
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("table `{name}` not found"))
}

#[tokio::test]
async fn introspect_sqlite_builds_abstract_schema() {
    let file_url = load_blog_schema("schema_design_introspect").await;

    let ignore = resolve_ignore_tables(&[]);
    let schema = introspect_from_url(&file_url, None, &ignore).await.unwrap();

    assert_eq!(schema.tables.len(), 2);
    let authors = table(&schema, "authors");
    assert_eq!(authors.primary_key.columns, vec!["id"]);
    assert!(authors.primary_key.autoincrement);
    assert!(authors
        .columns
        .iter()
        .any(|c| c.name == "email" && c.unique));

    let author_index = authors
        .indexes
        .iter()
        .find(|i| i.unique && i.columns == ["email"])
        .expect("unique index on email");
    assert!(author_index.unique);
    assert_eq!(author_index.columns, vec!["email"]);

    let posts = table(&schema, "posts");
    let author_fk = posts
        .foreign_keys
        .iter()
        .find(|fk| fk.columns == ["author_id"])
        .expect("author_id foreign key");
    assert_eq!(author_fk.referenced_table, "authors");
    assert_eq!(author_fk.referenced_columns, vec!["id"]);

    let author_id = posts
        .columns
        .iter()
        .find(|c| c.name == "author_id")
        .unwrap();
    assert!(author_id.foreign_key);
    assert!(!author_id.primary_key);

    assert!(
        !posts.indexes.is_empty() || !posts.foreign_keys.is_empty(),
        "posts should expose indexes or foreign keys"
    );
}

#[tokio::test]
async fn export_html_embeds_parseable_schema_json() {
    let file_url = load_blog_schema("schema_design_export").await;

    let ignore = resolve_ignore_tables(&[]);
    let schema = introspect_from_url(&file_url, None, &ignore).await.unwrap();
    let html = export_html(&schema).unwrap();

    assert!(html.contains("id=\"doido-schema-design\""));
    assert!(html.contains("class=\"badge pk\""));
    assert!(html.contains("class=\"badge fk\""));
    assert!(html.contains("data-tooltip"));

    let json = extract_embedded_json(&html);
    let parsed: SchemaDesign = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.tables.len(), 2);
    assert_eq!(table(&parsed, "posts").foreign_keys.len(), 1);
}

fn extract_embedded_json(html: &str) -> String {
    let marker = r#"<script type="application/json" id="doido-schema-design">"#;
    let start = html.find(marker).expect("embedded schema json marker") + marker.len();
    let rest = &html[start..];
    let end = rest.find("</script>").expect("closing script tag");
    rest[..end].to_string()
}
