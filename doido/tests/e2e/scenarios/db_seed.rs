//! `doido db seed` inserts fixture rows via the `db/seed` crate and serves them over HTTP.

use crate::common::db;
use crate::common::http;
use crate::common::server;
use crate::common::{AppHarness, BaseProfile};
use std::time::Duration;

const ARTICLE_SEED_MAIN: &str = r#"//! Database seeds — run with `doido db seed`.

#[path = "../../../app/models/mod.rs"]
mod models;

use doido::model::sea_orm::{ActiveModelTrait, EntityTrait, Set};
use doido::model::{config::YamlConfig, connect_with_url};

async fn run_seed() -> doido::Result<()> {
    let url = std::env::var("DATABASE_URL")
        .or_else(|_| YamlConfig::load().map(|c| c.database.url))
        .map_err(|e| doido::core::anyhow::anyhow!("{e}"))?;
    let db = connect_with_url(&url).await?;

    use models::article::{ActiveModel, Entity};
    if Entity::find().one(&db).await?.is_none() {
        ActiveModel {
            title: Set("Seeded headline".into()),
            body: Set(Some("from db/seed".into())),
            ..Default::default()
        }
        .insert(&db)
        .await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_seed().await {
        eprintln!("seed failed: {e}");
        std::process::exit(1);
    }
    println!("seed complete");
}
"#;

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn db_seed_inserts_models_and_serves_index() {
    let h = AppHarness::new("db_seed", BaseProfile::Default);
    h.generate(&[
        "generate",
        "scaffold",
        "Article",
        "title:string:not_null",
        "body:text",
    ]);
    std::fs::write(h.app.join("db/seed/src/main.rs"), ARTICLE_SEED_MAIN).unwrap();

    h.configure_server();
    h.build();
    h.prepare_database();
    h.seed_database();

    db::assert_table_exists(&h.app, "articles");
    db::assert_row_count(&h.app, "articles", 1);

    let base_url = format!("http://127.0.0.1:{}", h.port());
    let running = server::spawn(&h.bin(), &h.app);
    server::wait_until_http_ok(&format!("{base_url}/"), Duration::from_secs(60));

    let index = http::get_text(&format!("{base_url}/articles"));
    assert!(
        index.contains("Seeded headline"),
        "index should list the seeded article"
    );
    running.shutdown();
}
