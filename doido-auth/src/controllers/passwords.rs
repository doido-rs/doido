//! Default passwords controller (reset request + update stubs).

use doido_auth_macros::auth_controller;
use doido_controller::respond::Format;
use doido_controller::{Context, Response};
use doido_core::Result;
use serde::Deserialize;
use std::marker::PhantomData;

/// Default passwords controller for [`auth_routes!`](crate::auth_routes).
pub struct AuthPasswords<U>(PhantomData<U>);

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PasswordResetForm {
    pub email: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PasswordUpdateForm {
    pub password: String,
    pub password_confirmation: String,
    pub reset_token: String,
}

#[auth_controller]
impl<U> AuthPasswords<U>
where
    U: Send + Sync + 'static,
{
    /// GET `{prefix}/password/new` — request-reset form (HTML mode).
    pub async fn new(ctx: Context) -> Response {
        ctx.render("auth/password_new", serde_json::json!({}))
    }

    /// POST `{prefix}/password` — enqueue a reset email (stub).
    pub async fn create(ctx: Context) -> Result<Response> {
        let json = ctx.negotiated_format() == Format::Json;
        let _form: PasswordResetForm = if json {
            ctx.body_json().await?
        } else {
            ctx.form().await?
        };
        if json {
            Ok(ctx.json(serde_json::json!({ "status": "reset_email_sent" })))
        } else {
            Ok(ctx.render(
                "auth/password_new",
                serde_json::json!({ "notice": "If your email exists, reset instructions were sent." }),
            ))
        }
    }

    /// PATCH `{prefix}/password` — apply a reset token (stub).
    pub async fn update(mut ctx: Context) -> Result<Response> {
        let json = ctx.negotiated_format() == Format::Json;
        let _form: PasswordUpdateForm = if json {
            ctx.body_json().await?
        } else {
            ctx.form().await?
        };
        if json {
            Ok(ctx.status(204))
        } else {
            Ok(ctx.redirect_to("/users/sign_in"))
        }
    }
}
