//! Extra `AuthConfig` coverage: OAuth/JWT/two-factor parsing, every module's
//! `as_str`/`from_name`/`route_group`, the route-path helpers, and the
//! `validate`/`strategy_kinds` branches not hit by `config_test`.

use doido_auth::config::{
    AuthConfig, AuthModule, AuthRoutesConfig, OAuthProviderType, StrategyKind,
};
use doido_auth::AuthError;

#[test]
fn every_module_as_str_round_trips_through_from_name() {
    for module in AuthModule::ALL {
        let name = module.as_str();
        assert_eq!(
            AuthModule::from_name(name),
            Some(module),
            "from_name({name}) should recover {module:?}"
        );
    }
    assert_eq!(AuthModule::from_name("nope"), None);
}

#[test]
fn route_group_maps_every_module() {
    let expect = |m: AuthModule| m.route_group();
    assert_eq!(expect(AuthModule::Registerable), Some("registrations"));
    assert_eq!(expect(AuthModule::Recoverable), Some("passwords"));
    assert_eq!(expect(AuthModule::Confirmable), Some("confirmation"));
    assert_eq!(expect(AuthModule::Lockable), Some("unlock"));
    assert_eq!(expect(AuthModule::Omniauthable), Some("oauth"));
    assert_eq!(
        expect(AuthModule::TwoFactorAuthenticatable),
        Some("two_factor")
    );
    // Behavior-only modules mount no routes.
    for m in [
        AuthModule::DatabaseAuthenticatable,
        AuthModule::Rememberable,
        AuthModule::Trackable,
        AuthModule::Timeoutable,
        AuthModule::Validatable,
    ] {
        assert_eq!(m.route_group(), None, "{m:?} should have no route group");
    }
}

#[test]
fn route_config_path_helpers() {
    let routes = AuthRoutesConfig::default();
    assert_eq!(routes.sign_in_path(), "/users/sign_in");
    assert_eq!(routes.sign_out_path(), "/users/sign_out");
    assert_eq!(routes.sign_up_path(), "/users/sign_up");
    assert_eq!(routes.password_path(), "/users/password");
}

#[test]
fn parses_oauth_providers_of_both_types() {
    let yaml = "\
auth:
  oauth:
    google:
      type: oauth2
      client_id: gid
      client_secret: gsecret
      redirect_uri: https://app.test/cb
      scopes: [email, profile]
      authorize_url: https://accounts.google.com/o/oauth2/auth
      token_url: https://oauth2.googleapis.com/token
    twitter:
      type: oauth1
      consumer_key: ck
      consumer_secret: cs
";
    let config = AuthConfig::from_yaml(yaml).unwrap();
    let google = config.oauth.get("google").expect("google provider");
    assert_eq!(google.provider_type, OAuthProviderType::Oauth2);
    assert_eq!(google.client_id.as_deref(), Some("gid"));
    assert_eq!(google.scopes, vec!["email", "profile"]);
    assert_eq!(
        google.token_url.as_deref(),
        Some("https://oauth2.googleapis.com/token")
    );

    let twitter = config.oauth.get("twitter").expect("twitter provider");
    assert_eq!(twitter.provider_type, OAuthProviderType::Oauth1);
    assert_eq!(twitter.consumer_key.as_deref(), Some("ck"));
}

#[test]
fn parses_jwt_full_fields_and_defaults() {
    let full = AuthConfig::from_yaml(
        "auth:\n  jwt:\n    secret: s3cr3t\n    access_ttl: 60\n    refresh_ttl: 120\n    issuer: myiss\n",
    )
    .unwrap();
    let jwt = full.jwt.expect("jwt present");
    assert_eq!(jwt.secret, "s3cr3t");
    assert_eq!(jwt.access_ttl, 60);
    assert_eq!(jwt.refresh_ttl, 120);
    assert_eq!(jwt.issuer.as_deref(), Some("myiss"));
    assert!(jwt.validate().is_ok());

    // Only the secret set → the ttl fields fall back to their defaults.
    let defaulted = AuthConfig::from_yaml("auth:\n  jwt:\n    secret: k\n")
        .unwrap()
        .jwt
        .unwrap();
    assert_eq!(defaulted.access_ttl, 900);
    assert_eq!(defaulted.refresh_ttl, 604_800);
    assert!(defaulted.issuer.is_none());
}

#[test]
fn parses_two_factor_section() {
    let config = AuthConfig::from_yaml(
        "auth:\n  two_factor:\n    enabled: true\n    issuer: MyApp\n",
    )
    .unwrap();
    assert!(config.two_factor.enabled);
    assert_eq!(config.two_factor.issuer.as_deref(), Some("MyApp"));
}

#[test]
fn validate_rejects_unknown_strategy() {
    let config = AuthConfig::from_yaml("auth:\n  strategies:\n    - totally_made_up\n").unwrap();
    match config.validate() {
        Err(AuthError::UnknownStrategy(s)) => assert_eq!(s, "totally_made_up"),
        other => panic!("expected UnknownStrategy, got {other:?}"),
    }
}

#[test]
fn validate_rejects_two_factor_without_feature() {
    // The `auth-2fa` feature is off by default; enabling the module must fail
    // validation with a clear config error.
    let config = AuthConfig::from_yaml(
        "auth:\n  modules:\n    - database_authenticatable\n    - two_factor_authenticatable\n",
    )
    .unwrap();
    assert!(matches!(config.validate(), Err(AuthError::Config(_))));
}

#[test]
fn strategy_kinds_maps_cookie_and_jwt() {
    let config = AuthConfig::from_yaml(
        "auth:\n  strategies:\n    - cookie\n    - jwt\n  jwt:\n    secret: k\n",
    )
    .unwrap();
    assert_eq!(
        config.strategy_kinds(),
        vec![StrategyKind::Cookie, StrategyKind::Jwt]
    );
}
