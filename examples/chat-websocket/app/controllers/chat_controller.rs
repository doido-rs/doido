use crate::controllers::auth_helper::require_user;
use crate::services::chat::participant_of;
use doido::controller::{controller, Context, Response};

pub struct ChatController;

#[controller]
impl ChatController {
    /// GET /login — alias for the framework sign-in page.
    pub async fn login(ctx: Context) -> Response {
        ctx.redirect_to("/users/sign_in")
    }

    /// GET /chat — conversation list.
    pub async fn index(ctx: Context) -> doido::Result<Response> {
        let user = match require_user(ctx).await {
            Ok(u) => u,
            Err(_) => return Ok(ctx.redirect_to("/users/sign_in")),
        };
        Ok(ctx.render(
            "chat/index",
            serde_json::json!({
                "title": "Conversas",
                "user_id": user.id,
                "user_email": user.email,
            }),
        ))
    }

    /// GET /chat/{id} — single conversation.
    pub async fn show(ctx: Context) -> doido::Result<Response> {
        let user = match require_user(ctx).await {
            Ok(u) => u,
            Err(_) => return Ok(ctx.redirect_to("/users/sign_in")),
        };
        let id = parse_id(&ctx);
        if id == 0 || !participant_of(ctx.db(), id, user.id).await? {
            return Ok(ctx.redirect_to("/chat"));
        }
        Ok(ctx.render(
            "chat/show",
            serde_json::json!({
                "title": "Conversa",
                "conversation_id": id,
                "user_id": user.id,
                "user_email": user.email,
            }),
        ))
    }
}

fn parse_id(ctx: &Context) -> i64 {
    ctx.param("id").and_then(|v| v.parse().ok()).unwrap_or_default()
}
