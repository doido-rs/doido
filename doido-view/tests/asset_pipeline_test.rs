use doido_view::helpers::asset::{digest, digested_path};

#[test]
fn digested_paths_bust_the_cache_on_change() {
    let p1 = digested_path("app.css", b"body { color: red }");
    assert!(p1.starts_with("/assets/app-"), "{p1}");
    assert!(p1.ends_with(".css"));

    // same content -> same path; changed content -> different path
    assert_eq!(digest(b"x"), digest(b"x"));
    assert_ne!(digest(b"x"), digest(b"y"));
    assert_ne!(
        digested_path("app.css", b"a"),
        digested_path("app.css", b"b")
    );
}
