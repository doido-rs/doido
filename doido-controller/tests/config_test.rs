use doido_controller::{Config, Environment, ServerConfig, YamlConfig};

#[test]
fn server_config_defaults_to_0_0_0_0_3000() {
    let s = ServerConfig::default();
    assert_eq!(s.bind, "0.0.0.0");
    assert_eq!(s.port, 3000);
}

#[test]
fn yaml_config_parses_server_bind_and_port() {
    let cfg = YamlConfig::from_yaml("server:\n  bind: 127.0.0.1\n  port: 8080\n").unwrap();
    assert_eq!(cfg.server().bind, "127.0.0.1");
    assert_eq!(cfg.server().port, 8080);
}

#[test]
fn yaml_config_falls_back_to_default_server_when_absent() {
    let cfg = YamlConfig::from_yaml("{}").unwrap();
    assert_eq!(cfg.server().bind, "0.0.0.0");
    assert_eq!(cfg.server().port, 3000);
}

#[test]
fn environment_names_map_to_config_files() {
    assert_eq!(Environment::Development.as_str(), "development");
    assert_eq!(Environment::Test.as_str(), "test");
    assert_eq!(Environment::Production.as_str(), "production");
}

#[test]
fn get_env_reads_doido_env() {
    // Safe: this is the only test that touches DOIDO_ENV and tests run in one process.
    std::env::set_var("DOIDO_ENV", "production");
    assert_eq!(Environment::get_env(), Environment::Production);
    std::env::set_var("DOIDO_ENV", "test");
    assert_eq!(Environment::get_env(), Environment::Test);
    std::env::remove_var("DOIDO_ENV");
    assert_eq!(Environment::get_env(), Environment::Development);
}
