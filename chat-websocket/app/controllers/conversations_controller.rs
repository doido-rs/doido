use crate::controllers::auth_helper::require_user;
use crate::services::chat::{
    find_or_create_direct, list_messages, message_payload, participant_of, MessagePayload,
};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use doido::storage::Storage;
use serde::Deserialize;
use serde::Serialize;

use crate::models::conversation_participant::{
    Column as ParticipantColumn, Entity as ParticipantEntity,
};

#[derive(Deserialize)]
pub struct CreateConversationForm {
    pub recipient_id: i64,
}

#[derive(Serialize)]
pub struct ConversationPayload {
    pub id: i64,
    pub participant_ids: Vec<i64>,
}

pub struct ConversationsController;

#[controller]
impl ConversationsController {
    /// GET /conversations — list conversations for the signed-in user.
    pub async fn index(ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let rows = ParticipantEntity::find()
            .filter(ParticipantColumn::UserId.eq(user.id))
            .all(ctx.db())
            .await?;

        let mut payloads = Vec::new();
        for row in rows {
            let participants = ParticipantEntity::find()
                .filter(ParticipantColumn::ConversationId.eq(row.conversation_id))
                .all(ctx.db())
                .await?;
            let participant_ids: Vec<i64> = participants.into_iter().map(|p| p.user_id).collect();
            payloads.push(ConversationPayload {
                id: row.conversation_id,
                participant_ids,
            });
        }

        Ok(ctx.json(payloads))
    }

    /// GET /conversations/:id
    pub async fn show(ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let id = parse_id(&ctx);
        if !participant_of(ctx.db(), id, user.id).await? {
            return Ok(ctx.status(403));
        }

        let participants = ParticipantEntity::find()
            .filter(ParticipantColumn::ConversationId.eq(id))
            .all(ctx.db())
            .await?;
        let participant_ids: Vec<i64> = participants.into_iter().map(|p| p.user_id).collect();

        Ok(ctx.json(ConversationPayload {
            id,
            participant_ids,
        }))
    }

    /// POST /conversations — start (or reopen) a direct chat with `recipient_id`.
    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let form: CreateConversationForm = ctx.body_json().await?;
        let id = find_or_create_direct(ctx.db(), user.id, form.recipient_id).await?;
        let participants = ParticipantEntity::find()
            .filter(ParticipantColumn::ConversationId.eq(id))
            .all(ctx.db())
            .await?;
        let participant_ids: Vec<i64> = participants.into_iter().map(|p| p.user_id).collect();

        Ok(ctx.json(ConversationPayload {
            id,
            participant_ids,
        }))
    }

    /// GET /conversations/:id/messages
    pub async fn messages(ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let id = parse_id(&ctx);
        if !participant_of(ctx.db(), id, user.id).await? {
            return Ok(ctx.status(403));
        }

        let storage = Storage::from_config(ctx.db().clone()).await?;
        let records = list_messages(ctx.db(), id).await?;
        let mut payloads: Vec<MessagePayload> = Vec::new();
        for record in records {
            payloads.push(message_payload(&storage, &record).await?);
        }
        Ok(ctx.json(payloads))
    }
}

fn parse_id(ctx: &Context) -> i64 {
    ctx.param("id").and_then(|v| v.parse().ok()).unwrap_or_default()
}
