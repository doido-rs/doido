+++
title = "CLI y generadores"
description = "Todos los comandos de runtime y generadores de código del binario doido."
weight = 3
+++

El CLI global `doido` crea nuevas aplicaciones. Dentro de un proyecto generado,
`cargo doido` (un alias de Cargo configurado por `doido new`) es el punto de
entrada para comandos de runtime y generación de código.

## Comandos de runtime

| Comando | Descripción |
|---------|-------------|
| `doido new <name>` | Crea una nueva aplicación (`--database=sqlite\|postgres\|mysql`) |
| `cargo doido server` | Inicia el servidor web |
| `cargo doido routes` | Imprime la tabla de rutas |
| `cargo doido console` | Inicia una consola interactiva |
| `cargo doido db <cmd>` | Crea bases de datos, ejecuta migraciones de SeaORM, genera entidades |
| `cargo doido jobs <cmd>` | Inspecciona y gestiona jobs en segundo plano |
| `cargo doido worker` | Ejecuta el worker de jobs (`--once` para vaciar y salir) |
| `cargo doido credentials <cmd>` | Gestiona credenciales cifradas con AES-256-GCM |
| `cargo doido generate <gen>` | Ejecuta un generador de código (ver abajo) |

## Generadores

Ejecuta `cargo doido generate` sin argumentos para listar todos los generadores
registrados:

| Generador | Genera |
|-----------|--------|
| `controller` | Un controlador con actions |
| `helper` | Un helper de controlador en `app/helpers/` |
| `model` | Un modelo + migración de SeaORM |
| `migration` | Una migración independiente |
| `scaffold` | Modelo, controlador, vistas, rutas — el CRUD completo |
| `job` | Un job en segundo plano |
| `mailer` | Un mailer con plantillas |
| `channel` | Un canal WebSocket |
| `templates` | Plantillas de vista para un controlador existente |
| `generator` | Un nuevo generador personalizado (el registro es extensible) |

Los generadores inyectan rutas automáticamente en `config/routes.rs` y respetan
las reglas de pluralización declaradas en `config/inflection.yaml`.

## Crates del workspace

| Crate | Análogo en Rails | Responsabilidad |
|-------|------------------|-----------------|
| `doido` | binario `rails` | Punto de entrada, runtime de la app |
| `doido-core` | Active Support | Traits, errores, inflector, logger, utilidades |
| `doido-controller` | Action Dispatch + Controller + Rack | DSL de rutas, peticiones, params, helpers de controlador, middleware Tower, sesiones |
| `doido-model` | Active Record | Re-exports de sea-orm, pool de conexiones, helpers de test |
| `doido-view` | Action View | Plantillas Tera, layouts, partials |
| `doido-config` | `config/` de Rails | Config TOML/YAML por capas, credenciales, overrides por env |
| `doido-generators` | CLI + generadores de `rails` | Comandos de runtime y generadores de código |
| `doido-mailer` | Action Mailer | Composición y envío de correos |
| `doido-jobs` | Active Job | Jobs en segundo plano con backends conectables y reintentos |
| `doido-cache` | Active Support Cache | Caché conectable (memory / redis / memcache) |
| `doido-cable` | Action Cable | Canales WebSocket y pub/sub |
| `doido-storage` | Active Storage | Almacenamiento de archivos (disk / S3 / R2 / Azure) |

Para la intención de diseño completa de cada crate, consulta las
[especificaciones](https://github.com/doido-rs/doido/tree/master/docs).
