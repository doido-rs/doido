+++
title = "Arquitectura"
description = "Cómo está estructurado Doido — mapa de crates, boot y principios de diseño."
weight = 1
+++

> **Especificación de diseño:** [`docs/00-overview.md`](https://github.com/doido-rs/doido/blob/master/docs/00-overview.md)
> y [`docs/ARCHITECTURE.md`](https://github.com/doido-rs/doido/blob/master/docs/ARCHITECTURE.md).

Doido es un **workspace de crates enfocados**, no un monolito. Cada subsistema refleja
un componente de Rails y puede usarse por separado; generadores y CLI los conectan en
un layout convencional de aplicación.

## Mapa de crates

| Crate | Equivalente Rails | Responsabilidad |
|-------|-------------------|-----------------|
| `doido-core` | Active Support | Errores, inflector, utilidades |
| `doido-config` | — | YAML por entorno, credenciales cifradas |
| `doido-controller` | Action Dispatch + Controller | Rutas, controladores, helpers de controlador, middleware |
| `doido-auth` | Devise + OmniAuth + JWT | Trait AuthUser, estrategias, extractors, generadores |
| `doido-model` | Active Record | Re-exports sea-orm, pool, helpers de prueba |
| `doido-view` | Action View | Plantillas Tera, layouts, helpers |
| `doido-generators` | Generators + CLI | Scaffolds, `cargo doido server`, `cargo doido db` |
| `doido-mailer` | Action Mailer | Composición y entrega de correo |
| `doido-jobs` | Active Job | Jobs en segundo plano y colas |
| `doido-cache` | Active Support Cache | Cache stores conectables |
| `doido-cable` | Action Cable | Canales WebSocket y pub/sub |
| `doido-storage` | Active Storage | Archivos adjuntos y blob storage |

## Principios de diseño

**Convención sobre configuración.** Rutas en `config/routes.rs`, plantillas en `views/`,
modelos en `models/`, helpers de controlador en `helpers/` — los generadores montan el
layout para que te enfoques en la lógica.

**Specs TDD-first.** Cada crate tiene una spec en `docs/` del repositorio. Este manual
es la capa curada; las specs son la fuente de verdad para la intención de diseño.

**Stack async-native.** Los controladores son handlers `async fn` en axum. ORM es sea-orm.

**Backends conectables.** Jobs, caché, mail, storage, cable, sesiones y estrategias de auth
aceptan backends intercambiables — memoria/SQLite en dev, Redis/Postgres/S3 en producción.

## Secuencia de boot

1. Cargar `config/<env>.yml` (con descifrado opcional de credenciales).
2. Conectar el pool de base de datos (`doido-model`).
3. Inicializar auth cuando esté habilitado (`doido-auth::init`).
4. Construir el router axum desde `config/routes.rs` (`doido-controller`).
5. Apilar middleware (logging, sesiones, auth, CORS, …) según config.
6. Bind y servir (`cargo doido server`).

Consulta **[Primeros pasos](/es/docs/tutorials/getting-started/)** en la práctica o la
**[Referencia](/es/docs/reference/)** para APIs de subsistemas.
