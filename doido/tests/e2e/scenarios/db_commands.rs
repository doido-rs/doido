//! `doido db` task commands — especially `db schema dump|load`, plus reset/prepare.

use crate::common::db;
use crate::common::{AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn db_schema_dump_load_reset_and_prepare() {
    let h = AppHarness::new("db_commands", BaseProfile::Default);
    h.generate(&[
        "generate",
        "scaffold",
        "Post",
        "title:string",
        "body:text",
        "--api",
    ]);
    h.build();

    let bin = h.bin();
    db::prepare_database(&bin, &h.app);
    db::assert_table_exists(&h.app, "posts");

    db::schema_dump(&bin, &h.app);
    assert!(
        db::schema_file(&h.app).is_file(),
        "db schema dump should write db/schema.sql"
    );
    db::assert_schema_contains(&h.app, "posts");
    db::assert_schema_contains(&h.app, "CREATE TABLE");

    db::exec_sqlite(
        &h.app,
        "INSERT INTO posts (title, body) VALUES ('before reset', 'fixture')",
    );
    db::assert_row_count(&h.app, "posts", 1);

    db::db_reset(&bin, &h.app);
    db::assert_table_exists(&h.app, "posts");
    db::assert_row_count(&h.app, "posts", 0);

    db::exec_sqlite(
        &h.app,
        "INSERT INTO posts (title, body) VALUES ('prepare noop', 'row')",
    );
    db::assert_row_count(&h.app, "posts", 1);
    db::db_prepare(&bin, &h.app);
    db::assert_row_count(&h.app, "posts", 1);

    db::remove_sqlite_database(&h.app);
    db::create_database(&bin, &h.app);
    db::db_prepare(&bin, &h.app);
    db::assert_table_exists(&h.app, "posts");
    db::assert_row_count(&h.app, "posts", 0);

    db::remove_sqlite_database(&h.app);
    db::create_database(&bin, &h.app);
    db::schema_load(&bin, &h.app);
    db::assert_table_exists(&h.app, "posts");
    db::assert_table_exists(&h.app, "storage_blobs");
    db::assert_row_count(&h.app, "posts", 0);
}
