//! `AuthScaffoldGenerator` across many field types and both HTML and `--api`
//! modes — covers `render_controller`/`render_view`/`model_fields` and the
//! api-vs-html branch of `generate`.

use doido_auth::generators::{AuthGenerator, AuthScaffoldGenerator};

#[test]
fn html_scaffold_renders_every_field_type() {
    let files = AuthScaffoldGenerator
        .generate(&[
            "Report",
            "title:string",
            "body:text",
            "count:integer",
            "active:boolean",
            "price:decimal",
            "due:date",
            "seen_at:timestamp",
            "author:references",
        ])
        .unwrap();

    let migration = files
        .iter()
        .find(|f| f.path.contains("create_reports_table"))
        .expect("reports migration");
    for col in [
        "t.string(\"title\")",
        "t.text(\"body\")",
        "t.integer(\"count\")",
        "t.boolean(\"active\")",
        "t.decimal(\"price\")",
        "t.date(\"due\")",
        "t.timestamp(\"seen_at\")",
        "references(\"author\")",
        "references(\"user\")",
    ] {
        assert!(migration.content.contains(col), "migration missing {col}");
    }

    let form = files
        .iter()
        .find(|f| f.path == "app/views/reports/_form.html.tera")
        .expect("form partial");
    assert!(form.content.contains("<textarea"), "text field → textarea");
    assert!(
        form.content.contains("type=\"checkbox\""),
        "boolean field → checkbox"
    );
    assert!(
        form.content.contains("type=\"number\""),
        "numeric field → number input"
    );

    // The five CRUD views are all emitted in HTML mode.
    for view in ["index", "show", "new", "edit", "_form"] {
        assert!(
            files
                .iter()
                .any(|f| f.path == format!("app/views/reports/{view}.html.tera")),
            "missing view {view}"
        );
    }

    let model = files
        .iter()
        .find(|f| f.path == "app/models/report.rs")
        .expect("model extension");
    assert!(!model.content.is_empty());
}

#[test]
fn api_scaffold_skips_views_and_uses_api_controller() {
    let files = AuthScaffoldGenerator
        .generate(&["Gadget", "name:string", "--api"])
        .unwrap();

    assert!(
        !files.iter().any(|f| f.path.starts_with("app/views/")),
        "API scaffold must not emit views"
    );
    let controller = files
        .iter()
        .find(|f| f.path == "app/controllers/gadgets_controller.rs")
        .expect("api controller");
    assert!(controller.content.contains("require_user"));

    // Routes injected without the form (new/edit) routes in API mode.
    let routes = files
        .iter()
        .find(|f| f.path.ends_with("routes.rs"))
        .expect("routes.rs");
    assert!(routes.content.contains("gadgets"));
}
