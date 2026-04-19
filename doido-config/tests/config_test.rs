use doido_config::Config;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

fn write(dir: &TempDir, rel: &str, content: &str) {
    let path = dir.path().join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const BASE_TOML: &str = r#"
[server]
port = 3000
bind = "127.0.0.1"
[database]
url = "sqlite://dev.db"
pool_size = 5
[view]
engine = "tera"
templates_dir = "views"
layout = "application"
hot_reload = true
[log]
level = "info"
"#;

#[test]
#[serial]
fn test_load_base_config() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE_TOML);
    let config = Config::load_from_env(dir.path(), "noenv").unwrap();
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.server.bind, "127.0.0.1");
    assert_eq!(config.database.pool_size, 5);
    assert_eq!(config.log.level, "info");
    assert!(config.view.hot_reload);
}

#[test]
#[serial]
fn test_env_file_overrides_base() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE_TOML);
    write(
        &dir,
        "config/doido.staging.toml",
        "[server]\nbind = \"0.0.0.0\"\n[log]\nlevel = \"warn\"",
    );
    let config = Config::load_from_env(dir.path(), "staging").unwrap();
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.server.bind, "0.0.0.0");
    assert_eq!(config.log.level, "warn");
}

#[test]
#[serial]
fn test_credentials_override_base_and_env_file() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE_TOML);
    write(&dir, "config/doido.test.toml", "[database]\npool_size = 1");

    let key = [42u8; 32];
    let cred_toml = "[database]\nurl = \"postgres://secret@prod/db\"\n";
    let encrypted = doido_config::crypto::encrypt_credentials(cred_toml, &key).unwrap();
    write(&dir, "config/credentials.toml.enc", &encrypted);
    write(&dir, "config/master.key", &"2a".repeat(32)); // 0x2a = 42 decimal

    let config = Config::load_from_env(dir.path(), "test").unwrap();
    assert_eq!(config.database.url, "postgres://secret@prod/db");
    assert_eq!(config.database.pool_size, 1);
    assert_eq!(config.server.port, 3000);
}

#[test]
#[serial]
fn test_env_var_takes_highest_priority() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE_TOML);
    std::env::set_var("SERVER__PORT", "9999");
    std::env::set_var("LOG__LEVEL", "trace");
    let config = Config::load_from_env(dir.path(), "noenv").unwrap();
    std::env::remove_var("SERVER__PORT");
    std::env::remove_var("LOG__LEVEL");
    assert_eq!(config.server.port, 9999);
    assert_eq!(config.log.level, "trace");
}

#[test]
#[serial]
fn test_missing_base_config_returns_error() {
    let dir = TempDir::new().unwrap();
    let result = Config::load_from_env(dir.path(), "dev");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("config/doido.toml not found"));
}

#[test]
#[serial]
fn test_missing_master_key_with_credentials_file_returns_error() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE_TOML);
    write(
        &dir,
        "config/credentials.toml.enc",
        "not-valid-encrypted-content",
    );
    if std::env::var("DOIDO_MASTER_KEY").is_err() {
        let result = Config::load_from_env(dir.path(), "noenv");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("master key") || msg.contains("master.key"),
            "got: {msg}"
        );
    }
}

#[test]
#[serial]
fn test_load_from_uses_doido_env() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE_TOML);
    write(
        &dir,
        "config/doido.test.toml",
        "[log]\nlevel = \"test-level\"",
    );
    let config = Config::load_from_env(dir.path(), "test").unwrap();
    assert_eq!(config.log.level, "test-level");
}
