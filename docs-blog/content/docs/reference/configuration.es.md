+++
title = "Configuración"
description = "Config YAML por entorno, acceso tipado y overrides por variable de entorno."
weight = 2
+++

> **Especificación de diseño:** [`docs/05-config.md`](https://github.com/doido-rs/doido/blob/master/docs/05-config.md).
> Esta guía documenta el camino **implementado** (YAML por entorno). Ve la nota al final
> para lo que la especificación aplaza.

**Análogo en Rails: `config/`.** Doido lee un único archivo YAML por entorno,
`config/<env>.yml`, y lo expone como ajustes fuertemente tipados. Las variables de entorno
pueden sobrescribir cualquier valor, así que los secretos y ajustes por despliegue quedan
fuera del archivo. El tipo de config vive en `doido-controller`.

## Vistazo general

```rust
use doido::controller::{Config, YamlConfig, ServerConfig};
```

## YAML por entorno

Cada entorno tiene su propio archivo en `config/`, seleccionado por
`Environment::get_env()` (`development`, `test`, `production`). Las secciones de nivel
superior reconocidas son `server`, `logger` y `middleware`; otros subsistemas (cache,
jobs, storage, …) leen sus propias secciones del mismo archivo.

```yaml
# config/development.yml
server:
  bind: 0.0.0.0
  port: 3000
logger:
  level: debug
  format: verbose
  sql: true
middleware:
  cors:
    enabled: true
    allowed_origins: ["*"]
    allowed_methods: ["GET", "POST"]
```

## Acceso tipado

`YamlConfig` deserializa el archivo en structs tipados. Carga el archivo del entorno
actual con `load()` (con fallback a los valores por defecto cuando falta, vía la función
libre `config::load()`), un entorno específico con `load_env()`, o parsea una cadena
directamente con `from_yaml()`. Accede a las secciones mediante el trait `Config`:
`server()`, `logger()`, `middleware()`.

```rust
use doido::controller::{Config, YamlConfig};

// Carga config/<entorno-actual>.yml (p. ej. config/development.yml).
let config = YamlConfig::load()?;

let addr = format!("{}:{}", config.server().bind, config.server().port); // "0.0.0.0:3000"
let level = &config.logger().level;                                      // "debug"
let cors_on = config.middleware().cors.enabled;                          // true

// O nunca falles — fallback a los valores por defecto cuando el archivo falta/es inválido:
let config = doido::controller::config::load(); // Box<dyn Config>
```

`ServerConfig` tiene por defecto `0.0.0.0:3000`; `LoggerConfig` por defecto `info` (ve
[Core](@/docs/reference/core.es.md)); `MiddlewareConfig`/`CorsConfig` están desactivados a
menos que se habiliten.

## Variables de entorno

Cada `config/<env>.yml` se **renderiza con Tera al cargar**. Referencia variables con
`get_env`:

```yaml
database:
  url: '{{ get_env(name="DATABASE_URL", default="sqlite://db/development.db") }}'
```

| Variable | Propósito |
|----------|-----------|
| `DOIDO_ENV` | Selecciona `config/<env>.yml` (`development`, `test`, `production`) |
| `RUST_LOG` | Sobrescribe el nivel del logger |
| `DOIDO_MASTER_KEY` | Descifra `config/credentials.yml.enc` |
| `DOIDO_LOCALE` | Locale por defecto del backend |

Secretos de despliegue (`DATABASE_URL`, SMTP, storage, …) van en YAML con `get_env`.
Desarrollo: `.env` + `startup::prepare()`; producción: inyecta env para que el render
los sustituya. Opcional: `doido config render --env production -o config/production.yml`.

## Configuración de los subsistemas

Las secciones `server`/`logger`/`middleware` están tipadas por `YamlConfig`, pero cada
subsistema conectable lee su propia sección del mismo `config/<env>.yml`. Consulta la guía
correspondiente para las claves exactas:

```yaml
cache:   { type: memory }                 # → guía de Cache
jobs:    { backend: memory, queues: [default] }  # → guía de Jobs
storage: { driver: local }               # → guía de Storage
database: { url: sqlite://db/development.db }     # → guía de Modelos
```

## Especificación vs. implementación

> TOML por capas descartado (US-085). **YAML por entorno** con Tera `get_env`, render
> opcional (`doido config render`) y credenciales AES-256-GCM.

## Véase también

- [Core](@/docs/reference/core.es.md) — la sección `logger` y el `Environment`.
- [Middleware y sesiones](@/docs/reference/middleware.es.md) — la sección `middleware.cors`.
- [Modelos](@/docs/reference/models.es.md) — la sección `database` y el pool de conexiones.
