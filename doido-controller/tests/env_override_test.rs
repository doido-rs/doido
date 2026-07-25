use doido_controller::env_override::apply_env_overrides;
use serde_json::json;

#[test]
fn section_key_overrides_apply_and_coerce() {
    let mut config = json!({ "server": { "port": 3000 }, "logger": { "level": "info" } });
    apply_env_overrides(
        &mut config,
        &[
            ("SERVER__PORT".into(), "4000".into()),
            ("LOGGER__LEVEL".into(), "debug".into()),
            ("LOGGER__SQL".into(), "true".into()),
            ("CACHE__NAMESPACE".into(), "myapp".into()), // new section
        ],
    );

    assert_eq!(config["server"]["port"], 4000);
    assert_eq!(config["logger"]["level"], "debug");
    assert_eq!(config["logger"]["sql"], true);
    assert_eq!(config["cache"]["namespace"], "myapp");
}
