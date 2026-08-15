use crate::controllers::auth_helper::require_user;
use crate::services::chat::participant_of;
use doido::controller::{controller, Context, Response};
use serde_json::json;

pub struct ChatController;

#[controller]
impl ChatController {
    /// GET /login
    pub async fn login(mut ctx: Context) -> doido::Result<Response> {
        if ctx.session().get::<i64>("user_id").is_some() {
            return Ok(ctx.redirect_to("/chat"));
        }
        Ok(ctx.render("chat/login", json!({ "title": "Entrar" })))
    }

    /// GET /chat — conversation list.
    pub async fn index(ctx: Context) -> doido::Result<Response> {
        let user = match require_user(ctx).await {
            Ok(u) => u,
            Err(_) => return Ok(ctx.redirect_to("/login")),
        };
        Ok(ctx.render(
            "chat/index",
            json!({
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
            Err(_) => return Ok(ctx.redirect_to("/login")),
        };
        let id = parse_id(&ctx);
        if id == 0 || !participant_of(ctx.db(), id, user.id).await? {
            return Ok(ctx.redirect_to("/chat"));
        }
        Ok(ctx.render(
            "chat/show",
            json!({
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
