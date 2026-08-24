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
        "published:boolean:not_null",
    ]);
    h.run_with_db(
        |h| {
            db::assert_table_exists(&h.app, "articles");
            db::assert_column_exists(&h.app, "articles", "title");
            db::assert_column_exists(&h.app, "articles", "published");
        },
        |app| {
            let create_url = format!("{}/articles", app.base_url);

            // Unchecked checkbox is omitted from the body; `#[serde(default)]` → false.
            let draft = http::post_form(&create_url, &[("title", "Draft"), ("body", "content")]);
            assert!(
                draft == 200 || (300..400).contains(&draft),
                "draft create should succeed or redirect, got {draft}"
            );

            let published = http::post_form(
                &create_url,
                &[
                    ("title", "News"),
                    ("body", "content"),
                    ("published", "true"),
                ],
            );
            assert!(
                published == 200 || (300..400).contains(&published),
                "published create should succeed or redirect, got {published}"
            );

            let index = http::get_text(&format!("{}/articles", app.base_url));
            assert!(index.contains("Draft"), "index should list draft article");
            assert!(
                index.contains("News"),
                "index should list published article"
            );

            let form = http::get_text(&format!("{}/articles/1/edit", app.base_url));
            assert!(
                form.contains("value=\"true\"") && form.contains("name=\"published\""),
                "edit form should render checkbox with value=true"
            );
            assert!(
                form.contains("action=\"/articles/1\""),
                "edit form should POST to the record URL, got: {form}"
            );
            assert!(
                form.contains("name=\"_method\" value=\"patch\""),
                "edit form should tunnel PATCH via _method"
            );
            assert!(
                form.contains(">Update</button>"),
                "edit form submit button should say Update"
            );
            assert!(
                form.contains("value=\"Draft\"") || form.contains("value=\"News\""),
                "edit form should prefill existing field values"
            );

            let update_url = format!("{}/articles/1", app.base_url);
            let updated = http::post_form(
                &update_url,
                &[
                    ("_method", "patch"),
                    ("title", "Revised"),
                    ("body", "updated body"),
                    ("published", "true"),
                ],
            );
            assert!(
                updated == 200 || (300..400).contains(&updated),
                "edit form submit should update the record, got {updated}"
            );

            let index_after = http::get_text(&format!("{}/articles", app.base_url));
            assert!(
                index_after.contains("Revised"),
                "index should show updated title after edit submit"
            );
        },
    );
}
