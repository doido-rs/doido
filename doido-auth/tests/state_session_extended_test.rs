use doido_auth::session::SessionStrategy;
use doido_auth::state::global;
use doido_auth::strategy::AuthStrategy;
use doido_auth::testing::{
    create_test_user, hash_test_password, init_test_auth, jwt_for_user, send, session_for_user,
    test_auth_config, test_jwt_auth_config,
};
use doido_controller::axum::Router;
use doido_model::testing::TestDb;
use http::StatusCode;

#[tokio::test]
async fn jwt_config_boots_cookie_and_jwt_strategies() {
    let db = TestDb::new().await.unwrap();
    let config = test_jwt_auth_config("jwt-secret");
    let _guard = init_test_auth(db.conn().clone(), config).await.unwrap();
    let state = global();
    let names: Vec<_> = state.strategies.iter().map(|s| s.name()).collect();
    assert!(names.contains(&"cookie"));
    assert!(names.contains(&"jwt"));
    assert!(state.jwt.is_some());
}

#[tokio::test]
async fn session_strategy_authenticates_signed_in_user() {
    let db = TestDb::new().await.unwrap();
    let _guard = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "strat@example.com", "secret")
        .await
        .unwrap();
    let session = session_for_user(&user);
    let store = doido_controller::session::EncryptedCookieSessionStore::default();
    let encoded = store.encode(&session);
    let parts = http::Request::builder()
        .header(http::header::COOKIE, format!("_doido_session={encoded}"))
        .uri("/")
        .body(())
        .unwrap()
        .into_parts()
        .0;

    let strategy = SessionStrategy::default_dev();
    let identity = strategy
        .authenticate(&parts, db.conn())
        .await
        .unwrap()
        .expect("identity");
    assert_eq!(identity.user_id, serde_json::json!(user.id));
}

#[tokio::test]
async fn testing_helpers_cover_send_and_jwt() {
    let db = TestDb::new().await.unwrap();
    let config = test_jwt_auth_config("helper-secret");
    let _guard = init_test_auth(db.conn().clone(), config.clone())
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "helper@example.com", "secret")
        .await
        .unwrap();

    let digest = hash_test_password("secret");
    assert!(!digest.is_empty());

    let jwt_cfg = config.jwt.as_ref().unwrap();
    let token = jwt_for_user(jwt_cfg, user.id);
    assert!(!token.is_empty());

    let app = Router::new().route(
        "/ping",
        doido_controller::axum::routing::get(|| async { "pong" }),
    );
    let resp = send(app, "GET", "/ping", "").await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.body, "pong");
}
