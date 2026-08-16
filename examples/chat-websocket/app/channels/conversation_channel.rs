//! Real-time text messages for a conversation.
use crate::services::chat::{
    create_and_broadcast, participant_of, MessagePayload, TYPE_TEXT,
};
use doido::storage::Storage;
use doido_cable::{channel, streams, Channel, ChannelContext};
use serde_json::Value;

#[channel]
pub struct ConversationChannel;

#[async_trait::async_trait]
impl Channel for ConversationChannel {
    async fn subscribed(&self, ctx: &ChannelContext) -> doido::Result<()> {
        let params = ctx.params();
        let conversation_id = params["conversation_id"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok());
        let user_id = params["user_id"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok());

        let (Some(conversation_id), Some(user_id)) = (conversation_id, user_id) else {
            ctx.transmit(serde_json::json!({ "error": "conversation_id and user_id required" }));
            return Ok(());
        };

        let db = doido::model::pool::pool().clone();
        if !participant_of(&db, conversation_id, user_id).await? {
            ctx.transmit(serde_json::json!({ "error": "forbidden" }));
            return Ok(());
        }

        let stream = streams::stream_for("Conversation", conversation_id);
        ctx.stream_from(&stream).await;
        Ok(())
    }

    async fn unsubscribed(&self, ctx: &ChannelContext) -> doido::Result<()> {
        ctx.stop_all_streams().await;
        Ok(())
    }

    async fn received(&self, ctx: &ChannelContext, data: Value) -> doido::Result<()> {
        if data.get("action").and_then(|v| v.as_str()) != Some("speak") {
            return Ok(());
        }

        let params = ctx.params();
        let conversation_id = params["conversation_id"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok());
        let user_id = params["user_id"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok());
        let body = data.get("body").and_then(|v| v.as_str()).map(str::to_string);

        let (Some(conversation_id), Some(user_id), Some(body)) = (conversation_id, user_id, body)
        else {
            ctx.transmit(serde_json::json!({ "error": "invalid speak payload" }));
            return Ok(());
        };

        if body.trim().is_empty() {
            ctx.transmit(serde_json::json!({ "error": "body cannot be empty" }));
            return Ok(());
        }

        let db = doido::model::pool::pool().clone();
        if !participant_of(&db, conversation_id, user_id).await? {
            ctx.transmit(serde_json::json!({ "error": "forbidden" }));
            return Ok(());
        }

        let storage = Storage::from_config(db.clone()).await?;
        let payload: MessagePayload = create_and_broadcast(
            &db,
            &storage,
            conversation_id,
            user_id,
            TYPE_TEXT,
            Some(body),
            None,
            None,
            None,
            None,
        )
        .await?;

        ctx.transmit(serde_json::json!({
            "action": "message_sent",
            "message": payload,
        }));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doido_cable::ChannelContext;

    #[tokio::test]
    async fn subscribed_rejects_missing_params() {
        let (ctx, mut rx) = ChannelContext::for_test(r#"{"channel":"ConversationChannel"}"#);
        let channel = ConversationChannel;
        channel.subscribed(&ctx).await.unwrap();
        let raw = rx.recv().await.unwrap();
        assert!(raw.contains("error"));
    }
}
