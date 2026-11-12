//! Sign-in uses a single store fetch before password verification.

use doido_auth::handlers::authenticate;
use doido_auth::state::global;
use doido_auth::testing::{
    create_test_user, ensure_users_table, init_test_auth, test_auth_config, TestUser,
};
use doido_model::testing::TestDb;

#[tokio::test]
async fn sign_in_performs_one_fetch() {
    let db = TestDb::new().await.unwrap();
    ensure_users_table(db.conn()).await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "onefetch@example.com", "secret")
        .await
        .unwrap();

    let before = global().store.fetch_count();
    let _user: TestUser = authenticate("onefetch@example.com", "secret")
        .await
        .unwrap();
    let after = global().store.fetch_count();
    assert_eq!(after - before, 1, "expected exactly one UserStore fetch");
}
