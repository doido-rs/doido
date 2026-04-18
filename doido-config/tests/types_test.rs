use doido_config::Config;

#[test]
fn test_config_deserializes_all_sections() {
    let toml_str = r#"
[server]
port = 8080
bind = "0.0.0.0"
[database]
url = "postgres://localhost/mydb"
pool_size = 20
[view]
engine = "tera"
templates_dir = "views"
layout = "application"
hot_reload = false
[log]
level = "warn"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.bind, "0.0.0.0");
    assert_eq!(config.database.url, "postgres://localhost/mydb");
    assert_eq!(config.database.pool_size, 20);
    assert_eq!(config.view.engine, "tera");
    assert_eq!(config.view.templates_dir, "views");
    assert_eq!(config.view.layout, "application");
    assert!(!config.view.hot_reload);
    assert_eq!(config.log.level, "warn");
}

#[test]
fn test_config_uses_defaults_for_missing_sections() {
    let config: Config = toml::from_str("").unwrap();
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.server.bind, "127.0.0.1");
    assert_eq!(config.database.pool_size, 5);
    assert_eq!(config.log.level, "info");
    assert!(!config.view.hot_reload);
}
