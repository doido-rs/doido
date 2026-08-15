use crate::controllers::auth_helper::require_user;
use crate::services::chat::{
    create_and_broadcast, participant_of, MessagePayload, TYPE_FILE, TYPE_IMAGE,
};
use doido::controller::{controller, Response};
use doido::storage::Storage;
use serde::Deserialize;

pub struct MessagesController;

#[derive(Deserialize)]
pub struct CreateMessageForm {
    pub conversation_id: i64,
    pub message_type: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub attachment_signed_id: Option<String>,
}

#[controller]
impl MessagesController {
    /// POST /messages — create image or file messages (upload via storage API first).
    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let form: CreateMessageForm = ctx.body_json().await?;

        if form.message_type != TYPE_IMAGE && form.message_type != TYPE_FILE {
            return Ok(ctx.json(serde_json::json!({
                "error": "only image and file messages are accepted via HTTP; send text over the WebSocket"
            })));
        }

        if form.attachment_signed_id.is_none() {
            return Ok(ctx.json(serde_json::json!({
                "error": "attachment_signed_id is required for image and file messages"
            })));
        }

        if !participant_of(ctx.db(), form.conversation_id, user.id).await? {
            return Ok(ctx.status(403));
        }

        let storage = Storage::from_config(ctx.db().clone()).await?;
        let payload: MessagePayload = create_and_broadcast(
            ctx.db(),
            &storage,
            form.conversation_id,
            user.id,
            &form.message_type,
            form.body,
            form.attachment_signed_id,
        )
        .await?;

        Ok(ctx.json(payload))
    }
}
