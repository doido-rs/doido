use doido_generators::{Generator, ResourceGenerator};

#[test]
fn resource_generates_model_controller_routes_but_no_views() {
    let files = ResourceGenerator
        .generate(&["Widget", "name:string"])
        .unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    assert!(
        paths.iter().any(|p| p.contains("app/models/widget.rs")),
        "{paths:?}"
    );
    assert!(paths.iter().any(|p| p.contains("widgets_controller.rs")));
    assert!(paths.contains(&"config/routes.rs"));
    assert!(
        !paths.iter().any(|p| p.contains("app/views/")),
        "a resource has no views: {paths:?}"
    );
}
