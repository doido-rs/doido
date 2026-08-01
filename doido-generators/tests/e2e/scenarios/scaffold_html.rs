//! HTML scaffold: form POST and rendered index page over HTTP.

use crate::common::db;
use crate::common::http;
use crate::common::{AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn scaffold_html_form_and_index() {
    let h = AppHarness::new("scaffold_html", BaseProfile::Default);
    h.generate(&[
        "generate",
        "scaffold",
        "Article",
        "title:string:not_null",
        "body:text",
    ]);
    h.run_with_db(
        |h| {
            db::assert_table_exists(&h.app, "articles");
            db::assert_column_exists(&h.app, "articles", "title");
        },
        |app| {
            let status = http::post_form(
                &format!("{}/articles", app.base_url),
                &[("title", "News"), ("body", "content")],
            );
            assert!(
                status == 200 || (300..400).contains(&status),
                "create should succeed or redirect, got {status}"
            );
            let index = http::get_text(&format!("{}/articles", app.base_url));
            assert!(index.contains("News"), "index should list created article");
        },
    );
}
