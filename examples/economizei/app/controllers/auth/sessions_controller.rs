use crate::models::user::Model as User;
use crate::services::{i18n, tenant};
use doido::controller::{controller, Response};
use doido::model::password::HasSecurePassword;
use doido_auth::{sign_in, sign_out};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct SignInForm {
    pub email: String,
    pub password: String,
}

pub struct SessionsController;

#[controller]
impl SessionsController {
    /// GET /users/sign_in
    pub async fn new(ctx: doido::controller::Context) -> Response {
        ctx.render(
            "auth/sign_in",
            json!({
                "title": i18n::t("nav.sign_in"),
                "email_label": i18n::t("auth.email"),
                "password_label": i18n::t("auth.password"),
                "submit_label": i18n::t("nav.sign_in"),
            }),
        )
    }

    /// POST /users/sign_in
    pub async fn create(
        mut ctx: doido::controller::Context,
    ) -> doido::Result<Response> {
        let form: SignInForm = ctx.form().await?;
        if let Some(user) = User::find_by_email(ctx.db(), &form.email).await? {
            if user.authenticate(&form.password) {
                sign_in(ctx, &user)?;
                tenant::set_default_company(ctx, user.id).await?;
                return Ok(ctx.redirect_to("/"));
            }
        }
        Ok(ctx.render(
            "auth/sign_in",
            json!({
                "title": i18n::t("nav.sign_in"),
                "error": i18n::t("auth.invalid_credentials"),
                "email_label": i18n::t("auth.email"),
                "password_label": i18n::t("auth.password"),
                "submit_label": i18n::t("nav.sign_in"),
            }),
        ))
    }

    /// DELETE /users/sign_out — also accepts POST from HTML forms.
    pub async fn destroy(
        mut ctx: doido::controller::Context,
    ) -> doido::Result<Response> {
        sign_out(ctx)?;
        Ok(ctx.redirect_to("/users/sign_in"))
    }
}
