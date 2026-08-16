use crate::controllers::auth_helper::require_user;
use crate::models::user::Entity as UserEntity;
use crate::services::chat::{
    conversation_display_name, create_group_conversation, find_or_create_direct, load_conversation,
    list_messages, mark_conversation_read, message_payload, participant_of, unread_count,
    MessagePayload,
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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CreateConversationForm {
    Direct { recipient_id: i64 },
    Group { name: String, member_ids: Vec<i64> },
}

#[derive(Serialize, Clone)]
pub struct ParticipantInfo {
    pub id: i64,
    pub email: String,
}

#[derive(Serialize)]
pub struct ConversationPayload {
    pub id: i64,
    pub kind: String,
    pub name: Option<String>,
    pub display_name: String,
    pub participant_ids: Vec<i64>,
    pub participants: Vec<ParticipantInfo>,
    pub unread_count: u64,
    pub has_unread: bool,
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

async fn build_payload(
    db: &doido::model::sea_orm::DatabaseConnection,
    conversation_id: i64,
    current_user_id: i64,
) -> doido::Result<Option<ConversationPayload>> {
    let Some(conversation) = load_conversation(db, conversation_id).await? else {
        return Ok(None);
    };

    let participants = load_participants(db, conversation_id).await?;
    let participant_ids: Vec<i64> = participants.iter().map(|p| p.id).collect();
    let unread_count = unread_count(db, conversation_id, current_user_id).await?;
    let display_name = conversation_display_name(
        &conversation,
        &participants
            .iter()
            .map(|p| (p.id, p.email.clone()))
            .collect::<Vec<_>>(),
        current_user_id,
    );

    Ok(Some(ConversationPayload {
        id: conversation.id,
        kind: conversation.kind,
        name: conversation.name,
        display_name,
        participant_ids,
        participants,
        has_unread: unread_count > 0,
        unread_count,
    }))
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
            if let Some(payload) = build_payload(ctx.db(), row.conversation_id, user.id).await? {
                payloads.push(payload);
            }
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

        let Some(payload) = build_payload(ctx.db(), id, user.id).await? else {
            return Ok(ctx.status(404));
        };

        Ok(ctx.json(payload))
    }

    /// POST /conversations — start a direct chat or create a group.
    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let form: CreateConversationForm = ctx.body_json().await?;

        let id = match form {
            CreateConversationForm::Direct { recipient_id } => {
                find_or_create_direct(ctx.db(), user.id, recipient_id).await?
            }
            CreateConversationForm::Group { name, member_ids } => {
                create_group_conversation(ctx.db(), user.id, &name, &member_ids).await?
            }
        };

        let Some(payload) = build_payload(ctx.db(), id, user.id).await? else {
            return Ok(ctx.status(500));
        };

        Ok(ctx.json(payload))
    }

    /// GET /conversations/:id/messages
    pub async fn messages(ctx: Context) -> doido::Result<Response> {
        let user = require_user(ctx).await?;
        let id = parse_id(&ctx);
        if !participant_of(ctx.db(), id, user.id).await? {
            return Ok(ctx.status(403));
        }

        mark_conversation_read(ctx.db(), id, user.id).await?;

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
