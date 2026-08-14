+++
title = "Generadores y CLI"
description = "El binario doido: comandos de runtime, generadores de código, la DSL de campos, inyección automática de rutas y generadores personalizados."
weight = 7
+++

> **Especificación de diseño:** [`docs/06-cli.md`](https://github.com/doido-rs/doido/blob/master/docs/06-cli.md)
> y [`docs/06b-generators.md`](https://github.com/doido-rs/doido/blob/master/docs/06b-generators.md).
> Esta guía documenta la API tal como está implementada en `doido-generators`. Para una
> tabla rápida de comandos, ve [CLI y generadores](@/docs/reference/cli.es.md).

**Análogo en Rails: el binario `rails` + generadores.** `doido-generators` impulsa
`doido new` y el alias local `cargo doido` — comandos de runtime (`server`, `db`,
`worker`, …) y generadores de código (`generate scaffold`, `generate model`, …).
Una app generada arranca llamando a `doido::generators::run(Some(routes))`.

## Vistazo general

```rust
// src/main.rs de una app generada
#[tokio::main]
async fn main() {
    doido::generators::run(Some(config::routes::router())).await;
}
```

## Comandos de runtime

| Comando | Descripción |
|---------|-------------|
| `cargo doido server` | Inicia el servidor HTTP axum |
| `cargo doido routes` | Imprime la tabla de rutas |
| `cargo doido console` | Consola interactiva con el contexto de la app |
| `cargo doido db <cmd>` | `migrate`, `rollback`, `reset`, `status`, `seed` |
| `cargo doido worker [--once]` | Ejecuta el worker de jobs en segundo plano |
| `cargo doido jobs <cmd>` | Inspecciona/reintenta/descarta jobs en segundo plano |
| `cargo doido credentials <cmd>` | Gestiona credenciales |
| `cargo doido generate <name> …` | Ejecuta un generador de código |
| `cargo doido destroy <name> …` | Revierte un generador |
| `doido new <app>` | Crea una nueva aplicación |

```bash
cargo doido db migrate          # ejecuta las migraciones pendientes
cargo doido worker --once       # vacía la cola y sale
cargo doido routes              # imprime todas las rutas registradas
```

## Crear una aplicación

`doido new` genera un proyecto al estilo Rails; elige el driver de base de datos con
`--database`.

```bash
doido new blog --database=sqlite   # o postgres | mysql
cd blog
cargo doido db create && cargo doido db migrate
cargo doido server
```

`doido new` también genera `db/seed/` — un crate de workspace que se ejecuta con
`cargo doido db seed` para insertar fixtures vía `app/models/`. Edita
`db/seed/src/main.rs` después de generar modelos.

```bash
cargo doido generate model Post title:string body:text
cargo doido db seed
```

## Generadores de código

Ejecuta `cargo doido generate` sin argumentos para listar todos los generadores registrados. Cada
uno escribe archivos (y algunos inyectan rutas). Las subsecciones de abajo dan un ejemplo
ejecutable para cada uno; los campos `name:type` que reciben `model`, `scaffold` y `resource`
están documentados en [La DSL de campos](#la-dsl-de-campos).

| Generador | Genera |
|-----------|--------|
| `model` | `app/models/<name>.rs` + migración |
| `migration` | una migración independiente |
| `controller` | un `#[controller]` con stubs de action (+ ruta) |
| `helper` | un helper de controlador en `app/helpers/` |
| `scaffold` | modelo + migración + controlador + vistas + ruta |
| `resource` | modelo + migración + controlador + ruta (sin vistas) |
| `mailer` | un mailer + plantillas |
| `job` | un job en segundo plano |
| `channel` | un canal WebSocket |
| `templates` | plantillas de vista para un controlador existente |
| `locale` | un archivo de locale |
| `generator` | el esqueleto de un nuevo generador personalizado |
| `storage:install` | tablas de storage + config |
| `storage:adapter` | el esqueleto de un adapter de storage personalizado |

### model

Crea `app/models/<name>.rs` y su migración correspondiente.

```bash
cargo doido generate model Post title:string body:text
```

### migration

Una migración independiente; añade o elimina columnas con la DSL de campos.

```bash
cargo doido generate migration add_published_to_posts published:boolean
```

### controller

Un `#[controller]` con un stub de action por nombre, más su ruta.

```bash
cargo doido generate controller Pages home about
```

### helper

Un módulo de helper de controlador en `app/helpers/`.

```bash
cargo doido generate helper Posts
```

### scaffold

El stack completo de CRUD — modelo, migración, controlador, vistas y ruta — en un comando.

```bash
cargo doido generate scaffold Post title:string:not_null body:text author:references
```

### resource

Como `scaffold` pero sin vistas — el stack para modo API.

```bash
cargo doido generate resource Post title:string body:text
cargo doido generate resource Post title:string --api   # solo JSON
```

### mailer

Un mailer más una plantilla por action.

```bash
cargo doido generate mailer User welcome
```

### job

Un job en segundo plano en `app/jobs/`.

```bash
cargo doido generate job SendNewsletter
```

### channel

Un canal WebSocket en `app/channels/`.

```bash
cargo doido generate channel Chat
```

### templates

Extrae las plantillas de vista integradas de un controlador existente para personalizarlas.

```bash
cargo doido generate templates Posts
```

### locale

Un archivo de locale i18n inicial (por defecto `en`).

```bash
cargo doido generate locale pt
```

### generator

Genera el esqueleto de un nuevo generador personalizado — ve [Generadores personalizados](#generadores-personalizados).

```bash
cargo doido generate generator policy
```

### storage:install

Tablas de storage más config — ve [Storage](@/docs/reference/storage.es.md).

```bash
cargo doido generate storage:install
```

### storage:adapter

El esqueleto de un adapter de storage personalizado.

```bash
cargo doido generate storage:adapter Cloudinary
```

### Generadores de auth

Cuando `doido-auth` está en `Cargo.toml`, tres generadores adicionales aparecen en
**Auth (doido-auth)**:

| Generador | Genera |
|-----------|--------|
| `auth:install` | Migración + modelo User, controladores de auth, vistas, config, rutas |
| `auth:controller` | Controlador con `CurrentUser` / guards de auth |
| `auth:scaffold` | Scaffold con auth y ownership por `user_id` |

El camino más rápido es `doido new blog --database=sqlite --auth`, que añade `doido-auth`
y ejecuta `auth:install` por ti.

### auth:install

Migración + modelo User, controladores de auth, vistas, config y rutas.

```bash
cargo doido generate auth:install          # auth HTML con cookie/sesión
cargo doido generate auth:install --api    # endpoints de auth solo JSON
```

### auth:controller

Un controlador ya cableado con guards `CurrentUser` / `require_user`.

```bash
cargo doido generate auth:controller Dashboard
```

### auth:scaffold

Un scaffold con auth y ownership por `user_id`.

```bash
cargo doido generate auth:scaffold Post title:string body:text
```

## La DSL de campos

Los generadores de model, scaffold y resource reciben campos como `name:type[:modifier…]`.
Los tipos mapean a columnas de migración; los modificadores añaden constraints e índices.

```bash
cargo doido generate model Post \
  title:string:not_null \
  slug:string:unique \
  body:text \
  author:references \
  views:integer:index
```

## Inyección automática de rutas

Los generadores que producen un controlador (`scaffold`, `resource`, `controller`) parsean
`config/routes.rs`, insertan la ruta correspondiente (p. ej. `resources!(posts,
PostsController);`) dentro del bloque `routes! { … }` y omiten los controladores ya
registrados — de modo que un resource generado queda accesible sin editar las rutas a mano.

## Revertir un generador

`cargo doido destroy` elimina lo que el `generate` correspondiente creó.

```bash
cargo doido generate scaffold Post title:string
cargo doido destroy  scaffold Post           # elimina los archivos generados (y la ruta)
```

## Generadores personalizados

El sistema de generadores es un registro extensible. Implementa el trait `Generator`
(devolviendo `GeneratedFile`s) y regístralo; `cargo doido generate generator <name>` genera un
esqueleto por ti.

```rust
use doido::generators::{Generator, GeneratedFile};
use doido::Result;

struct PolicyGenerator;

impl Generator for PolicyGenerator {
    fn name(&self) -> &str { "policy" }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().unwrap_or("application");
        Ok(vec![GeneratedFile {
            path: format!("app/policies/{name}_policy.rs"),
            content: format!("// {name} policy\n"),
        }])
    }
}

// Regístralo, luego ejecuta/lista mediante el registro:
let mut registry = doido::generators::default_registry();
registry.register(Box::new(PolicyGenerator));
let files = registry.run("policy", &["post"])?;
let names = registry.list(); // incluye "policy"
```

## Véase también

- [Modelos](@/docs/reference/models.es.md) — lo que producen `generate model`/`migration`.
- [Controladores y enrutamiento](@/docs/reference/controllers.es.md) — el bloque `routes!` que editan los generadores.
- [Helpers de controlador](@/docs/reference/helpers.es.md) — lo que produce `generate helper`.
- [Jobs](@/docs/reference/jobs.es.md), [Mailer](@/docs/reference/mailer.es.md), [Cable](@/docs/reference/cable.es.md) — sus generadores.
- [Auth](@/docs/reference/auth.es.md) — `auth:install`, estrategias y extractors.
