//! Default sessions controller (`sign_in` / `sign_out`).

use crate::handlers::{authenticate, sign_in, sign_out};
use crate::user::AuthUser;
use doido_auth_macros::auth_controller;
use doido_controller::respond::Format;
use doido_core::Result;
use doido_model::password::HasSecurePassword;
use serde::Deserialize;
use serde::Serialize;
use std::marker::PhantomData;

/// Default sessions controller for [`auth_routes!`](crate::auth_routes).
pub struct AuthSessions<U>(PhantomData<U>);

#[derive(Debug, Deserialize)]
pub struct SignInForm {
    pub email: String,
    pub password: String,
}

#[auth_controller]
impl<U> AuthSessions<U>
where
    U: AuthUser + HasSecurePassword + Serialize + Send + Sync + 'static,
{
    /// GET `{prefix}/sign_in` — sign-in form (HTML mode).
    pub async fn new(ctx: doido_controller::Context) -> doido_controller::Response {
        ctx.render("auth/sign_in", serde_json::json!({}))
    }

    /// POST `{prefix}/sign_in` — authenticate and establish a session.
    pub async fn create(mut ctx: doido_controller::Context) -> Result<doido_controller::Response> {
        let json = ctx.negotiated_format() == Format::Json;
        let form: SignInForm = if json {
            ctx.body_json().await?
        } else {
            ctx.form().await?
        };

        match authenticate::<U>(ctx.db(), &form.email, &form.password).await {
            Ok(user) => {
                // `trackable` module (best-effort — never blocks sign-in).
                let _ = crate::trackable::record_sign_in(ctx.db(), &form.email, None).await;
                sign_in(ctx, &user)?;
                if json {
                    Ok(ctx.json(user))
                } else {
                    Ok(ctx.redirect_to("/"))
                }
            }
            Err(_) if json => Ok(ctx.status(401)),
            Err(_) => Ok(ctx.render(
                "auth/sign_in",
                serde_json::json!({ "error": "Invalid email or password" }),
            )),
        }
    }

    /// DELETE `{prefix}/sign_out` — clear the session.
    pub async fn destroy(mut ctx: doido_controller::Context) -> Result<doido_controller::Response> {
        sign_out(ctx)?;
        if ctx.negotiated_format() == Format::Json {
            Ok(ctx.status(204))
        } else {
            Ok(ctx.redirect_to("/"))
        }
    }
}
