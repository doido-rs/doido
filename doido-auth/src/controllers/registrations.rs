//! Default registrations controller (`sign_up`).

use crate::handlers::{register_user, sign_in};
use crate::user::{AuthUser, RegisterableAuthUser};
use doido_auth_macros::auth_controller;
use doido_controller::respond::Format;
use doido_core::Result;
use doido_model::password::HasSecurePassword;
use serde::Deserialize;
use serde::Serialize;
use std::marker::PhantomData;

/// Default registrations controller for [`auth_routes!`](crate::auth_routes).
pub struct AuthRegistrations<U>(PhantomData<U>);

#[derive(Debug, Deserialize)]
pub struct SignUpForm {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub password_confirmation: Option<String>,
}

#[auth_controller]
impl<U> AuthRegistrations<U>
where
    U: AuthUser + HasSecurePassword + RegisterableAuthUser + Serialize + Send + Sync + 'static,
{
    /// GET `{prefix}/sign_up` — registration form (HTML mode).
    pub async fn new(ctx: doido_controller::Context) -> doido_controller::Response {
        ctx.render("auth/sign_up", serde_json::json!({}))
    }

    /// POST `{prefix}/sign_up` — create an account and sign in.
    pub async fn create(mut ctx: doido_controller::Context) -> Result<doido_controller::Response> {
        let json = ctx.negotiated_format() == Format::Json;
        let form: SignUpForm = if json {
            ctx.body_json().await?
        } else {
            ctx.form().await?
        };

        if let Some(ref confirm) = form.password_confirmation {
            if form.password != *confirm {
                return registration_error(ctx, json, "Password confirmation does not match");
            }
        }

        let db = ctx.db().clone();
        let user =
            match register_user::<U, _, _>(&db, &form.email, &form.password, |email, digest| {
                let db = db.clone();
                async move { U::register(&db, email, digest).await }
            })
            .await
            {
                Ok(user) => user,
                Err(crate::error::AuthError::EmailTaken) => {
                    return registration_error(ctx, json, "Email has already been taken");
                }
                Err(e) => return Err(doido_core::anyhow::anyhow!(e.to_string())),
            };

        sign_in(ctx, &user)?;
        if json {
            Ok(ctx.json(user))
        } else {
            Ok(ctx.redirect_to("/"))
        }
    }
}

fn registration_error(
    ctx: &doido_controller::Context,
    json: bool,
    message: &str,
) -> Result<doido_controller::Response> {
    if json {
        Ok(ctx.status(422))
    } else {
        Ok(ctx.render("auth/sign_up", serde_json::json!({ "error": message })))
    }
}
