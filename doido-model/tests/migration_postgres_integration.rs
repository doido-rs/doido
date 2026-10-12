//! Optional PostgreSQL integration checks for the migration DSL.
//!
//! Run with: `DOIDO_TEST_DATABASE_URL=postgres://… cargo test -p doido-model --features postgres migration_postgres_integration -- --ignored`

use doido_model::migration::{create_index, enable_extension, IndexColumn, IndexOpts};
use doido_model::sea_orm::sea_query::Expr;
use doido_model::sea_orm::{ConnectOptions, Database, DatabaseConnection};
use doido_model::SchemaManager;

async fn postgres_conn() -> Option<DatabaseConnection> {
    let url = std::env::var("DOIDO_TEST_DATABASE_URL").ok()?;
    let mut opts = ConnectOptions::new(url);
    opts.sqlx_logging(false);
    Database::connect(opts).await.ok()
}

#[tokio::test]
#[ignore = "requires DOIDO_TEST_DATABASE_URL and postgres feature"]
async fn enable_extension_is_idempotent_on_postgres() {
    let Some(conn) = postgres_conn().await else {
        return;
    };
    let manager = SchemaManager::new(&conn);
    enable_extension(&manager, "pgcrypto").await.unwrap();
    enable_extension(&manager, "pgcrypto").await.unwrap();
}

#[tokio::test]
#[ignore = "requires DOIDO_TEST_DATABASE_URL and postgres feature"]
async fn partial_unique_index_on_postgres() {
    let Some(conn) = postgres_conn().await else {
        return;
    };
    let manager = SchemaManager::new(&conn);
    doido_model::migration::create_table(&manager, "partial_idx_demo", |t| {
        t.string("token");
    })
    .await
    .unwrap();
    create_index(
        &manager,
        "partial_idx_demo",
        &[IndexColumn::new("token")],
        IndexOpts {
            unique: true,
            where_expr: Some(Expr::col("token").is_not_null()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    doido_model::migration::drop_table(&manager, "partial_idx_demo")
        .await
        .unwrap();
}
