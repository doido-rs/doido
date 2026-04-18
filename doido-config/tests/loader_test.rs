use doido_config::crypto::encrypt_credentials;
use doido_config::loader::{deep_merge, load_layers, load_toml};
use std::fs;
use tempfile::TempDir;

fn write(dir: &TempDir, rel: &str, content: &str) {
    let path = dir.path().join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const BASE: &str = r#"
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
fn test_load_toml_reads_file() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE);
    let val = load_toml(&dir.path().join("config/doido.toml")).unwrap();
    assert!(val.is_some());
    assert_eq!(val.unwrap()["server"]["port"].as_integer(), Some(3000));
}

#[test]
fn test_load_toml_returns_none_for_missing_file() {
    let dir = TempDir::new().unwrap();
    let val = load_toml(&dir.path().join("config/doido.toml")).unwrap();
    assert!(val.is_none());
}

#[test]
fn test_deep_merge_overrides_leaf_values() {
    let base: toml::Value = toml::from_str("[server]\nport = 3000\nbind = \"127.0.0.1\"").unwrap();
    let over: toml::Value = toml::from_str("[server]\nbind = \"0.0.0.0\"").unwrap();
    let merged = deep_merge(base, over);
    assert_eq!(merged["server"]["port"].as_integer(), Some(3000));
    assert_eq!(merged["server"]["bind"].as_str(), Some("0.0.0.0"));
}

#[test]
fn test_deep_merge_adds_new_keys() {
    let base: toml::Value = toml::from_str("[server]\nport = 3000").unwrap();
    let over: toml::Value = toml::from_str("[database]\nurl = \"postgres://\"").unwrap();
    let merged = deep_merge(base, over);
    assert_eq!(merged["server"]["port"].as_integer(), Some(3000));
    assert_eq!(merged["database"]["url"].as_str(), Some("postgres://"));
}

#[test]
fn test_load_layers_applies_env_override_file() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE);
    write(&dir, "config/doido.prod.toml",
        "[server]\nbind = \"0.0.0.0\"\n[log]\nlevel = \"warn\"");
    let val = load_layers(dir.path(), "prod").unwrap();
    assert_eq!(val["server"]["port"].as_integer(), Some(3000));
    assert_eq!(val["server"]["bind"].as_str(), Some("0.0.0.0"));
    assert_eq!(val["log"]["level"].as_str(), Some("warn"));
}

#[test]
fn test_load_layers_skips_missing_env_file() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE);
    let val = load_layers(dir.path(), "noenv").unwrap();
    assert_eq!(val["server"]["port"].as_integer(), Some(3000));
}

#[test]
fn test_load_layers_errors_on_missing_base() {
    let dir = TempDir::new().unwrap();
    let result = load_layers(dir.path(), "dev");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("config/doido.toml not found"), "got: {msg}");
}

#[test]
fn test_load_layers_merges_credentials() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE);

    let key = [0u8; 32];
    let cred_toml = "[database]\nurl = \"postgres://secret@prod/db\"\n";
    let encrypted = encrypt_credentials(cred_toml, &key).unwrap();
    write(&dir, "config/credentials.toml.enc", &encrypted);

    let hex_key = "00".repeat(32);
    write(&dir, "config/master.key", &hex_key);

    let val = load_layers(dir.path(), "noenv").unwrap();
    assert_eq!(val["database"]["url"].as_str(), Some("postgres://secret@prod/db"));
    assert_eq!(val["server"]["port"].as_integer(), Some(3000));
}

#[test]
fn test_load_layers_skips_credentials_when_file_absent() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE);
    let val = load_layers(dir.path(), "noenv").unwrap();
    assert_eq!(val["server"]["port"].as_integer(), Some(3000));
}

#[test]
fn test_load_layers_errors_when_credentials_exist_but_key_missing() {
    let dir = TempDir::new().unwrap();
    write(&dir, "config/doido.toml", BASE);
    write(&dir, "config/credentials.toml.enc", "fake-encrypted-content");
    if std::env::var("DOIDO_MASTER_KEY").is_err() {
        let result = load_layers(dir.path(), "noenv");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("master key") || msg.contains("master.key"),
            "got: {msg}"
        );
    }
}
