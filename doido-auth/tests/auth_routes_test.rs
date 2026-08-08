//! Integration tests for `auth_routes!` inside `doido_auth::routes!`.

use doido_auth::testing::{create_test_user, init_test_auth, send, test_auth_config, TestUser};
use doido_model::testing::TestDb;
use http::StatusCode;

#[tokio::test]
async fn auth_routes_default_controllers_sign_in() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "routes@example.com", "secret")
        .await
        .unwrap();

    let app = doido_auth::routes! {
        auth_routes!(TestUser);
    };

    let resp = send(
        app,
        "POST",
        "/users/sign_in",
        r#"{"email":"routes@example.com","password":"secret"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.set_cookie.unwrap().contains("_doido_session"));
}

#[tokio::test]
async fn auth_routes_custom_controller_override() {
    mod auth {
        use doido_auth::testing::TestUser;
        use doido_auth::{sign_in, AuthUser};
        use doido_controller::{controller, Response};
        use doido_core::Result;
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct SignInForm {
            email: String,
            password: String,
        }

        pub struct SessionsController;

        #[controller]
        impl SessionsController {
            pub async fn create(mut ctx: doido_controller::Context) -> Result<Response> {
                let form: SignInForm = ctx.body_json().await?;
                let user = TestUser::find_by_email(ctx.db(), &form.email)
                    .await?
                    .ok_or_else(|| doido_core::anyhow::anyhow!("missing user"))?;
                sign_in(ctx, &user)?;
                Ok(ctx.json(serde_json::json!({ "custom": true })))
            }

            pub async fn destroy(mut ctx: doido_controller::Context) -> Result<Response> {
                doido_auth::sign_out(ctx)?;
                Ok(ctx.status(204))
            }
        }
    }

    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "custom@example.com", "secret")
        .await
        .unwrap();

    let app = doido_auth::routes! {
        auth_routes!(
            TestUser,
            only: [sessions],
            actions: {
                sessions: {
                    create: auth::SessionsController::create,
                    destroy: auth::SessionsController::destroy
                }
            }
        );
    };

    let resp = send(
        app,
        "POST",
        "/users/sign_in",
        r#"{"email":"custom@example.com","password":"secret"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("\"custom\":true"));
}
