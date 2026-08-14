//! Default passwords controller (`recoverable` reset request + update).

use crate::recoverable;
use doido_auth_macros::auth_controller;
use doido_core::Result;
use serde::Deserialize;
use std::marker::PhantomData;

/// Default passwords controller for [`auth_routes!`](crate::auth_routes).
pub struct AuthPasswords<U>(PhantomData<U>);

#[derive(Debug, Deserialize)]
pub struct PasswordResetForm {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordUpdateForm {
    pub password: String,
    #[serde(default)]
    pub password_confirmation: Option<String>,
    pub reset_password_token: String,
}

#[derive(Debug, Deserialize)]
pub struct EditQuery {
    #[serde(default)]
    pub reset_password_token: String,
}

#[auth_controller]
impl<U> AuthPasswords<U>
where
    U: Send + Sync + 'static,
{
    /// GET `{prefix}/password/new` — request-reset form (HTML mode).
    pub async fn new(ctx: doido_controller::Context) -> doido_controller::Response {
        ctx.render("auth/password_new", serde_json::json!({}))
    }

    /// POST `{prefix}/password` — generate a token and email reset instructions.
    /// Always responds generically to avoid leaking which emails exist.
    pub async fn create(mut ctx: doido_controller::Context) -> Result<doido_controller::Response> {
        let json = ctx.wants_json();
        let form: PasswordResetForm = if json {
            ctx.body_json().await?
        } else {
            ctx.form().await?
        };

        if let Some(token) = recoverable::request_reset(ctx.db(), &form.email).await? {
            let _ = recoverable::send_reset_email(&form.email, &token).await;
        }

        if json {
            Ok(ctx.json(serde_json::json!({ "status": "reset_email_sent" })))
        } else {
            Ok(ctx.render(
                "auth/password_new",
                serde_json::json!({ "notice": "If your email exists, reset instructions were sent." }),
            ))
        }
    }

    /// GET `{prefix}/password/edit?reset_password_token=…` — choose-new-password
    /// form (HTML mode).
    pub async fn edit(ctx: doido_controller::Context) -> doido_controller::Response {
        let token = ctx
            .params::<EditQuery>()
            .map(|q| q.reset_password_token)
            .unwrap_or_default();
        ctx.render(
            "auth/password_edit",
            serde_json::json!({ "reset_password_token": token }),
        )
    }

    /// PATCH `{prefix}/password` — set a new password using a valid reset token.
    pub async fn update(mut ctx: doido_controller::Context) -> Result<doido_controller::Response> {
        let json = ctx.wants_json();
        let form: PasswordUpdateForm = if json {
            ctx.body_json().await?
        } else {
            ctx.form().await?
        };

        if let Some(ref confirm) = form.password_confirmation {
            if &form.password != confirm {
                return password_error(ctx, json, "Password confirmation does not match");
            }
        }

        let reset =
            recoverable::reset_password(ctx.db(), &form.reset_password_token, &form.password)
                .await?;
        if !reset {
            return password_error(ctx, json, "Reset link is invalid or has expired");
        }

        if json {
            Ok(ctx.json(serde_json::json!({ "status": "password_reset" })))
        } else {
            Ok(ctx.redirect_to("/users/sign_in"))
        }
    }
}

fn password_error(
    ctx: &mut doido_controller::Context,
    json: bool,
    message: &str,
) -> Result<doido_controller::Response> {
    if json {
        Ok(ctx.status(422))
    } else {
        Ok(ctx.render(
            "auth/password_edit",
            serde_json::json!({ "error": message }),
        ))
    }
}
