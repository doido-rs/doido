# doido-cable — Spec

Rails analogue: **Action Cable**

> **Status (2026-07-28): partial.** Implemented: ActionCable wire frames (`protocol`),
> `PubSub` backends (memory/redis/db), the `Channel` trait, a real `#[channel]` macro, and
> heartbeat helpers. **Not yet implemented — no live server:** there is no
> `axum::extract::ws` upgrade handler anywhere, no `server.rs`, no connection lifecycle /
> ping loop, no `ChannelRegistry`, no subscription routing/dispatch, no
> `ctx.stream_from/transmit/params/stop_all_streams`, and no `cable!()` route macro. Today
> this is a protocol + pub/sub library; the sections below describe the full server target.
> See [ARCHITECTURE.md](ARCHITECTURE.md).

## Decisions (resolved in interview)

- **Channel definition:** `#[channel]` macro generates `Channel` trait impl — same pattern as `#[controller]`
- **Pub/Sub backends:** pluggable — in-memory, Redis, Database selectable via config
- **Authentication:** Tower middleware (transport-level) + `#[cable_connection]` struct (identity resolution)
- **Broadcasting:** both global `Cable::broadcast_to()` and `ctx.cable.broadcast_to()`; stringly-typed stream names
- **Wire protocol:** ActionCable wire protocol — compatible with `@rails/actioncable` JS client out of the box
- **Generator:** `doido generate channel <Name> [actions]` added to `doido-generators`

## ActionCable Wire Protocol

Client→Server messages:
```json
{ "command": "subscribe",   "identifier": "{\"channel\":\"ChatChannel\",\"room\":\"1\"}" }
{ "command": "unsubscribe", "identifier": "{\"channel\":\"ChatChannel\",\"room\":\"1\"}" }
{ "command": "message",     "identifier": "{\"channel\":\"ChatChannel\",\"room\":\"1\"}", "data": "{\"action\":\"speak\",\"message\":\"hello\"}" }
```

Server→Client messages:
```json
{ "type": "welcome" }
{ "type": "confirm_subscription",  "identifier": "{\"channel\":\"ChatChannel\",\"room\":\"1\"}" }
{ "type": "reject_subscription",   "identifier": "{\"channel\":\"ChatChannel\",\"room\":\"1\"}" }
{ "identifier": "{\"channel\":\"ChatChannel\",\"room\":\"1\"}", "message": { "content": "hello" } }
{ "type": "ping", "message": 1718000000 }
```

Ping interval configurable via `[cable] ping_interval = 3` (seconds).

## Channel Definition

```rust
#[channel]
struct ChatChannel;

impl ChatChannel {
    // Lifecycle — called on subscribe
    async fn subscribed(ctx: &ChannelContext) -> Result<()> {
        let room = ctx.params["room"].as_str()?;
        ctx.stream_from(format!("chat:{room}")).await
    }

    // Lifecycle — called on unsubscribe / disconnect
    async fn unsubscribed(ctx: &ChannelContext) -> Result<()> {
        Ok(())
    }

    // Action — called by client `{ "action": "speak", "message": "hello" }`
    async fn speak(ctx: &ChannelContext, data: serde_json::Value) -> Result<()> {
        let room = ctx.params["room"].as_str()?;
        Cable::broadcast_to(format!("chat:{room}"), data).await
    }
}
```

`#[channel]` macro responsibilities:
- Implements `Channel` trait on the struct
- Routes incoming `action` messages to matching method names
- Registers the channel in the global `ChannelRegistry` by struct name (`"ChatChannel"`)

## `Channel` Trait

```rust
pub trait Channel: Send + Sync {
    async fn subscribed(&self, ctx: &ChannelContext) -> Result<()>;
    async fn unsubscribed(&self, ctx: &ChannelContext) -> Result<()>;
    async fn receive(&self, ctx: &ChannelContext, action: &str, data: Value) -> Result<()>;
}
```

## `ChannelContext`

```rust
ctx.params               // subscription params from identifier JSON
ctx.identity             // resolved identity from CableConnection
ctx.stream_from(name)    // subscribe this connection to a named stream
ctx.stop_all_streams()   // unsubscribe from all streams
ctx.transmit(data)       // send message directly to this connection only
ctx.cable                // injected CableHandle for broadcasting
```

## Authentication — Two Layers

### Layer 1: Tower Middleware (transport-level)

The WebSocket upgrade endpoint is protected by the same opt-in middleware stack as HTTP routes. Unauthenticated upgrade requests are rejected before the socket is opened.

```rust
routes! {
    namespace!(cable, {
        middleware!(AuthLayer);   // rejects if not authenticated
        cable!(ChatChannel);
        cable!(NotificationsChannel);
    });
}
```

### Layer 2: `#[cable_connection]` (identity resolution)

Resolves *who* the authenticated user is from the upgrade request:

```rust
#[cable_connection]
struct AppConnection;

impl AppConnection {
    async fn connect(ctx: &ConnectionContext) -> Result<Identity> {
        // ctx has access to headers, cookies, query params
        let session = ctx.session.get::<UserId>("user_id")?;
        let user = User::find_by_id(session).one(&ctx.db).await?;
        Ok(Identity::from(user))
    }
}
```

`Identity` is available on all channels via `ctx.identity`.  
Returning `Err` from `connect` rejects the WebSocket connection.

## Broadcasting API

### Global (callable from anywhere — controllers, jobs, mailers)

```rust
Cable::broadcast_to("chat:room_1", json!({ "message": "hello" })).await?;
```

### Injected via `ctx.cable` (inside controllers and channels)

```rust
ctx.cable.broadcast_to("notifications:user_42", json!({ "event": "new_message" })).await?;
```

Both call the same underlying `PubSub` backend.

## Pub/Sub Backends

```rust
pub trait PubSub: Send + Sync {
    async fn publish(&self, stream: &str, message: &[u8]) -> Result<()>;
    async fn subscribe(&self, stream: &str) -> Result<impl Stream<Item = Vec<u8>>>;
    async fn unsubscribe(&self, stream: &str) -> Result<()>;
}
```

| Backend | Feature flag | Description |
|---------|-------------|-------------|
| `InMemoryPubSub` | default | `tokio::broadcast`, single-process |
| `RedisPubSub` | `feature = "cable-redis"` | Redis pub/sub, multi-process |
| `DbPubSub` | `feature = "cable-db"` | sea-orm polling, no extra infra |

```toml
[cable]
backend = "memory"        # "memory" | "redis" | "db"
ping_interval = 3         # seconds
mount = "/cable"          # WebSocket endpoint path

[cable.redis]
url = "${REDIS_URL}"
```

## `cable!` Route Macro

Registers a channel at the cable endpoint inside `routes!`:

```rust
routes! {
    cable!(ChatChannel);
    cable!(NotificationsChannel);
    cable!(AppearanceChannel);
}
```

All channels share the same WebSocket endpoint (`/cable` by default). The protocol `identifier` field routes each message to the correct channel.

## Generator

Added to `doido-generators`:

```
doido generate channel Chat speak typing
```

Creates:
- `channels/chat_channel.rs` — `#[channel]` struct with `subscribed`, `unsubscribed`, and specified actions
- `views/mailers/` — no views (cable channels are not view-rendered)

Does **not** inject into `config/routes.rs` automatically — prints a hint:
```
Add to config/routes.rs:
    cable!(ChatChannel);
```

## Module Structure

```
doido-cable/
  src/
    lib.rs
    channel.rs          ← Channel trait + ChannelContext
    connection.rs       ← CableConnection trait + ConnectionContext + Identity
    registry.rs         ← ChannelRegistry (name → Box<dyn Channel>)
    protocol.rs         ← ActionCable wire protocol encode/decode
    broadcast.rs        ← Cable::broadcast_to + CableHandle
    pubsub/
      mod.rs            ← PubSub trait
      memory.rs
      redis.rs
      db.rs
    server.rs           ← axum WebSocket handler, ping loop, connection lifecycle
```

## Known Requirements

- WebSocket handler built on `axum::extract::ws::WebSocket`
- Full ActionCable wire protocol — compatible with `@rails/actioncable`
- `ChannelRegistry` maps channel name string → channel handler (populated by `#[channel]` macro)
- Ping loop runs every `cable.ping_interval` seconds per connection
- `Cable::broadcast_to` is a global async fn backed by the configured `PubSub`
- In test env, `InMemoryPubSub` always used

## TDD Surface

- Test protocol decoder parses all ActionCable client→server message types
- Test protocol encoder produces correct server→client message JSON
- Test `#[channel]` macro routes `action` field to correct method
- Test `subscribed` called on subscribe command
- Test `unsubscribed` called on disconnect
- Test `stream_from` registers connection on named stream
- Test `Cable::broadcast_to` delivers to all stream subscribers
- Test `CableConnection::connect` returning `Err` closes the socket
- Test `Identity` available on `ctx.identity` inside channel actions
- Test `InMemoryPubSub` fan-out to multiple subscribers
- Test ping messages sent at configured interval
- Integration test: connect fake client → subscribe → broadcast → assert message received
- Integration test: invalid `connect` → connection rejected, no channel access
