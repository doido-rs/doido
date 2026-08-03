//! Custom strategy registry tests.

use async_trait::async_trait;
use doido_auth::identity::AuthIdentity;
use doido_auth::registry::{has_strategy, register_strategy, registered_strategies};
use doido_auth::strategy::AuthStrategy;
use doido_auth::testing::{init_test_auth, test_auth_config};
use doido_core::Result;
use doido_model::sea_orm::DatabaseConnection;
use doido_model::testing::TestDb;
use http::request::Parts;
use std::sync::Arc;

struct FixedIdentityStrategy;

#[async_trait]
impl AuthStrategy for FixedIdentityStrategy {
    fn name(&self) -> &str {
        "fixed"
    }

    async fn authenticate(
        &self,
        _parts: &Parts,
        _db: &DatabaseConnection,
    ) -> Result<Option<AuthIdentity>> {
        Ok(Some(AuthIdentity::new(42_i64)))
    }
}

#[test]
fn register_and_lookup_custom_strategy() {
    register_strategy("fixed", Arc::new(FixedIdentityStrategy));
    assert!(has_strategy("fixed"));
    let names = registered_strategies();
    assert!(names.contains(&"fixed".to_string()));
}

#[tokio::test]
async fn custom_strategy_boots_when_registered() {
    register_strategy("fixed", Arc::new(FixedIdentityStrategy));
    let db = TestDb::new().await.unwrap();
    let mut config = test_auth_config();
    config.strategies = vec!["cookie".into(), "fixed".into()];
    config.validate().unwrap();
    let _auth = init_test_auth(db.conn().clone(), config).await.unwrap();
    assert!(doido_auth::global().strategies.len() >= 2);
}

#[tokio::test]
async fn unknown_strategy_fails_validation() {
    let config = doido_auth::AuthConfig {
        strategies: vec!["not_registered".into()],
        ..Default::default()
    };
    let err = config.validate().unwrap_err();
    assert!(matches!(err, doido_auth::AuthError::UnknownStrategy(_)));
}
