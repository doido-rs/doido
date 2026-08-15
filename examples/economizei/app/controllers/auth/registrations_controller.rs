use crate::models::user::{ActiveModel, Column, Entity};
use crate::services::{i18n, tenant};
use doido::controller::{controller, Response};
use doido::model::password::hash_password;
use doido::model::sea_orm::{entity::prelude::*, Set};
use doido_auth::sign_in;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct SignUpForm {
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

pub struct RegistrationsController;

#[controller]
impl RegistrationsController {
    /// GET /users/sign_up
    pub async fn new(ctx: doido::controller::Context) -> Response {
        ctx.render("auth/sign_up", json!({ "title": i18n::t("nav.sign_up") }))
    }

    /// POST /users/sign_up
    pub async fn create(
        mut ctx: doido::controller::Context,
    ) -> doido::Result<Response> {
        let form: SignUpForm = ctx.form().await?;
        if form.password != form.password_confirmation {
            return Ok(ctx.render(
                "auth/sign_up",
                json!({
                    "title": i18n::t("nav.sign_up"),
                    "error": i18n::t("auth.password_mismatch"),
                }),
            ));
        }
        if Entity::find()
            .filter(Column::Email.eq(&form.email))
            .one(ctx.db())
            .await?
            .is_some()
        {
            return Ok(ctx.render(
                "auth/sign_up",
                json!({
                    "title": i18n::t("nav.sign_up"),
                    "error": i18n::t("auth.email_taken"),
                }),
            ));
        }
        let now = chrono::Utc::now().naive_utc();
        let digest = hash_password(&form.password)?;
        let record = ActiveModel {
            email: Set(form.email),
            password_digest: Set(digest),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let user = record.insert(ctx.db()).await?;
        sign_in(ctx, &user)?;
        tenant::set_default_company(ctx, user.id).await?;
        Ok(ctx.redirect_to("/"))
    }
}
