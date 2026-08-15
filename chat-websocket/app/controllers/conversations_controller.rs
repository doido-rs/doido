use crate::controllers::auth_helper::require_user;
use crate::models::user::Entity as UserEntity;
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

#[derive(Serialize, Clone)]
pub struct ParticipantInfo {
    pub id: i64,
    pub email: String,
}

#[derive(Serialize)]
pub struct ConversationPayload {
    pub id: i64,
    pub participant_ids: Vec<i64>,
    pub participants: Vec<ParticipantInfo>,
}

pub struct ConversationsController;

async fn load_participants(
    db: &doido::model::sea_orm::DatabaseConnection,
    conversation_id: i64,
) -> doido::Result<Vec<ParticipantInfo>> {
    let rows = ParticipantEntity::find()
        .filter(ParticipantColumn::ConversationId.eq(conversation_id))
        .all(db)
        .await?;
    let user_ids: Vec<i64> = rows.iter().map(|r| r.user_id).collect();
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    let users = UserEntity::find()
        .filter(crate::models::user::Column::Id.is_in(user_ids))
        .all(db)
        .await?;
    Ok(users
        .into_iter()
        .map(|u| ParticipantInfo {
            id: u.id,
            email: u.email,
        })
        .collect())
}

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
            let participants = load_participants(ctx.db(), row.conversation_id).await?;
            let participant_ids: Vec<i64> = participants.iter().map(|p| p.id).collect();
            payloads.push(ConversationPayload {
                id: row.conversation_id,
                participant_ids,
                participants,
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

        let participants = load_participants(ctx.db(), id).await?;
        let participant_ids: Vec<i64> = participants.iter().map(|p| p.id).collect();

        Ok(ctx.json(ConversationPayload {
            id,
            participant_ids,
            participants,
        }))
    }

    /// POST /conversations — start (or reopen) a direct chat with `recipient_id`.
    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let form: CreateConversationForm = ctx.body_json().await?;
        let id = find_or_create_direct(ctx.db(), user.id, form.recipient_id).await?;
        let participants = load_participants(ctx.db(), id).await?;
        let participant_ids: Vec<i64> = participants.iter().map(|p| p.id).collect();

        Ok(ctx.json(ConversationPayload {
            id,
            participant_ids,
            participants,
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
