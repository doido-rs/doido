use crate::models::user::{Entity as UserEntity, Model as User};
use doido::controller::Context;
use doido::model::sea_orm::EntityTrait;

/// Resolve the signed-in user from the session cookie.
pub async fn require_user(ctx: &mut Context) -> doido::Result<User> {
    let user_id = ctx
        .session()
        .get::<i64>("user_id")
        .ok_or_else(|| doido::core::anyhow::anyhow!("unauthorized"))?;
    UserEntity::find_by_id(user_id)
        .one(ctx.db())
        .await?
        .ok_or_else(|| doido::core::anyhow::anyhow!("unauthorized").into())
}
