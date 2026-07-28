+++
title = "Primeros pasos"
description = "Crea, ejecuta y entiende tu primera aplicación Doido."
weight = 1
+++

Doido sigue convenciones al estilo Rails: un único binario `doido` genera la
aplicación, ejecuta el servidor, gestiona la base de datos y corre los
generadores de código.

## Crea una aplicación

```bash
# Crea una nueva aplicación (sqlite por defecto; --database=postgres|mysql)
doido new blog
cd blog

# Prepara la base de datos y ejecuta las migraciones pendientes
doido db create
doido db migrate

# Levanta el servidor HTTP en http://0.0.0.0:3000
doido server
```

`GET /` responde con JSON desde el `HelloController` generado:

```json
{ "message": "Hello word!" }
```

## Una muestra del código

Un controlador es un bloque `impl` normal anotado con `#[controller]`:

```rust
use doido_controller::controller;
use serde_json::json;

pub struct HelloController;

#[controller]
impl HelloController {
    pub async fn index(ctx: doido_controller::Context) -> doido_controller::Response {
        ctx.json(json!({ "message": "Hello word!" }))
    }
}
```

Las rutas se declaran con la macro `routes!` en `config/routes.rs`:

```rust
use crate::controllers::HelloController;
use doido::controller::{axum, routes};

pub fn router() -> axum::Router {
    routes! {
        get!("/", HelloController::index);
        // resources!(PostsController);   // las 7 rutas REST
    }
}
```

## Estructura del proyecto

Una aplicación generada sigue convenciones al estilo Rails:

```
my-app/
├── Cargo.toml
├── src/main.rs              ← delega a doido::run(routes)
├── config/
│   ├── application.toml      ← configuración base
│   ├── development.yml       ← overrides por entorno
│   ├── test.yml
│   ├── production.yml
│   ├── routes.rs            ← macro routes!
│   └── inflection.yaml      ← reglas de pluralización personalizadas
├── app/
│   ├── controllers/
│   ├── models/
│   └── views/
├── db/
│   ├── migration/           ← crate de migraciones de SeaORM
│   └── schema/
└── tests/
```

## Configuración

La configuración es por capas: `config/application.toml` provee la base, luego
`config/<env>.yml` (development / test / production) hace override por entorno.
Las credenciales cifradas y las variables de entorno `SECTION__KEY` hacen override
por encima.

```yaml
# config/development.yml
server:
  bind: 0.0.0.0
  port: 3000
database:
  url: sqlite://db/development.db
logger:
  level: debug
  format: verbose
cache:
  type: memory
```

## Próximos pasos

- **[Instalación](@/docs/installation.es.md)** — requisitos previos y cómo instalar la CLI.
- **[CLI y generadores](@/docs/cli.es.md)** — todos los comandos de runtime y generadores de código.
- **[Controladores y enrutamiento](@/docs/guides/controllers.es.md)** — la guía de petición/respuesta.
