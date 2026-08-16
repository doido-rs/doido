//! Chat domain helpers: conversations, messages, cable broadcasts, attachments.
use crate::models::conversation::ActiveModel as ConversationActive;
use crate::models::conversation_participant::{
    ActiveModel as ParticipantActive, Column as ParticipantColumn, Entity as ParticipantEntity,
};
use crate::models::message::{ActiveModel as MessageActive, Entity as MessageEntity, Model as Message};
use crate::state;
use base64::Engine;
use chrono::Utc;
use doido::model::sea_orm::{
    entity::prelude::*, ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use doido::storage::Storage;
use serde::Serialize;
use serde_json::json;

pub const TYPE_TEXT: &str = "text";
pub const TYPE_IMAGE: &str = "image";
pub const TYPE_FILE: &str = "file";

const ATTACHMENT_NAME: &str = "attachment";
const RECORD_TYPE: &str = "Message";

/// JSON shape returned by the REST API and pushed over the cable.
#[derive(Debug, Serialize)]
pub struct MessagePayload {
    pub id: i64,
    pub conversation_id: i64,
    pub user_id: i64,
    pub message_type: String,
    pub body: Option<String>,
    pub attachment: Option<AttachmentPayload>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct AttachmentPayload {
    pub filename: String,
    pub content_type: Option<String>,
    pub byte_size: i64,
    pub url: String,
}

/// Ensure `user_id` belongs to `conversation_id`.
pub async fn participant_of(
    db: &DatabaseConnection,
    conversation_id: i64,
    user_id: i64,
) -> doido::Result<bool> {
    let found = ParticipantEntity::find()
        .filter(ParticipantColumn::ConversationId.eq(conversation_id))
        .filter(ParticipantColumn::UserId.eq(user_id))
        .one(db)
        .await?;
    Ok(found.is_some())
}

/// Count messages from other participants that the user has not read yet.
pub async fn unread_count(
    db: &DatabaseConnection,
    conversation_id: i64,
    user_id: i64,
) -> doido::Result<u64> {
    let participant = ParticipantEntity::find()
        .filter(ParticipantColumn::ConversationId.eq(conversation_id))
        .filter(ParticipantColumn::UserId.eq(user_id))
        .one(db)
        .await?;

    let Some(participant) = participant else {
        return Ok(0);
    };

    let mut query = MessageEntity::find()
        .filter(crate::models::message::Column::ConversationId.eq(conversation_id))
        .filter(crate::models::message::Column::UserId.ne(user_id));

    if let Some(last_read_at) = participant.last_read_at {
        query = query.filter(crate::models::message::Column::CreatedAt.gt(last_read_at));
    }

    query.count(db).await.map_err(Into::into)
}

/// Mark all messages in a conversation as read for the given participant.
pub async fn mark_conversation_read(
    db: &DatabaseConnection,
    conversation_id: i64,
    user_id: i64,
) -> doido::Result<()> {
    let participant = ParticipantEntity::find()
        .filter(ParticipantColumn::ConversationId.eq(conversation_id))
        .filter(ParticipantColumn::UserId.eq(user_id))
        .one(db)
        .await?;

    let Some(participant) = participant else {
        return Ok(());
    };

    let mut active: ParticipantActive = participant.into();
    active.last_read_at = Set(Some(Utc::now()));
    active.update(db).await?;
    Ok(())
}

/// Find an existing 1:1 conversation between two users, if any.
pub async fn find_direct_conversation(
    db: &DatabaseConnection,
    user_a: i64,
    user_b: i64,
) -> doido::Result<Option<i64>> {
    let rows = ParticipantEntity::find()
        .filter(ParticipantColumn::UserId.is_in([user_a, user_b]))
        .all(db)
        .await?;

    use std::collections::HashMap;
    let mut by_conversation: HashMap<i64, Vec<i64>> = HashMap::new();
    for row in rows {
        by_conversation
            .entry(row.conversation_id)
            .or_default()
            .push(row.user_id);
    }

    for (conversation_id, users) in by_conversation {
        if users.len() == 2 && users.contains(&user_a) && users.contains(&user_b) {
            return Ok(Some(conversation_id));
        }
    }
    Ok(None)
}

/// Create a new conversation with two participants.
pub async fn create_direct_conversation(
    db: &DatabaseConnection,
    user_a: i64,
    user_b: i64,
) -> doido::Result<i64> {
    let conversation = ConversationActive {
        ..Default::default()
    }
    .insert(db)
    .await?;

    for user_id in [user_a, user_b] {
        crate::models::conversation_participant::ActiveModel {
            conversation_id: Set(conversation.id),
            user_id: Set(user_id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(conversation.id)
}

/// Find or create a direct conversation between the current user and `recipient_id`.
pub async fn find_or_create_direct(
    db: &DatabaseConnection,
    current_user_id: i64,
    recipient_id: i64,
) -> doido::Result<i64> {
    if current_user_id == recipient_id {
        return Err(doido::core::anyhow::anyhow!("cannot chat with yourself").into());
    }

    if let Some(id) = find_direct_conversation(db, current_user_id, recipient_id).await? {
        return Ok(id);
    }

    create_direct_conversation(db, current_user_id, recipient_id).await
}

pub async fn list_messages(
    db: &DatabaseConnection,
    conversation_id: i64,
) -> doido::Result<Vec<Message>> {
    MessageEntity::find()
        .filter(crate::models::message::Column::ConversationId.eq(conversation_id))
        .order_by_asc(crate::models::message::Column::Id)
        .all(db)
        .await
        .map_err(Into::into)
}

pub async fn insert_message(
    db: &DatabaseConnection,
    conversation_id: i64,
    user_id: i64,
    message_type: &str,
    body: Option<String>,
    image_data: Option<Vec<u8>>,
    image_content_type: Option<String>,
    image_filename: Option<String>,
) -> doido::Result<Message> {
    let now = Utc::now();
    let record = MessageActive {
        conversation_id: Set(conversation_id),
        user_id: Set(user_id),
        message_type: Set(message_type.to_string()),
        body: Set(body),
        image_data: Set(image_data),
        image_content_type: Set(image_content_type),
        image_filename: Set(image_filename),
        created_at: Set(now),
        ..Default::default()
    };
    record.insert(db).await.map_err(Into::into)
}

pub fn decode_image_data(encoded: &str) -> doido::Result<Vec<u8>> {
    let payload = encoded
        .split_once(',')
        .map(|(_, data)| data)
        .unwrap_or(encoded);
    base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|e| doido::core::anyhow::anyhow!("invalid base64 image data: {e}").into())
}

pub async fn attach_blob(
    storage: &Storage,
    message_id: i64,
    signed_id: &str,
) -> doido::Result<()> {
    let key = storage.verify_signed_id(signed_id)?;
    storage
        .attach(RECORD_TYPE, &message_id.to_string(), ATTACHMENT_NAME, &key)
        .await
}

fn db_image_attachment(message: &Message) -> Option<AttachmentPayload> {
    let data = message.image_data.as_ref()?;
    Some(AttachmentPayload {
        filename: message
            .image_filename
            .clone()
            .unwrap_or_else(|| "image".into()),
        content_type: message.image_content_type.clone(),
        byte_size: data.len() as i64,
        url: format!("/messages/{}/attachment", message.id),
    })
}

pub async fn message_payload(
    storage: &Storage,
    message: &Message,
) -> doido::Result<MessagePayload> {
    let attachment = if message.message_type == TYPE_IMAGE {
        db_image_attachment(message).or_else(|| {
            // Legacy messages may still use doido-storage attachments.
            None
        })
    } else {
        None
    };

    let attachment = if attachment.is_some() {
        attachment
    } else if message.message_type == TYPE_IMAGE || message.message_type == TYPE_FILE {
        if let Some(blob) = storage
            .one(RECORD_TYPE, &message.id.to_string(), ATTACHMENT_NAME)
            .await?
        {
            Some(AttachmentPayload {
                filename: blob.filename.clone(),
                content_type: blob.content_type.clone(),
                byte_size: blob.byte_size,
                url: storage.redirect_path(&blob),
            })
        } else {
            None
        }
    } else {
        None
    };

    Ok(MessagePayload {
        id: message.id,
        conversation_id: message.conversation_id,
        user_id: message.user_id,
        message_type: message.message_type.clone(),
        body: message.body.clone(),
        attachment,
        created_at: message.created_at.to_rfc3339(),
    })
}

pub async fn broadcast_message(
    conversation_id: i64,
    payload: &MessagePayload,
) -> doido::Result<()> {
    let stream = doido::cable::streams::stream_for("Conversation", conversation_id);
    let identifier = format!(
        r#"{{"channel":"ConversationChannel","conversation_id":"{conversation_id}"}}"#
    );
    state::cable()
        .broadcast(
            &stream,
            &identifier,
            json!({
                "action": "new_message",
                "message": payload,
            }),
        )
        .await?;
    Ok(())
}

pub async fn create_and_broadcast(
    db: &DatabaseConnection,
    storage: &Storage,
    conversation_id: i64,
    user_id: i64,
    message_type: &str,
    body: Option<String>,
    attachment_signed_id: Option<String>,
    image_data: Option<Vec<u8>>,
    image_content_type: Option<String>,
    image_filename: Option<String>,
) -> doido::Result<MessagePayload> {
    let message = insert_message(
        db,
        conversation_id,
        user_id,
        message_type,
        body,
        image_data,
        image_content_type,
        image_filename,
    )
    .await?;

    if let Some(signed_id) = attachment_signed_id {
        attach_blob(storage, message.id, &signed_id).await?;
    }

    let payload = message_payload(storage, &message).await?;
    broadcast_message(conversation_id, &payload).await?;
    Ok(payload)
}
