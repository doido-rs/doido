//! JWT strategy tests.

use doido_auth::jwt::JwtStrategy;
use doido_auth::strategy::AuthStrategy;
use doido_auth::testing::{init_test_auth, jwt_for_user, test_jwt_auth_config};
use doido_model::testing::TestDb;

#[tokio::test]
async fn issue_and_verify_access_token() {
    let config = test_jwt_auth_config("jwt-test-secret");
    let strategy = JwtStrategy::new(config.jwt.clone().unwrap()).unwrap();
    let tokens = strategy.issue_tokens(&serde_json::json!(42)).unwrap();
    let claims = strategy.verify_token(&tokens.access_token).unwrap();
    assert_eq!(claims.sub, serde_json::json!(42));
    assert_eq!(claims.typ.as_deref(), Some("access"));
}

#[tokio::test]
async fn wrong_secret_fails_verification() {
    let good = JwtStrategy::new(doido_auth::JwtConfig {
        secret: "good".into(),
        access_ttl: 900,
        refresh_ttl: 604_800,
        issuer: None,
    })
    .unwrap();
    let bad = JwtStrategy::new(doido_auth::JwtConfig {
        secret: "bad".into(),
        access_ttl: 900,
        refresh_ttl: 604_800,
        issuer: None,
    })
    .unwrap();
    let token = good
        .issue_tokens(&serde_json::json!(1))
        .unwrap()
        .access_token;
    assert!(bad.verify_token(&token).is_err());
}

#[tokio::test]
async fn jwt_strategy_authenticates_bearer_header() {
    let db = TestDb::new().await.unwrap();
    let config = test_jwt_auth_config("header-secret");
    let _auth = init_test_auth(db.conn().clone(), config.clone())
        .await
        .unwrap();
    let token = jwt_for_user(config.jwt.as_ref().unwrap(), 7);

    let parts = http::Request::builder()
        .header(http::header::AUTHORIZATION, format!("Bearer {token}"))
        .uri("/")
        .body(())
        .unwrap()
        .into_parts()
        .0;

    let strategy = JwtStrategy::new(config.jwt.unwrap()).unwrap();
    let identity = strategy
        .authenticate(&parts, db.conn())
        .await
        .unwrap()
        .expect("identity");
    assert_eq!(identity.user_id, serde_json::json!(7));
}
