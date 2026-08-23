//! `doido db schema diagram` — ER diagram HTML export.

use crate::common::db;
use crate::common::{AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn schema_diagram_exports_html_with_full_metadata() {
    let h = AppHarness::new("schema_diagram", BaseProfile::Default);
    h.generate(&["generate", "resource", "Post", "title:string", "body:text"]);
    h.generate(&[
        "generate",
        "resource",
        "Comment",
        "body:text",
        "post:references",
    ]);
    h.generate(&[
        "generate",
        "resource",
        "Sku",
        "code:string:unique:not_null",
        "qty:integer:index",
        "active:boolean",
    ]);
    h.build();

    let bin = h.bin();
    db::prepare_database(&bin, &h.app);
    db::assert_table_exists(&h.app, "posts");
    db::assert_table_exists(&h.app, "comments");
    db::assert_table_exists(&h.app, "skus");

    db::schema_diagram(&bin, &h.app);

    // Scaffold migrations add reference columns but not SQLite FK constraints;
    // add a tiny schema with an explicit FK to validate relationship export.
    db::exec_sqlite(
        &h.app,
        "CREATE TABLE diagram_authors (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
    );
    db::exec_sqlite(
        &h.app,
        "CREATE TABLE diagram_books (id INTEGER PRIMARY KEY, author_id INTEGER NOT NULL REFERENCES diagram_authors(id))",
    );
    db::schema_diagram(&bin, &h.app);

    let html_path = db::schema_diagram_file(&h.app);
    assert!(
        html_path.is_file(),
        "db schema diagram should write {}",
        html_path.display()
    );

    let html = std::fs::read_to_string(&html_path).unwrap_or_else(|e| {
        panic!("read {}: {e}", html_path.display());
    });
    assert!(html.contains("id=\"doido-schema-design\""));
    assert!(html.contains("class=\"badge pk\""));
    assert!(html.contains("class=\"badge fk\""));
    assert!(html.contains("data-tooltip"));

    let schema = db::parse_schema_design_json(&html);
    let tables = schema["tables"]
        .as_array()
        .expect("tables array in embedded schema json");
    let table_names: Vec<&str> = tables
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(
        table_names.iter().any(|n| *n == "comments"),
        "expected comments table, got {table_names:?}"
    );
    assert!(
        table_names.iter().any(|n| *n == "posts"),
        "expected posts table, got {table_names:?}"
    );
    assert!(
        table_names.iter().any(|n| *n == "skus"),
        "expected skus table, got {table_names:?}"
    );

    let comments = tables
        .iter()
        .find(|t| t["name"] == "comments")
        .expect("comments table in schema json");
    assert!(
        comments["columns"]
            .as_array()
            .and_then(|cols| cols.iter().find(|c| c["name"] == "post_id"))
            .is_some(),
        "comments.post_id column should be exported"
    );

    let books = tables
        .iter()
        .find(|t| t["name"] == "diagram_books")
        .expect("diagram_books table in schema json");
    let book_fks = books["foreign_keys"].as_array().expect("diagram_books fks");
    assert!(
        book_fks.iter().any(|fk| {
            fk["columns"]
                .as_array()
                .map(|cols| cols.iter().any(|c| c == "author_id"))
                .unwrap_or(false)
                && fk["referenced_table"] == "diagram_authors"
        }),
        "expected author_id foreign key on diagram_books -> diagram_authors"
    );

    let skus = tables
        .iter()
        .find(|t| t["name"] == "skus")
        .expect("skus table in schema json");
    let sku_has_qty_index = skus["indexes"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(
            skus["constraints"]
                .as_array()
                .into_iter()
                .flatten(),
        )
        .any(|idx| {
            idx["columns"]
                .as_array()
                .map(|cols| cols.iter().any(|c| c == "qty"))
                .unwrap_or(false)
        });
    assert!(sku_has_qty_index, "expected index on skus.qty");
    let code_col = skus["columns"]
        .as_array()
        .and_then(|cols| cols.iter().find(|c| c["name"] == "code"))
        .expect("code column on skus");
    assert_eq!(code_col["unique"], true);
}
