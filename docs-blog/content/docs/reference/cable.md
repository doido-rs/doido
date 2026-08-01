+++
title = "Cable (WebSockets)"
description = "Real-time channels with the #[channel] macro, stream broadcasting, pluggable pub/sub, and connection auth."
weight = 11
aliases = ['/docs/guides/cable/']

+++

> **Design spec:** [`docs/12-cable.md`](https://github.com/doido-rs/doido/blob/master/docs/12-cable.md).
> This guide documents the API as implemented in `doido-cable`.

**Rails analogue: Action Cable.** Channels give you real-time, bidirectional WebSocket
communication speaking the ActionCable wire protocol (compatible with `@rails/actioncable`).
Connections subscribe to named streams; broadcasting to a stream reaches every subscriber.
Pub/Sub is pluggable (in-memory, Redis, or database).

## At a glance

```rust
use doido::cable::{channel, Channel, ChannelContext, Cable, MemoryPubSub, CableConnection};
```

## Defining a channel

`#[channel]` derives the channel's registration name. Implement the `Channel` trait's
lifecycle hooks: `subscribed` (on subscribe), `unsubscribed` (on disconnect), and `received`
(an inbound message). Route on the message payload inside `received`.

```rust
use doido::cable::{channel, Channel, ChannelContext};
use doido::Result;
use serde_json::Value;

#[channel]
struct ChatChannel;

#[async_trait::async_trait]
impl Channel for ChatChannel {
    async fn subscribed(&self, _ctx: &ChannelContext) -> Result<()> { Ok(()) }
    async fn unsubscribed(&self, _ctx: &ChannelContext) -> Result<()> { Ok(()) }

    async fn received(&self, ctx: &ChannelContext, data: Value) -> Result<()> {
        // Dispatch on the payload (e.g. an "action" field, ActionCable-style):
        if data["action"] == "speak" {
            // …broadcast the message to the room…
        }
        Ok(())
    }
}
```

`ChannelContext` carries the subscription `identifier` and the current `stream`.

## Broadcasting to streams

`Cable` publishes to a named stream from anywhere in the app: `broadcast` sends a structured
ActionCable message (identifier + JSON), `broadcast_to` sends a raw string. Use
`streams::stream_from` to build consistent stream names.

```rust
use doido::cable::{Cable, MemoryPubSub, streams};
use std::sync::Arc;
use serde_json::json;

let cable = Cable::new(Arc::new(MemoryPubSub::new()));
let stream = streams::stream_from("chat:room:1");

cable.broadcast(&stream, "ChatChannel", json!({ "text": "Hi" })).await?;
cable.broadcast_to(&stream, "raw string").await?;
```

## Subscribing to a stream

Subscribe through the underlying pub/sub to receive a broadcast channel of messages (used
internally to relay to the WebSocket).

```rust
let mut rx = cable.pubsub().subscribe("chat:room:1").await?;
let message = rx.recv().await?; // a JSON ServerMessage string
```

## Pub/Sub backends

`PubSub` is the pluggable trait (`subscribe`, `publish`). `MemoryPubSub` (in-process) is the
default; `RedisPubSub` (feature `cable-redis`) fans out across processes; `DbPubSub` (feature
`cable-db`) polls the database.

```rust
use doido::cable::{Cable, MemoryPubSub};
let cable = Cable::new(std::sync::Arc::new(MemoryPubSub::new()));
```

## Connection identity & authorization

`CableConnection` models one connection's identity and whether it's authorized — the Rails
`identified_by` / `reject_unauthorized_connection` analogue. Resolve identity when the socket
connects and reject unauthorized ones before any channel runs.

```rust
use doido::cable::CableConnection;

let mut conn = CableConnection::new();
conn.identify("current_user", &user.id.to_string());
conn.authorize();

if !conn.is_authorized() {
    // close the socket
}
let uid = conn.identifier("current_user");
```

## Wire protocol

The server speaks the ActionCable protocol: `ServerFrame` (`welcome`, `ping`,
`confirm_subscription`, `reject_subscription`), inbound `CableFrame` (`subscribe`,
`unsubscribe`, `message`), and `ServerMessage` (a broadcast's `identifier` + `message`). A
heartbeat ping keeps connections alive. This is wire-compatible with the standard
`@rails/actioncable` JavaScript client.

## Testing

`MemoryPubSub` makes broadcasts observable in-process — subscribe, broadcast, and assert on
the received `ServerMessage`.

```rust
use doido::cable::{Cable, MemoryPubSub, ServerMessage};
use std::sync::Arc;
use serde_json::json;

let cable = Cable::new(Arc::new(MemoryPubSub::new()));
let mut rx = cable.pubsub().subscribe("room:1").await?;
cable.broadcast("room:1", "ChatChannel", json!({ "text": "hi" })).await?;
let raw = rx.recv().await?;
```

## Spec vs. implementation

> `Channel::received(ctx, data)` receives the whole payload — there is **no** separate
> `action` argument, so you dispatch on the data yourself (e.g. an `action` field).
> `ChannelContext` is a plain data struct (`identifier`, `stream`); build stream names with
> `streams::stream_from` and broadcast via `Cable`.

## See also

- [Middleware & sessions](@/docs/reference/middleware.md) — authenticating the WebSocket upgrade.
- [Cache](@/docs/reference/cache.md) — the Redis backend is shared with pub/sub.
- [Generators & CLI](@/docs/reference/generators.md) — `doido generate channel`.
