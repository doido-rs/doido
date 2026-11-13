//! Redis `UserStore` sign-up and sign-in (requires local Redis).

#![cfg(feature = "auth-redis")]

use doido_auth::authenticate;
use doido_auth::config::{AuthConfig, UserStoreConfig, UserStoreType};
use doido_auth::state::global;
use doido_auth::testing::{init_test_auth, TestUser};
use doido_auth::user_model::store::RedisUserStore;
use doido_model::testing::TestDb;

#[tokio::test]
#[ignore = "requires redis on 127.0.0.1:6379"]
async fn redis_store_sign_in_after_insert() {
    let db = TestDb::new().await.unwrap();
    let config = AuthConfig {
        user_store: UserStoreConfig {
            store_type: UserStoreType::Redis,
            ..Default::default()
        },
        ..doido_auth::testing::test_auth_config()
    };
    let _auth = init_test_auth(db.conn().clone(), config).await.unwrap();
    let store = global().store.clone();
    let redis = store.redis().expect("redis store");
    redis
        .insert_user(1, "redis@example.com", "digest-placeholder")
        .await
        .unwrap();
    // Password check uses bcrypt — insert a known hash for a test password.
    let digest = doido_auth::testing::hash_test_password("secret");
    redis
        .insert_user(2, "redis2@example.com", &digest)
        .await
        .unwrap();
    let user: TestUser = authenticate("redis2@example.com", "secret")
        .await
        .expect("sign in");
    assert_eq!(user.email, "redis2@example.com");
}
