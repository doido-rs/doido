use doido_view::helpers::asset::{
    image_tag, image_tag_alt, javascript_include_tag, stylesheet_link_tag,
};

#[test]
fn image_tag_serves_from_assets() {
    assert_eq!(image_tag("logo.png"), "<img src=\"/assets/logo.png\">");
    assert_eq!(
        image_tag("/uploads/x.png"),
        "<img src=\"/uploads/x.png\">",
        "rooted paths pass through"
    );
    assert!(image_tag_alt("logo.png", "Logo").contains("alt=\"Logo\""));
}

#[test]
fn stylesheet_and_javascript_tags_add_extensions() {
    assert_eq!(
        stylesheet_link_tag("application"),
        "<link rel=\"stylesheet\" href=\"/assets/application.css\">"
    );
    assert_eq!(
        javascript_include_tag("application"),
        "<script src=\"/assets/application.js\"></script>"
    );
    // already-suffixed names aren't doubled
    assert!(stylesheet_link_tag("app.css").contains("/assets/app.css\""));
}
