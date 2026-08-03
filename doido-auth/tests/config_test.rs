//! Auth config YAML parsing tests.

use doido_auth::config::{AuthConfig, YamlConfig};
use doido_auth::AuthError;

#[test]
fn defaults_to_cookie_strategy() {
    let config = AuthConfig::default();
    assert_eq!(config.strategies, vec!["cookie"]);
    assert_eq!(config.routes.prefix, "/users");
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
