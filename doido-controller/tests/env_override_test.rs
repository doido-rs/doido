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

#[test]
fn coerces_float_and_false_bool() {
    let mut config = json!({});
    apply_env_overrides(
        &mut config,
        &[
            ("CACHE__TTL".into(), "2.5".into()),
            ("LOGGER__SQL".into(), "false".into()),
        ],
    );
    assert_eq!(config["cache"]["ttl"], 2.5);
    assert_eq!(config["logger"]["sql"], false);
}

#[test]
fn skips_vars_without_double_underscore() {
    let mut config = json!({ "server": { "port": 3000 } });
    apply_env_overrides(&mut config, &[("PORT".into(), "9999".into())]);
    assert_eq!(config["server"]["port"], 3000);
}

#[test]
fn non_object_root_is_left_unchanged() {
    let mut config = json!("plain");
    apply_env_overrides(&mut config, &[("SERVER__PORT".into(), "4000".into())]);
    assert_eq!(config, json!("plain"));
}

#[test]
fn from_process_env_reads_section_key_vars() {
    let key = "COVERAGE_TEST__PORT";
    std::env::set_var(key, "8765");
    let mut config = json!({});
    doido_controller::env_override::from_process_env(&mut config);
    std::env::remove_var(key);
    assert_eq!(config["coverage_test"]["port"], 8765);
}
