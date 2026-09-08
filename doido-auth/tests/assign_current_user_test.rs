//! `assign_current_user` stages the authenticated user (and a `signed_in` flag)
//! onto the controller context so views can render `current_user`.

use doido_auth::assign_current_user;
use doido_auth::identity::AuthIdentity;
use doido_auth::testing::{create_test_user, init_test_auth, test_auth_config, TestUser};
use doido_controller::Context;
use doido_model::testing::TestDb;

fn ctx_with_identity(identity: Option<AuthIdentity>) -> Context {
    let mut parts = http::Request::builder()
        .uri("/")
        .body(())
        .unwrap()
        .into_parts()
        .0;
    if let Some(id) = identity {
        parts.extensions.insert(id);
    }
    Context::from_request_parts(parts)
}

#[tokio::test]
async fn assigns_current_user_when_authenticated() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "view@example.com", "secret")
        .await
        .unwrap();

    let mut ctx = ctx_with_identity(Some(AuthIdentity::new(user.id)));
    assign_current_user::<TestUser>(&mut ctx).await;

    assert_eq!(
        ctx.assigns().get("signed_in"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        ctx.assigns()
            .get("current_user")
            .and_then(|u| u.get("email")),
        Some(&serde_json::json!("view@example.com"))
    );
}

#[tokio::test]
async fn marks_anonymous_when_no_identity() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();

    let mut ctx = ctx_with_identity(None);
    assign_current_user::<TestUser>(&mut ctx).await;

    assert_eq!(
        ctx.assigns().get("signed_in"),
        Some(&serde_json::json!(false))
    );
    assert!(ctx.assigns().get("current_user").is_none());
}
