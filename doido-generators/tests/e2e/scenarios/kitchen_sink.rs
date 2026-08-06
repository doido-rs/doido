//! Kitchen-sink app: every generator, one build, migrations clean, HTTP smoke.

use crate::common::cli::doido;
use crate::common::db;
use crate::common::http;
use crate::common::{AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn every_generator_compiles_migrates_and_serves() {
    let h = AppHarness::new("kitchen_sink", BaseProfile::WithCable);

    let generators: [&[&str]; 14] = [
        &[
            "generate",
            "scaffold",
            "Post",
            "title:string",
            "body:text",
            "--api",
        ],
        &["generate", "model", "Comment", "body:text"],
        &["generate", "controller", "Pages"],
        &["generate", "helper", "Posts"],
        &["generate", "job", "SendEmail"],
        &["generate", "mailer", "UserMailer"],
        &["generate", "channel", "ChatRoom"],
        &[
            "generate",
            "migration",
            "add_views_to_posts",
            "views:integer",
        ],
        &[
            "generate",
            "migration",
            "remove_views_from_posts",
            "views:integer",
        ],
        &["generate", "storage:install"],
        &["generate", "storage:adapter", "Cloud"],
        &["generate", "locale", "fr"],
        &["generate", "templates"],
        &["generate", "generator", "widget"],
    ];
    for args in generators {
        doido(&h.app).args(args).assert().success();
    }

    h.run_with_db(
        |h| {
            db::assert_table_exists(&h.app, "posts");
            db::assert_table_exists(&h.app, "comments");
            db::assert_table_exists(&h.app, "storage_blobs");
            assert!(h.app.join("config/locales/fr.yml").is_file());
            assert!(h.app.join("lib/generators/widget").is_dir());
        },
        |app| {
            assert_eq!(http::get_status(&format!("{}/", app.base_url)), 200);
            http::api_crud_cycle(
                &app.base_url,
                "posts",
                serde_json::json!({ "title": "Kitchen", "body": "sink" }),
                serde_json::json!({ "title": "Updated", "body": "sink" }),
            );
        },
    );
}
