+++
title = "Arquitetura"
description = "Como o Doido é estruturado — mapa de crates, boot e princípios de design."
weight = 1
+++

> **Especificação de design:** [`docs/00-overview.md`](https://github.com/doido-rs/doido/blob/master/docs/00-overview.md)
> e [`docs/ARCHITECTURE.md`](https://github.com/doido-rs/doido/blob/master/docs/ARCHITECTURE.md).

O Doido é um **workspace de crates focados**, não um monólito. Cada subsistema espelha
um componente Rails e pode ser usado isoladamente; geradores e CLI conectam tudo num
layout convencional de aplicação.

## Mapa de crates

| Crate | Equivalente Rails | Responsabilidade |
|-------|-------------------|------------------|
| `doido-core` | Active Support | Erros, inflector, utilitários |
| `doido-config` | — | YAML por ambiente, credenciais criptografadas |
| `doido-controller` | Action Dispatch + Controller | Rotas, controllers, middleware |
| `doido-model` | Active Record | Re-exports sea-orm, pool, helpers de teste |
| `doido-view` | Action View | Templates Tera, layouts, helpers |
| `doido-generators` | Generators + CLI | Scaffolds, `cargo doido server`, `cargo doido db` |
| `doido-mailer` | Action Mailer | Composição e entrega de e-mail |
| `doido-jobs` | Active Job | Jobs em background e filas |
| `doido-cache` | Active Support Cache | Cache stores plugáveis |
| `doido-cable` | Action Cable | Canais WebSocket e pub/sub |
| `doido-storage` | Active Storage | Arquivos anexados e blob storage |

## Princípios de design

**Convenção sobre configuração.** Rotas em `config/routes.rs`, templates em `views/`,
models em `models/` — os geradores montam o layout para você focar na lógica de negócio.

**Specs TDD-first.** Cada crate tem uma spec em `docs/` no repositório. Este manual é
a camada curada; as specs são a fonte da verdade para intenção de design.

**Stack async-native.** Controllers são handlers `async fn` no axum. ORM é sea-orm.

**Backends plugáveis.** Jobs, cache, mail, storage, cable e sessões aceitam backends
intercambiáveis — memória/SQLite no dev, Redis/Postgres/S3 em produção.

## Sequência de boot

1. Carregar `config/<env>.yml` (com descriptografia opcional de credenciais).
2. Conectar o pool do banco (`doido-model`).
3. Montar o router axum a partir de `config/routes.rs` (`doido-controller`).
4. Empilhar middleware (logging, sessões, CORS, …) conforme config.
5. Bind e servir (`cargo doido server`).

Veja **[Primeiros passos](/pt/docs/tutorials/getting-started/)** na prática ou a
**[Referência](/pt/docs/reference/)** para APIs de subsistemas.
