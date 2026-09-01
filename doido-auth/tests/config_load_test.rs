//! `AuthConfig` file loading (`load`/`YamlConfig::load_env`). In its own test
//! binary because it mutates the process-global cwd and `DOIDO_ENV`.

use doido_auth::config::load;

#[test]
fn load_reads_env_file_and_defaults_when_missing() {
    let original = std::env::current_dir().unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("config")).unwrap();
    std::fs::write(
        dir.path().join("config/test.yml"),
        "auth:\n  user_model: Account\n  modules:\n    - database_authenticatable\n    - lockable\n",
    )
    .unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    // `load()` reads `config/<DOIDO_ENV>.yml`.
    std::env::set_var("DOIDO_ENV", "test");
    let cfg = load();
    assert_eq!(cfg.user_model.as_deref(), Some("Account"));
    assert!(cfg.has_module(doido_auth::config::AuthModule::Lockable));

    // Missing file → default config (the `unwrap_or_default` path).
    std::env::set_var("DOIDO_ENV", "production");
    let defaulted = load();
    assert_eq!(defaulted.modules.len(), 5);
    assert!(defaulted.user_model.is_none());

    std::env::remove_var("DOIDO_ENV");
    std::env::set_current_dir(original).unwrap();
}
