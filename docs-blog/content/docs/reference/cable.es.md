+++
title = "Cable (WebSockets)"
description = "Canales en tiempo real con la macro #[channel], broadcast por stream, pub/sub conectable y autenticación de conexión."
weight = 11
+++

> **Especificación de diseño:** [`docs/12-cable.md`](https://github.com/doido-rs/doido/blob/master/docs/12-cable.md).
> Esta guía documenta la API tal como está implementada en `doido-cable`.

**Análogo en Rails: Action Cable.** Los canales ofrecen comunicación WebSocket bidireccional
en tiempo real hablando el protocolo de cable de ActionCable (compatible con
`@rails/actioncable`). Las conexiones se suscriben a streams con nombre; un broadcast a un
stream alcanza a todos los suscriptores. El Pub/Sub es conectable (en memoria, Redis o base
de datos).

## Vistazo general

```rust
use doido::cable::{channel, Channel, ChannelContext, Cable, MemoryPubSub, CableConnection};
```

## Definir un canal

`#[channel]` deriva el nombre de registro del canal. Implementa los hooks de ciclo de vida
del trait `Channel`: `subscribed` (al suscribirse), `unsubscribed` (al desconectar) y
`received` (un mensaje entrante). Enruta según el payload del mensaje dentro de `received`.

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
        // Despacha según el payload (p. ej. un campo "action", al estilo ActionCable):
        if data["action"] == "speak" {
            // …hacer broadcast del mensaje a la sala…
        }
        Ok(())
    }
}
```

`ChannelContext` transporta el `identifier` de la suscripción y el `stream` actual.

## Broadcast a streams

`Cable` publica en un stream con nombre desde cualquier parte de la app: `broadcast` envía
un mensaje ActionCable estructurado (identifier + JSON), `broadcast_to` envía una cadena
cruda. Usa `streams::stream_from` para construir nombres de stream consistentes.

```rust
use doido::cable::{Cable, MemoryPubSub, streams};
use std::sync::Arc;
use serde_json::json;

let cable = Cable::new(Arc::new(MemoryPubSub::new()));
let stream = streams::stream_from("chat:room:1");

cable.broadcast(&stream, "ChatChannel", json!({ "text": "Hi" })).await?;
cable.broadcast_to(&stream, "raw string").await?;
```

## Suscribirse a un stream

Suscríbete mediante el pub/sub subyacente para recibir un canal de broadcast de mensajes
(usado internamente para retransmitir al WebSocket).

```rust
let mut rx = cable.pubsub().subscribe("chat:room:1").await?;
let message = rx.recv().await?; // una cadena ServerMessage en JSON
```

## Backends de Pub/Sub

`PubSub` es el trait conectable (`subscribe`, `publish`). `MemoryPubSub` (en proceso) es el
por defecto; `RedisPubSub` (feature `cable-redis`) distribuye entre procesos; `DbPubSub`
(feature `cable-db`) hace polling de la base de datos.

```rust
use doido::cable::{Cable, MemoryPubSub};
let cable = Cable::new(std::sync::Arc::new(MemoryPubSub::new()));
```

## Identidad y autorización de la conexión

`CableConnection` modela la identidad de una conexión y si está autorizada — el análogo de
`identified_by` / `reject_unauthorized_connection` de Rails. Resuelve la identidad cuando el
socket conecta y rechaza las no autorizadas antes de que corra cualquier canal.

```rust
use doido::cable::CableConnection;

let mut conn = CableConnection::new();
conn.identify("current_user", &user.id.to_string());
conn.authorize();

if !conn.is_authorized() {
    // cierra el socket
}
let uid = conn.identifier("current_user");
```

## Protocolo de cable

El servidor habla el protocolo ActionCable: `ServerFrame` (`welcome`, `ping`,
`confirm_subscription`, `reject_subscription`), `CableFrame` entrante (`subscribe`,
`unsubscribe`, `message`) y `ServerMessage` (el `identifier` + `message` de un broadcast). Un
ping de heartbeat mantiene vivas las conexiones. Esto es compatible a nivel de cable con el
cliente JavaScript estándar `@rails/actioncable`.

## Pruebas

`MemoryPubSub` hace los broadcasts observables en proceso — suscríbete, haz broadcast y
verifica el `ServerMessage` recibido.

```rust
use doido::cable::{Cable, MemoryPubSub, ServerMessage};
use std::sync::Arc;
use serde_json::json;

let cable = Cable::new(Arc::new(MemoryPubSub::new()));
let mut rx = cable.pubsub().subscribe("room:1").await?;
cable.broadcast("room:1", "ChatChannel", json!({ "text": "hi" })).await?;
let raw = rx.recv().await?;
```

## Especificación vs. implementación

> `Channel::received(ctx, data)` recibe el payload completo — **no** hay un argumento
> `action` separado, así que despachas según los datos tú mismo (p. ej. un campo `action`).
> `ChannelContext` es una struct de datos simple (`identifier`, `stream`); construye nombres
> de stream con `streams::stream_from` y haz broadcast vía `Cable`.

## Véase también

- [Middleware y sesiones](@/docs/reference/middleware.es.md) — autenticar el upgrade del WebSocket.
- [Cache](@/docs/reference/cache.es.md) — el backend Redis se comparte con el pub/sub.
- [Generadores y CLI](@/docs/reference/generators.es.md) — `doido generate channel`.
