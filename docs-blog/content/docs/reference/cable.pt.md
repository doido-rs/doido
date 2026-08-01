+++
title = "Cable (WebSockets)"
description = "Canais em tempo real com a macro #[channel], broadcast por stream, pub/sub plugável e autenticação de conexão."
weight = 11
+++

> **Especificação de design:** [`docs/12-cable.md`](https://github.com/doido-rs/doido/blob/master/docs/12-cable.md).
> Este guia documenta a API como implementada em `doido-cable`.

**Análogo no Rails: Action Cable.** Os canais dão comunicação WebSocket bidirecional em
tempo real falando o protocolo de fio do ActionCable (compatível com `@rails/actioncable`).
As conexões assinam streams nomeados; um broadcast para um stream alcança todos os
assinantes. O Pub/Sub é plugável (em memória, Redis ou banco de dados).

## Visão geral

```rust
use doido::cable::{channel, Channel, ChannelContext, Cable, MemoryPubSub, CableConnection};
```

## Definindo um canal

`#[channel]` deriva o nome de registro do canal. Implemente os hooks de ciclo de vida do
trait `Channel`: `subscribed` (na assinatura), `unsubscribed` (na desconexão) e `received`
(uma mensagem recebida). Faça o roteamento pelo payload da mensagem dentro de `received`.

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
        // Despacha pelo payload (ex.: um campo "action", no estilo ActionCable):
        if data["action"] == "speak" {
            // …faz broadcast da mensagem para a sala…
        }
        Ok(())
    }
}
```

`ChannelContext` carrega o `identifier` da assinatura e o `stream` atual.

## Broadcast para streams

`Cable` publica em um stream nomeado de qualquer lugar da app: `broadcast` envia uma mensagem
ActionCable estruturada (identifier + JSON), `broadcast_to` envia uma string crua. Use
`streams::stream_from` para montar nomes de stream consistentes.

```rust
use doido::cable::{Cable, MemoryPubSub, streams};
use std::sync::Arc;
use serde_json::json;

let cable = Cable::new(Arc::new(MemoryPubSub::new()));
let stream = streams::stream_from("chat:room:1");

cable.broadcast(&stream, "ChatChannel", json!({ "text": "Hi" })).await?;
cable.broadcast_to(&stream, "raw string").await?;
```

## Assinando um stream

Assine pelo pub/sub subjacente para receber um canal de broadcast de mensagens (usado
internamente para repassar ao WebSocket).

```rust
let mut rx = cable.pubsub().subscribe("chat:room:1").await?;
let message = rx.recv().await?; // uma string ServerMessage em JSON
```

## Backends de Pub/Sub

`PubSub` é o trait plugável (`subscribe`, `publish`). `MemoryPubSub` (em processo) é o
padrão; `RedisPubSub` (feature `cable-redis`) distribui entre processos; `DbPubSub` (feature
`cable-db`) faz polling do banco.

```rust
use doido::cable::{Cable, MemoryPubSub};
let cable = Cable::new(std::sync::Arc::new(MemoryPubSub::new()));
```

## Identidade & autorização da conexão

`CableConnection` modela a identidade de uma conexão e se ela está autorizada — o análogo ao
`identified_by` / `reject_unauthorized_connection` do Rails. Resolva a identidade quando o
socket conecta e rejeite as não autorizadas antes de qualquer canal rodar.

```rust
use doido::cable::CableConnection;

let mut conn = CableConnection::new();
conn.identify("current_user", &user.id.to_string());
conn.authorize();

if !conn.is_authorized() {
    // fecha o socket
}
let uid = conn.identifier("current_user");
```

## Protocolo de fio

O servidor fala o protocolo ActionCable: `ServerFrame` (`welcome`, `ping`,
`confirm_subscription`, `reject_subscription`), `CableFrame` de entrada (`subscribe`,
`unsubscribe`, `message`) e `ServerMessage` (o `identifier` + `message` de um broadcast). Um
ping de heartbeat mantém as conexões vivas. Isso é compatível no fio com o cliente
JavaScript padrão `@rails/actioncable`.

## Testes

`MemoryPubSub` torna os broadcasts observáveis em processo — assine, faça broadcast e
verifique o `ServerMessage` recebido.

```rust
use doido::cable::{Cable, MemoryPubSub, ServerMessage};
use std::sync::Arc;
use serde_json::json;

let cable = Cable::new(Arc::new(MemoryPubSub::new()));
let mut rx = cable.pubsub().subscribe("room:1").await?;
cable.broadcast("room:1", "ChatChannel", json!({ "text": "hi" })).await?;
let raw = rx.recv().await?;
```

## Especificação vs. implementação

> `Channel::received(ctx, data)` recebe o payload inteiro — **não** há um argumento `action`
> separado, então você mesmo despacha pelos dados (ex.: um campo `action`). `ChannelContext`
> é uma struct de dados simples (`identifier`, `stream`); monte nomes de stream com
> `streams::stream_from` e faça broadcast via `Cable`.

## Veja também

- [Middleware & sessões](@/docs/reference/middleware.pt.md) — autenticando o upgrade do WebSocket.
- [Cache](@/docs/reference/cache.pt.md) — o backend Redis é compartilhado com o pub/sub.
- [Geradores & CLI](@/docs/reference/generators.pt.md) — `cargo doido generate channel`.
