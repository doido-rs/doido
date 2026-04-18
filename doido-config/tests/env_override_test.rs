use doido_config::env_override::apply_overrides_from;

fn empty_table() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}

#[test]
fn test_sets_integer_value() {
    let mut v = empty_table();
    apply_overrides_from(
        &mut v,
        vec![("SERVER__PORT".to_string(), "9090".to_string())].into_iter(),
    );
    assert_eq!(v["server"]["port"].as_integer(), Some(9090));
}

#[test]
fn test_sets_string_value() {
    let mut v = empty_table();
    apply_overrides_from(
        &mut v,
        vec![("LOG__LEVEL".to_string(), "debug".to_string())].into_iter(),
    );
    assert_eq!(v["log"]["level"].as_str(), Some("debug"));
}

#[test]
fn test_sets_boolean_value() {
    let mut v = empty_table();
    apply_overrides_from(
        &mut v,
        vec![("VIEW__HOT_RELOAD".to_string(), "false".to_string())].into_iter(),
    );
    assert_eq!(v["view"]["hot_reload"].as_bool(), Some(false));
}

#[test]
fn test_ignores_single_underscore_vars() {
    let mut v = empty_table();
    apply_overrides_from(
        &mut v,
        vec![
            ("DOIDO_ENV".to_string(), "test".to_string()),
            ("PATH".to_string(), "/usr/bin".to_string()),
        ].into_iter(),
    );
    assert!(v.as_table().unwrap().is_empty());
}

#[test]
fn test_ignores_empty_segment_from_trailing_double_underscore() {
    let mut v = empty_table();
    apply_overrides_from(
        &mut v,
        vec![("SERVER__".to_string(), "foo".to_string())].into_iter(),
    );
    assert!(v.as_table().unwrap().is_empty());
}

#[test]
fn test_supports_three_level_nesting() {
    let mut v = empty_table();
    apply_overrides_from(
        &mut v,
        vec![("A__B__C".to_string(), "42".to_string())].into_iter(),
    );
    assert_eq!(v["a"]["b"]["c"].as_integer(), Some(42));
}

#[test]
fn test_overrides_existing_value() {
    let mut v: toml::Value = toml::from_str("[server]\nport = 3000").unwrap();
    apply_overrides_from(
        &mut v,
        vec![("SERVER__PORT".to_string(), "8080".to_string())].into_iter(),
    );
    assert_eq!(v["server"]["port"].as_integer(), Some(8080));
}
