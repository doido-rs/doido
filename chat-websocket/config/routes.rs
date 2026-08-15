use crate::channels::conversation_channel::ConversationChannel;
use crate::controllers::{ConversationsController, HelloController, MessagesController};
use crate::models::user::Model as User;
use crate::state;
use doido::cable::{self, Cable};
use doido::controller::axum;
use doido::storage::{self, Storage};
use std::sync::Arc;

fn needs_full_router() -> bool {
    matches!(
        std::env::args().nth(1).as_deref(),
        None | Some("server") | Some("routes")
    )
}

fn minimal_router() -> axum::Router {
    doido::auth::routes! {
        get!("/", HelloController::index);
        auth_routes!(User);
    }
}

async fn full_router() -> axum::Router {
    let _ = doido::model::pool::init().await;

    let pubsub = match cable::pubsub_from_config().await {
        Ok(ps) => ps,
        Err(_) => Arc::new(cable::MemoryPubSub::new()) as Arc<dyn cable::PubSub>,
    };
    let cable_handle = Arc::new(Cable::new(pubsub.clone()));
    state::init_cable(cable_handle);

    let http = doido::auth::routes! {
        get!("/", HelloController::index);
        auth_routes!(User);

        get!("/conversations", ConversationsController::index);
        post!("/conversations", ConversationsController::create);
        get!("/conversations/{id}", ConversationsController::show);
        get!("/conversations/{id}/messages", ConversationsController::messages);
        post!("/messages", MessagesController::create);
    };

    let ws = doido::cable::cable!(pubsub, [ConversationChannel]);

    let storage = Storage::from_config(doido::model::pool::pool().clone())
        .await
        .expect("storage config");
    let storage_routes = storage::serving::routes(storage);

    http.merge(ws).merge(storage_routes)
}

pub fn router() -> axum::Router {
    if !needs_full_router() {
        return minimal_router();
    }

    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(full_router())
    })
}
