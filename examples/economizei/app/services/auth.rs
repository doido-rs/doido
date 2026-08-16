use crate::models::user::{Entity as UserEntity, Model as User};
use doido::controller::Context;
use doido::model::sea_orm::entity::prelude::*;
use doido_auth::{sign_in_with_session, USER_ID_KEY};

pub async fn require_user(ctx: &mut Context) -> doido::Result<User> {
    let user_id = current_user_id(ctx)?;
    UserEntity::find_by_id(user_id)
        .one(ctx.db())
        .await?
        .ok_or_else(|| doido::core::anyhow::anyhow!("user not found"))
}

pub fn current_user_id(ctx: &mut Context) -> doido::Result<i64> {
    let session = ctx.session();
    let value = session
        .data
        .get(USER_ID_KEY)
        .ok_or_else(|| doido::core::anyhow::anyhow!("unauthorized"))?;
    value
        .as_i64()
        .ok_or_else(|| doido::core::anyhow::anyhow!("invalid session user id"))
}

#[allow(dead_code)]
pub fn sign_in_user(ctx: &mut Context, user: &User) {
    sign_in_with_session(ctx.session(), user);
}

pub fn optional_user_id(ctx: &mut Context) -> Option<i64> {
    current_user_id(ctx).ok()
}
