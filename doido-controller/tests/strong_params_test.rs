use doido_controller::params::Params;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, PartialEq)]
struct PostForm {
    title: String,
    body: String,
}

#[test]
fn require_returns_nested_params() {
    let p = Params::new(json!({"post": {"title": "Hi", "body": "x"}}));
    let post = p.require("post").expect("post key present");
    assert_eq!(post.get("title").unwrap(), "Hi");
}

#[test]
fn require_missing_key_is_error() {
    let p = Params::new(json!({"other": {}}));
    assert!(p.require("post").is_err());
}

#[test]
fn permit_drops_unlisted_keys() {
    let p = Params::new(json!({"post": {"title": "Hi", "body": "x", "admin": true}}));
    let permitted = p.require("post").unwrap().permit(&["title", "body"]);
    assert!(
        permitted.get("admin").is_none(),
        "mass-assignment key filtered out"
    );
    assert_eq!(permitted.get("title").unwrap(), "Hi");
    assert_eq!(permitted.get("body").unwrap(), "x");
}

#[test]
fn permitted_params_deserialize_into_a_struct() {
    let p = Params::new(json!({"post": {"title": "Hi", "body": "x", "admin": true}}));
    let form: PostForm = p
        .require("post")
        .unwrap()
        .permit(&["title", "body"])
        .deserialize()
        .unwrap();
    assert_eq!(
        form,
        PostForm {
            title: "Hi".into(),
            body: "x".into()
        }
    );
}
