//! Auth state boot tests.

use doido_auth::init;
use doido_auth::testing::{init_test_auth, test_auth_config};
use doido_auth::try_global;
use doido_model::testing::TestDb;

#[tokio::test]
async fn init_is_idempotent() {
    let db = TestDb::new().await.unwrap();
    let config = test_auth_config();
    let _auth = init_test_auth(db.conn().clone(), config.clone())
        .await
        .unwrap();
    init(db.conn().clone(), &config).await.unwrap();
    assert!(try_global().is_some());
}

#[tokio::test]
async fn init_fails_on_invalid_config() {
    let config = doido_auth::AuthConfig {
        strategies: vec!["jwt".into()],
        jwt: None,
        ..Default::default()
    };
    assert!(config.validate().is_err());
}
