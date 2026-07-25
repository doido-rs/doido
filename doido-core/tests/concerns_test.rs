use doido_core::concerns::Sluggable;

struct Post {
    title: String,
}
// "Including" the concern: implement only the required method.
impl Sluggable for Post {
    fn sluggable_source(&self) -> &str {
        &self.title
    }
}

#[test]
fn a_concern_provides_shared_default_behavior() {
    let post = Post {
        title: "  Hello   Wonderful World ".to_string(),
    };
    assert_eq!(post.slug(), "hello-wonderful-world");
}
