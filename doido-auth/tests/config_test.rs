//! Auth config YAML parsing tests.

use doido_auth::config::{AuthConfig, YamlConfig};
use doido_auth::{AuthError, AuthModule};

#[test]
fn defaults_to_cookie_strategy() {
    let config = AuthConfig::default();
    assert_eq!(config.strategies, vec!["cookie"]);
    assert_eq!(config.routes.prefix, "/users");
}

#[test]
fn defaults_to_devise_default_modules() {
    let config = AuthConfig::default();
    assert!(config.has_module(AuthModule::DatabaseAuthenticatable));
    assert!(config.has_module(AuthModule::Registerable));
    assert!(config.has_module(AuthModule::Recoverable));
    assert!(config.has_module(AuthModule::Rememberable));
    assert!(config.has_module(AuthModule::Validatable));
    // Not in the default set.
    assert!(!config.has_module(AuthModule::Confirmable));
    assert!(!config.has_module(AuthModule::Lockable));
    assert!(!config.has_module(AuthModule::Omniauthable));
}

#[test]
fn parses_modules_list() {
    let yaml = r#"
auth:
  modules:
    - database_authenticatable
    - registerable
    - confirmable
    - lockable
    - omniauthable
"#;
    let config = AuthConfig::from_yaml(yaml).unwrap();
    assert!(config.has_module(AuthModule::Confirmable));
    assert!(config.has_module(AuthModule::Lockable));
    assert!(config.has_module(AuthModule::Omniauthable));
    assert!(!config.has_module(AuthModule::Recoverable));
}

#[test]
fn enabled_route_groups_reflect_modules() {
    let yaml = r#"
auth:
  modules: [database_authenticatable, registerable, confirmable, lockable, omniauthable]
"#;
    let config = AuthConfig::from_yaml(yaml).unwrap();
    let groups = config.enabled_route_groups();
    assert_eq!(groups[0], "sessions"); // always present
    assert!(groups.contains(&"registrations"));
    assert!(groups.contains(&"confirmation"));
    assert!(groups.contains(&"unlock"));
    assert!(groups.contains(&"oauth"));
    assert!(!groups.contains(&"passwords")); // recoverable not enabled
}

#[test]
fn validate_requires_database_authenticatable() {
    let yaml = "auth:\n  modules: [registerable]\n";
    let config = AuthConfig::from_yaml(yaml).unwrap();
    let err = config.validate().unwrap_err();
    assert!(matches!(err, AuthError::Config(_)));
}

#[test]
fn parses_full_auth_section() {
    let yaml = r#"
auth:
  user_model: User
  strategies: [cookie, jwt]
  jwt:
    secret: test-secret
    access_ttl: 60
    refresh_ttl: 3600
    issuer: myapp
  routes:
    prefix: /accounts
    sign_in: login
"#;
    let config = AuthConfig::from_yaml(yaml).unwrap();
    assert_eq!(config.user_model.as_deref(), Some("User"));
    assert_eq!(config.strategies, vec!["cookie", "jwt"]);
    assert_eq!(config.jwt.as_ref().unwrap().secret, "test-secret");
    assert_eq!(config.routes.prefix, "/accounts");
    assert_eq!(config.routes.sign_in, "login");
}

#[test]
fn jwt_strategy_requires_secret() {
    let config = AuthConfig {
        strategies: vec!["jwt".into()],
        jwt: Some(doido_auth::JwtConfig {
            secret: String::new(),
            access_ttl: 900,
            refresh_ttl: 604_800,
            issuer: None,
        }),
        ..Default::default()
    };
    let err = config.validate().unwrap_err();
    assert!(matches!(err, AuthError::Config(_)));
}

#[test]
fn ignores_unrelated_yaml_sections() {
    let yaml = "server:\n  port: 3000\n";
    let parsed = YamlConfig::from_yaml(yaml).unwrap();
    assert_eq!(parsed.auth.strategies, vec!["cookie"]);
}

#[test]
fn strategy_kinds_filters_builtin_strategies() {
    let config = AuthConfig {
        strategies: vec!["cookie".into(), "jwt".into(), "ldap".into()],
        ..Default::default()
    };
    let kinds = config.strategy_kinds();
    assert_eq!(kinds.len(), 2);
}

#[test]
fn route_paths_trim_trailing_slash_on_prefix() {
    let config = AuthConfig::from_yaml(
        r#"
auth:
  routes:
    prefix: /accounts/
    sign_in: login
"#,
    )
    .unwrap();
    assert_eq!(config.routes.sign_in_path(), "/accounts/login");
}

#[test]
fn jwt_strategy_requires_jwt_section() {
    let config = AuthConfig {
        strategies: vec!["jwt".into()],
        jwt: None,
        ..Default::default()
    };
    let err = config.validate().unwrap_err();
    assert!(matches!(err, AuthError::Config(_)));
}
