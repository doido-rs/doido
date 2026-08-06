//! Default two-factor controller (feature `auth-2fa`).

use doido_auth_macros::auth_controller;
use doido_controller::{Context, Response};
use doido_core::Result;
use std::marker::PhantomData;

/// Default two-factor controller for [`auth_routes!`](crate::auth_routes).
pub struct AuthTwoFactor<U>(PhantomData<U>);

#[auth_controller]
impl<U> AuthTwoFactor<U>
where
    U: Send + Sync + 'static,
{
    /// GET `{prefix}/two_factor/new` — enrollment form.
    pub async fn new(ctx: Context) -> Response {
        ctx.render("auth/two_factor", serde_json::json!({}))
    }

    /// POST `{prefix}/two_factor` — enable 2FA (stub).
    pub async fn create(ctx: Context) -> Result<Response> {
        Ok(ctx.status(501))
    }

    /// POST `{prefix}/two_factor/challenge` — verify TOTP after sign-in (stub).
    pub async fn challenge(ctx: Context) -> Result<Response> {
        Ok(ctx.status(501))
    }
}
