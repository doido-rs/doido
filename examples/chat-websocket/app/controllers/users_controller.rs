use crate::controllers::auth_helper::require_user;
use crate::models::user::Entity as UserEntity;
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::EntityTrait;
use serde::Serialize;

#[derive(Serialize)]
pub struct UserSummary {
    pub id: i64,
    pub email: String,
}

pub struct UsersController;

#[controller]
impl UsersController {
    /// GET /users — list users (for starting a new conversation).
    pub async fn index(ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let rows = UserEntity::find().all(ctx.db()).await?;
        let users: Vec<UserSummary> = rows
            .into_iter()
            .filter(|u| u.id != user.id)
            .map(|u| UserSummary {
                id: u.id,
                email: u.email,
            })
            .collect();
        Ok(ctx.json(users))
    }
}
