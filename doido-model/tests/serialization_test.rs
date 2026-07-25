use doido_model::serialization::{
    as_json_except, as_json_only, deserialize_column, serialize_column,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct User {
    id: i32,
    name: String,
    password_digest: String,
}

#[test]
fn as_json_except_hides_keys() {
    let u = User {
        id: 1,
        name: "Ada".into(),
        password_digest: "secret".into(),
    };
    let j = as_json_except(&u, &["password_digest"]);
    assert!(j.get("password_digest").is_none(), "sensitive key removed");
    assert_eq!(j["name"], "Ada");
    assert_eq!(j["id"], 1);
}

#[test]
fn as_json_only_keeps_listed_keys() {
    let u = User {
        id: 1,
        name: "Ada".into(),
        password_digest: "secret".into(),
    };
    let j = as_json_only(&u, &["id", "name"]);
    assert_eq!(j.as_object().unwrap().len(), 2);
    assert_eq!(j["id"], 1);
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Prefs {
    theme: String,
    notify: bool,
}

#[test]
fn serialized_column_round_trips() {
    let prefs = Prefs {
        theme: "dark".into(),
        notify: true,
    };
    let stored = serialize_column(&prefs).unwrap();
    let loaded: Prefs = deserialize_column(&stored).unwrap();
    assert_eq!(loaded, prefs);
}
