use doido_model::validation::{Errors, Validate};

struct Post {
    title: String,
    body: String,
}

impl Validate for Post {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e.length("body", &self.body, Some(10), None);
        e
    }
}

#[test]
fn a_valid_record_has_no_errors() {
    let p = Post {
        title: "Hi".into(),
        body: "a long enough body".into(),
    };
    assert!(p.is_valid());
    assert!(p.validate().is_empty());
}

#[test]
fn presence_flags_a_blank_field() {
    let p = Post {
        title: "   ".into(),
        body: "a long enough body".into(),
    };
    let e = p.validate();
    assert!(!e.is_empty());
    assert_eq!(e.on("title"), vec!["can't be blank"]);
    assert!(e
        .full_messages()
        .contains(&"title can't be blank".to_string()));
}

#[test]
fn length_flags_a_too_short_field() {
    let p = Post {
        title: "Hi".into(),
        body: "short".into(),
    };
    assert!(!p.is_valid());
    assert!(p
        .validate()
        .full_messages()
        .contains(&"body is too short (minimum is 10 characters)".to_string()));
}

#[test]
fn errors_can_be_added_and_counted() {
    let mut e = Errors::new();
    e.add("base", "something went wrong");
    assert_eq!(e.len(), 1);
    assert_eq!(e.on("base"), vec!["something went wrong"]);
}
