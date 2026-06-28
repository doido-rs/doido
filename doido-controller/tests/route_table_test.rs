//! Verifies the `routes!` macro registers the global route table that
//! `doido server` / `doido routes` print. Kept in its own test binary so no
//! parallel sibling test overwrites the global table mid-assertion.

mod posts_controller {
    pub async fn index() -> &'static str {
        "index"
    }
    pub async fn new() -> &'static str {
        "new"
    }
    pub async fn create() -> &'static str {
        "create"
    }
    pub async fn show() -> &'static str {
        "show"
    }
    pub async fn edit() -> &'static str {
        "edit"
    }
    pub async fn update() -> &'static str {
        "update"
    }
    pub async fn destroy() -> &'static str {
        "destroy"
    }
}

async fn about_handler() -> &'static str {
    "about"
}

#[test]
fn routes_macro_registers_full_table() {
    let _app: axum::Router = doido_controller::routes! {
        get!("/about", about_handler)
        resources!(posts, posts_controller)
    };

    let routes = doido_controller::all_routes();
    let has =
        |method: &str, path: &str| routes.iter().any(|r| r.method == method && r.path == path);

    assert!(has("GET", "/about"));
    assert!(has("GET", "/posts"));
    assert!(has("POST", "/posts"));
    assert!(has("GET", "/posts/new"));
    assert!(has("GET", "/posts/{id}"));
    assert!(has("PUT|PATCH", "/posts/{id}"));
    assert!(has("DELETE", "/posts/{id}"));
    assert!(has("GET", "/posts/{id}/edit"));

    // The formatted table includes a header and the method/path pairs.
    let table = doido_controller::route_table::format_routes();
    assert!(table.contains("METHOD"));
    assert!(table.contains("/posts/{id}/edit"));
}
