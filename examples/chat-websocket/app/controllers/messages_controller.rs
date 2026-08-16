use crate::controllers::auth_helper::require_user;
use crate::models::message::Entity as MessageEntity;
use crate::services::chat::{
    create_and_broadcast, decode_image_data, participant_of, MessagePayload, TYPE_FILE, TYPE_IMAGE,
};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::EntityTrait;
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
    #[serde(default)]
    pub image_data: Option<String>,
    #[serde(default)]
    pub image_content_type: Option<String>,
    #[serde(default)]
    pub image_filename: Option<String>,
}

#[controller]
impl MessagesController {
    /// POST /messages — create image or file messages.
    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let form: CreateMessageForm = ctx.body_json().await?;

        if form.message_type != TYPE_IMAGE && form.message_type != TYPE_FILE {
            return Ok(ctx.json(serde_json::json!({
                "error": "only image and file messages are accepted via HTTP; send text over the WebSocket"
            })));
        }

        if !participant_of(ctx.db(), form.conversation_id, user.id).await? {
            return Ok(ctx.status(403));
        }

        let (image_data, image_content_type, image_filename, attachment_signed_id) =
            if form.message_type == TYPE_IMAGE {
                let Some(encoded) = form.image_data else {
                    return Ok(ctx.json(serde_json::json!({
                        "error": "image_data is required for image messages"
                    })));
                };
                let bytes = decode_image_data(&encoded)?;
                (
                    Some(bytes),
                    form.image_content_type,
                    form.image_filename,
                    None,
                )
            } else {
                if form.attachment_signed_id.is_none() {
                    return Ok(ctx.json(serde_json::json!({
                        "error": "attachment_signed_id is required for file messages"
                    })));
                }
                (None, None, None, form.attachment_signed_id)
            };

        let storage = Storage::from_config(ctx.db().clone()).await?;
        let payload: MessagePayload = create_and_broadcast(
            ctx.db(),
            &storage,
            form.conversation_id,
            user.id,
            &form.message_type,
            form.body,
            attachment_signed_id,
            image_data,
            image_content_type,
            image_filename,
        )
        .await?;

        Ok(ctx.json(payload))
    }

    /// GET /messages/:id/attachment — serve an image stored in the database.
    pub async fn attachment(ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let id = parse_id(&ctx);
        let Some(message) = MessageEntity::find_by_id(id).one(ctx.db()).await? else {
            return Ok(ctx.status(404));
        };

        if !participant_of(ctx.db(), message.conversation_id, user.id).await? {
            return Ok(ctx.status(403));
        }

        if message.message_type != TYPE_IMAGE {
            return Ok(ctx.status(404));
        }

        let Some(data) = message.image_data else {
            return Ok(ctx.status(404));
        };

        let content_type = message
            .image_content_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        let filename = message.image_filename.as_deref();

        Ok(ctx.send_data(data, content_type, filename))
    }
}

fn parse_id(ctx: &Context) -> i64 {
    ctx.param("id").and_then(|v| v.parse().ok()).unwrap_or_default()
}
