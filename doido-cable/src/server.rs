//! The live WebSocket cable server: an axum `ws` upgrade handler that speaks the
//! ActionCable wire protocol, a [`ChannelRegistry`] routing subscriptions to
//! [`Channel`] handlers, a heartbeat ping loop, and a pub/sub bridge so
//! broadcasts reach subscribed clients.

use crate::channel::{Channel, ChannelContext, ChannelName};
use crate::protocol::{CableFrame, ServerFrame};
use crate::pubsub::PubSub;
use doido_controller::axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use doido_controller::axum::extract::State;
use doido_controller::axum::response::Response;
use doido_controller::axum::routing::get;
use doido_controller::axum::Router;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Default heartbeat when none is configured (matches ActionCable's 3s).
const DEFAULT_HEARTBEAT: Duration = Duration::from_secs(3);

/// Maps ActionCable channel names to their handlers, over a shared pub/sub
/// backend (Rails' channel routing).
pub struct ChannelRegistry {
    channels: HashMap<String, Arc<dyn Channel>>,
    pubsub: Arc<dyn PubSub>,
    heartbeat: Duration,
}

impl ChannelRegistry {
    pub fn new(pubsub: Arc<dyn PubSub>) -> Self {
        Self {
            channels: HashMap::new(),
            pubsub,
            heartbeat: DEFAULT_HEARTBEAT,
        }
    }

    /// Set the heartbeat ping interval (from `cable.ping_interval`).
    pub fn with_heartbeat(mut self, interval: Duration) -> Self {
        self.heartbeat = interval;
        self
    }

    /// Register a channel handler under an explicit name.
    pub fn register(&mut self, name: impl Into<String>, channel: Arc<dyn Channel>) -> &mut Self {
        self.channels.insert(name.into(), channel);
        self
    }

    /// Register a channel by its `#[channel]`-derived name.
    pub fn register_channel<C>(&mut self, channel: C) -> &mut Self
    where
        C: Channel + ChannelName + 'static,
    {
        self.register(C::channel_name(), Arc::new(channel))
    }

    /// Look up a channel handler by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.channels.get(name).cloned()
    }

    /// The pub/sub backend the registry shares with its connections.
    pub fn pubsub(&self) -> Arc<dyn PubSub> {
        self.pubsub.clone()
    }
}

/// Extract the `channel` name from an ActionCable subscription identifier (a
/// JSON string like `{"channel":"ChatChannel","room":"1"}`).
fn channel_name_of(identifier: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(identifier)
        .ok()?
        .get("channel")?
        .as_str()
        .map(str::to_string)
}

/// Send a server frame to a connection's outbound sink (best-effort).
fn send_frame(tx: &tokio::sync::mpsc::UnboundedSender<String>, frame: &ServerFrame) {
    if let Ok(json) = frame.to_json() {
        let _ = tx.send(json);
    }
}

/// Handle one client frame against a connection's subscription state. Factored
/// out of [`handle_socket`] so it can be unit-tested without a live socket.
async fn handle_frame(
    frame: CableFrame,
    registry: &ChannelRegistry,
    tx: &tokio::sync::mpsc::UnboundedSender<String>,
    subs: &mut HashMap<String, ChannelContext>,
) {
    match frame {
        CableFrame::Subscribe { identifier } => {
            let channel = channel_name_of(&identifier).and_then(|name| registry.get(&name));
            let Some(channel) = channel else {
                send_frame(tx, &ServerFrame::RejectSubscription { identifier });
                return;
            };
            let ctx = ChannelContext::new(identifier.clone(), tx.clone(), registry.pubsub());
            match channel.subscribed(&ctx).await {
                Ok(()) => {
                    send_frame(
                        tx,
                        &ServerFrame::ConfirmSubscription {
                            identifier: identifier.clone(),
                        },
                    );
                    subs.insert(identifier, ctx);
                }
                Err(_) => send_frame(tx, &ServerFrame::RejectSubscription { identifier }),
            }
        }
        CableFrame::Message { identifier, data } => {
            if let Some(ctx) = subs.get(&identifier) {
                if let Some(channel) = channel_name_of(&identifier).and_then(|n| registry.get(&n)) {
                    let _ = channel.received(ctx, data).await;
                }
            }
        }
        CableFrame::Unsubscribe { identifier } => {
            if let Some(ctx) = subs.remove(&identifier) {
                if let Some(channel) = channel_name_of(&identifier).and_then(|n| registry.get(&n)) {
                    let _ = channel.unsubscribed(&ctx).await;
                }
                ctx.stop_all_streams().await;
            }
        }
    }
}

/// Drive a single upgraded WebSocket connection: greet with `welcome`, ping on a
/// heartbeat, and dispatch inbound ActionCable frames until the socket closes.
pub async fn handle_socket(socket: WebSocket, registry: Arc<ChannelRegistry>) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Writer task: fan every queued outbound string into the socket.
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(Message::text(msg)).await.is_err() {
                break;
            }
        }
    });

    let _ = tx.send(ServerFrame::Welcome.to_json().unwrap_or_default());

    // Heartbeat task: ActionCable `ping` frames on the configured interval.
    let heartbeat_tx = tx.clone();
    let heartbeat_interval = registry.heartbeat;
    let heartbeat = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(heartbeat_interval);
        ticker.tick().await; // consume the immediate first tick
        loop {
            ticker.tick().await;
            let ping = crate::heartbeat::ping_now().to_json().unwrap_or_default();
            if heartbeat_tx.send(ping).is_err() {
                break;
            }
        }
    });

    let mut subs: HashMap<String, ChannelContext> = HashMap::new();
    while let Some(Ok(message)) = stream.next().await {
        match message {
            Message::Text(text) => {
                if let Ok(frame) = CableFrame::parse(text.as_str()) {
                    handle_frame(frame, &registry, &tx, &mut subs).await;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    heartbeat.abort();
    writer.abort();
    for (_, ctx) in subs {
        ctx.stop_all_streams().await;
    }
}

/// axum handler that upgrades a request to a cable WebSocket connection.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(registry): State<Arc<ChannelRegistry>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, registry))
}

/// A `Router` mounting the cable endpoint at `/cable`, backed by `registry`.
pub fn route(registry: Arc<ChannelRegistry>) -> Router {
    Router::new()
        .route("/cable", get(ws_handler))
        .with_state(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pubsub::MemoryPubSub;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::mpsc;

    struct EchoChannel {
        subscribed: Arc<AtomicBool>,
        received: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl Channel for EchoChannel {
        async fn subscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> {
            self.subscribed.store(true, Ordering::SeqCst);
            Ok(())
        }
        async fn unsubscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> {
            Ok(())
        }
        async fn received(
            &self,
            ctx: &ChannelContext,
            data: serde_json::Value,
        ) -> doido_core::Result<()> {
            self.received.store(true, Ordering::SeqCst);
            ctx.transmit(data);
            Ok(())
        }
    }

    impl ChannelName for EchoChannel {
        fn channel_name() -> &'static str {
            "EchoChannel"
        }
    }

    fn registry_with(subscribed: Arc<AtomicBool>, received: Arc<AtomicBool>) -> ChannelRegistry {
        let mut registry = ChannelRegistry::new(Arc::new(MemoryPubSub::new()));
        registry.register_channel(EchoChannel {
            subscribed,
            received,
        });
        registry
    }

    #[test]
    fn channel_name_of_reads_the_channel_field() {
        assert_eq!(
            channel_name_of(r#"{"channel":"EchoChannel","room":"1"}"#).as_deref(),
            Some("EchoChannel")
        );
        assert_eq!(channel_name_of("not json"), None);
    }

    #[tokio::test]
    async fn subscribe_confirms_and_calls_subscribed() {
        let subscribed = Arc::new(AtomicBool::new(false));
        let registry = registry_with(subscribed.clone(), Arc::new(AtomicBool::new(false)));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut subs = HashMap::new();

        let identifier = r#"{"channel":"EchoChannel"}"#.to_string();
        handle_frame(
            CableFrame::Subscribe {
                identifier: identifier.clone(),
            },
            &registry,
            &tx,
            &mut subs,
        )
        .await;

        assert!(subscribed.load(Ordering::SeqCst), "subscribed() ran");
        assert!(subs.contains_key(&identifier), "subscription tracked");
        let frame = ServerFrame::parse(&rx.recv().await.unwrap()).unwrap();
        assert_eq!(frame, ServerFrame::ConfirmSubscription { identifier });
    }

    #[tokio::test]
    async fn subscribe_to_unknown_channel_is_rejected() {
        let registry = registry_with(
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        );
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut subs = HashMap::new();

        let identifier = r#"{"channel":"NopeChannel"}"#.to_string();
        handle_frame(
            CableFrame::Subscribe {
                identifier: identifier.clone(),
            },
            &registry,
            &tx,
            &mut subs,
        )
        .await;

        assert!(subs.is_empty());
        let frame = ServerFrame::parse(&rx.recv().await.unwrap()).unwrap();
        assert_eq!(frame, ServerFrame::RejectSubscription { identifier });
    }

    #[tokio::test]
    async fn message_dispatches_to_received() {
        let received = Arc::new(AtomicBool::new(false));
        let registry = registry_with(Arc::new(AtomicBool::new(false)), received.clone());
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut subs = HashMap::new();
        let identifier = r#"{"channel":"EchoChannel"}"#.to_string();

        handle_frame(
            CableFrame::Subscribe {
                identifier: identifier.clone(),
            },
            &registry,
            &tx,
            &mut subs,
        )
        .await;
        let _confirm = rx.recv().await.unwrap();

        handle_frame(
            CableFrame::Message {
                identifier: identifier.clone(),
                data: serde_json::json!({ "text": "hi" }),
            },
            &registry,
            &tx,
            &mut subs,
        )
        .await;

        assert!(received.load(Ordering::SeqCst), "received() ran");
        // EchoChannel transmits the data back.
        let echoed = crate::protocol::ServerMessage::parse(&rx.recv().await.unwrap()).unwrap();
        assert_eq!(echoed.message["text"], "hi");
    }
}
