//! Default registrations controller (`sign_up`).

use crate::confirmable;
use crate::messages::{json_error, t};
use crate::middleware::{complete_session_sign_in, pipeline_sign_up};
use crate::state::global;
use crate::user::{RegisterableUserModel, UserModel};
use doido_auth_macros::auth_controller;
use doido_core::Result;
use doido_model::password::HasSecurePassword;
use serde::Deserialize;
use serde::Serialize;
use std::marker::PhantomData;

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
    U: UserModel + HasSecurePassword + RegisterableUserModel + Serialize + Send + Sync + 'static,
{
    pub async fn new(ctx: doido_controller::Context) -> doido_controller::Response {
        ctx.render("auth/sign_up", serde_json::json!({}))
    }

    pub async fn create(ctx: &mut doido_controller::Context) -> Result<doido_controller::Response> {
        let json = ctx.wants_json();
        let form: SignUpForm = if json {
            ctx.body_json().await?
        } else {
            ctx.form().await?
        };

        if let Some(ref confirm) = form.password_confirmation {
            if form.password != *confirm {
                return registration_error(ctx, json, &t("auth.password_confirmation_mismatch"));
            }
        }

        let store = global().store.clone();
        let user = match pipeline_sign_up::<U, _, _>(
            store.as_ref(),
            &form.email,
            &form.password,
            |email, digest| {
                let store = store.clone();
                async move { U::register(store.as_ref(), email, digest).await }
            },
        )
        .await
        {
            Ok(user) => user,
            Err(crate::error::AuthError::EmailTaken) => {
                return registration_error(ctx, json, &t("auth.email_taken"));
            }
            Err(crate::error::AuthError::Validation(msg)) => {
                return registration_error(ctx, json, &msg);
            }
            Err(e) => return Err(doido_core::anyhow::anyhow!(e.to_string())),
        };

        if confirmable::is_enabled() {
            if let Some(token) =
                confirmable::generate_confirmation(store.as_ref(), &form.email).await?
            {
                let _ = confirmable::send_confirmation_email(&form.email, &token).await;
            }
            return if json {
                Ok(ctx.json(serde_json::json!({ "status": "confirmation_sent" })))
            } else {
                Ok(ctx.render(
                    "auth/sign_in",
                    serde_json::json!({ "notice": t("auth.confirmation_sent") }),
                ))
            };
        }

        complete_session_sign_in(ctx.session(), &user);
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
        Ok(json_error(422, message))
    } else {
        Ok(ctx.render("auth/sign_up", serde_json::json!({ "error": message })))
    }
}
