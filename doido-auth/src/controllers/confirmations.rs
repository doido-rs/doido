//! Default confirmations controller (`confirmable` — confirm + resend).

use crate::confirmable;
use doido_auth_macros::auth_controller;
use doido_core::Result;
use serde::Deserialize;
use std::marker::PhantomData;

/// Default confirmations controller for [`auth_routes!`](crate::auth_routes).
pub struct AuthConfirmations<U>(PhantomData<U>);

#[derive(Debug, Deserialize)]
pub struct ConfirmQuery {
    #[serde(default)]
    pub confirmation_token: String,
}

#[derive(Debug, Deserialize)]
pub struct ResendForm {
    pub email: String,
}

#[auth_controller]
impl<U> AuthConfirmations<U>
where
    U: Send + Sync + 'static,
{
    /// GET `{prefix}/confirmation?confirmation_token=…` — confirm an account.
    pub async fn show(ctx: doido_controller::Context) -> Result<doido_controller::Response> {
        let json = ctx.wants_json();
        let token = ctx
            .params::<ConfirmQuery>()
            .map(|q| q.confirmation_token)
            .unwrap_or_default();

        let confirmed = confirmable::confirm(ctx.db(), &token).await?;
        if !confirmed {
            return if json {
                Ok(ctx.status(422))
            } else {
                Ok(ctx.render(
                    "auth/sign_in",
                    serde_json::json!({ "error": "Confirmation link is invalid or has expired" }),
                ))
            };
        }
        if json {
            Ok(ctx.json(serde_json::json!({ "status": "confirmed" })))
        } else {
            Ok(ctx.redirect_to("/users/sign_in"))
        }
    }

    /// POST `{prefix}/confirmation` — resend confirmation instructions. Responds
    /// generically to avoid leaking which emails exist.
    pub async fn create(mut ctx: doido_controller::Context) -> Result<doido_controller::Response> {
        let json = ctx.wants_json();
        let form: ResendForm = if json {
            ctx.body_json().await?
        } else {
            ctx.form().await?
        };

        if let Some(token) = confirmable::generate_confirmation(ctx.db(), &form.email).await? {
            let _ = confirmable::send_confirmation_email(&form.email, &token).await;
        }

        if json {
            Ok(ctx.json(serde_json::json!({ "status": "confirmation_sent" })))
        } else {
            Ok(ctx.render(
                "auth/sign_in",
                serde_json::json!({ "notice": "If your email exists, a confirmation link was sent." }),
            ))
        }
    }
}
