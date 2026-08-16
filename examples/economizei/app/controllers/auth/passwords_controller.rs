use crate::services::i18n;
use doido::controller::{controller, Response};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct PasswordResetForm {
    pub email: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct PasswordUpdateForm {
    pub password: String,
    pub password_confirmation: String,
    pub reset_token: String,
}

pub struct PasswordsController;

#[controller]
impl PasswordsController {
    /// GET /users/password/new
    pub async fn new(ctx: doido::controller::Context) -> Response {
        ctx.render(
            "auth/password_new",
            json!({ "title": i18n::t("auth.reset_password") }),
        )
    }

    /// POST /users/password
    pub async fn create(
        mut ctx: doido::controller::Context,
    ) -> doido::Result<Response> {
        let _form: PasswordResetForm = ctx.form().await?;
        Ok(ctx.render(
            "auth/password_new",
            json!({
                "title": i18n::t("auth.reset_password"),
                "notice": i18n::t("auth.reset_sent"),
            }),
        ))
    }

    /// GET /users/password/edit
    pub async fn edit(ctx: doido::controller::Context) -> Response {
        ctx.render(
            "auth/password_edit",
            json!({ "title": i18n::t("auth.change_password") }),
        )
    }

    /// PATCH /users/password
    pub async fn update(
        mut ctx: doido::controller::Context,
    ) -> doido::Result<Response> {
        let _form: PasswordUpdateForm = ctx.form().await?;
        Ok(ctx.redirect_to("/users/sign_in"))
    }
}
