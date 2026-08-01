+++
title = "Generadores y CLI"
description = "El binario doido: comandos de runtime, generadores de código, la DSL de campos, inyección automática de rutas y generadores personalizados."
weight = 7
+++

> **Especificación de diseño:** [`docs/06-cli.md`](https://github.com/doido-rs/doido/blob/master/docs/06-cli.md)
> y [`docs/06b-generators.md`](https://github.com/doido-rs/doido/blob/master/docs/06b-generators.md).
> Esta guía documenta la API tal como está implementada en `doido-generators`. Para una
> tabla rápida de comandos, ve [CLI y generadores](@/docs/reference/cli.es.md).

**Análogo en Rails: el binario `rails` + generadores.** `doido-generators` impulsa el único
binario `doido` — tanto los comandos de runtime (`server`, `db`, `worker`, …) como los
generadores de código (`generate scaffold`, `generate model`, …). Una app generada arranca
llamando a `doido::generators::run(Some(routes))`.

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
| `doido server` | Inicia el servidor HTTP axum |
| `doido routes` | Imprime la tabla de rutas |
| `doido console` | Consola interactiva con el contexto de la app |
| `doido db <cmd>` | `migrate`, `rollback`, `reset`, `status`, `seed` |
| `doido worker [--once]` | Ejecuta el worker de jobs en segundo plano |
| `doido jobs <cmd>` | Inspecciona/reintenta/descarta jobs en segundo plano |
| `doido credentials <cmd>` | Gestiona credenciales |
| `doido generate <name> …` | Ejecuta un generador de código |
| `doido destroy <name> …` | Revierte un generador |
| `doido new <app>` | Crea una nueva aplicación |

```bash
doido db migrate          # ejecuta las migraciones pendientes
doido worker --once       # vacía la cola y sale
doido routes              # imprime todas las rutas registradas
```

## Crear una aplicación

`doido new` genera un proyecto al estilo Rails; elige el driver de base de datos con
`--database`.

```bash
doido new blog --database=sqlite   # o postgres | mysql
cd blog
doido db create && doido db migrate
doido server
```

## Generadores de código

Ejecuta `doido generate` sin argumentos para listar todos los generadores registrados. Cada
uno escribe archivos (y algunos inyectan rutas):

| Generador | Genera |
|-----------|--------|
| `model` | `app/models/<name>.rs` + migración |
| `migration` | una migración independiente |
| `controller` | un `#[controller]` con stubs de action (+ ruta) |
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

```bash
doido generate model Post title:string body:text
doido generate scaffold Post title:string body:text     # CRUD completo
doido generate controller Pages home about
doido generate mailer User welcome
```

## La DSL de campos

Los generadores de model, scaffold y resource reciben campos como `name:type[:modifier…]`.
Los tipos mapean a columnas de migración; los modificadores añaden constraints e índices.

```bash
doido generate model Post \
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

`doido destroy` elimina lo que el `generate` correspondiente creó.

```bash
doido generate scaffold Post title:string
doido destroy  scaffold Post           # elimina los archivos generados (y la ruta)
```

## Generadores personalizados

El sistema de generadores es un registro extensible. Implementa el trait `Generator`
(devolviendo `GeneratedFile`s) y regístralo; `doido generate generator <name>` genera un
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
- [Jobs](@/docs/reference/jobs.es.md), [Mailer](@/docs/reference/mailer.es.md), [Cable](@/docs/reference/cable.es.md) — sus generadores.
