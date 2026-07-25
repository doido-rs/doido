use doido_view::helpers::link::{button_to, link_to, link_to_class};

#[test]
fn link_to_builds_an_anchor() {
    assert_eq!(link_to("Home", "/"), "<a href=\"/\">Home</a>");
    // values are escaped
    assert!(link_to("A & B", "/x?a=1&b=2").contains("A &amp; B"));
    assert!(link_to("x", "/x?a=1&b=2").contains("a=1&amp;b=2"));
}

#[test]
fn link_to_class_adds_a_class() {
    assert_eq!(
        link_to_class("Go", "/go", "btn"),
        "<a href=\"/go\" class=\"btn\">Go</a>"
    );
}

#[test]
fn button_to_tunnels_delete() {
    let b = button_to("Delete", "/posts/1", "delete");
    assert!(b.contains("name=\"_method\" value=\"DELETE\""));
    assert!(b.contains("<button type=\"submit\">Delete</button>"));
}
