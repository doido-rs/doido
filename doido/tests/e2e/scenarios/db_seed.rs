//! `doido db seed` inserts fixture rows in-process (via `db/seeds.rs`) and serves
//! them over HTTP.

use crate::common::db;
use crate::common::http;
use crate::common::server;
use crate::common::{AppHarness, BaseProfile};
use std::time::Duration;

/// A `db/seeds.rs` module: runs in-process from the app binary against the `db`
/// connection the CLI passes in, using the app's own `crate::models`.
const ARTICLE_SEEDS: &str = r#"//! Database seeds — run with `doido db seed`.

use doido::model::sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

pub async fn run(db: &DatabaseConnection) -> doido::Result<()> {
    use crate::models::article::{ActiveModel, Entity};
    if Entity::find().one(db).await?.is_none() {
        ActiveModel {
            title: Set("Seeded headline".into()),
            body: Set(Some("from db/seed".into())),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
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
    std::fs::write(h.app.join("db/seeds.rs"), ARTICLE_SEEDS).unwrap();

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
