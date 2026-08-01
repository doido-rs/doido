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
use doido::controller::env_override::{apply_env_overrides, from_process_env};
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

## Overrides por variable de entorno

Cualquier ajuste se puede sobrescribir con una variable de entorno llamada `SECTION__KEY`
(doble guion bajo): `SERVER__PORT=4000` establece `server.port`. Los valores se convierten
a bool o número cuando parsean, si no se mantienen como cadenas — y se crea una nueva
sección si aún no existe.

```bash
# Override en el arranque — ideal para secretos y valores por despliegue:
SERVER__PORT=4000 LOGGER__LEVEL=warn DATABASE__URL=postgres://... doido server
```

Los overrides se aplican al valor de config ya parseado, antes de la deserialización
tipada:

```rust
use doido::controller::env_override::{apply_env_overrides, from_process_env};

let mut value: serde_json::Value = serde_json::to_value(&raw_config)?;

// Toma las variables SECTION__KEY directamente del entorno del proceso:
from_process_env(&mut value);

// …o aplica un conjunto explícito (útil en pruebas):
apply_env_overrides(&mut value, &[
    ("SERVER__PORT".into(), "4000".into()),
    ("LOGGER__LEVEL".into(), "debug".into()),
]);
```

## Configuración de los subsistemas

Las secciones `server`/`logger`/`middleware` están tipadas por `YamlConfig`, pero cada
subsistema conectable lee su propia sección del mismo `config/<env>.yml`. Consulta la guía
correspondiente para las claves exactas:

```yaml
cache:   { type: memory }                 # → guía de Cache
jobs:    { backend: memory, queues: [default] }  # → guía de Jobs
storage: { service: local }               # → guía de Storage
database: { url: sqlite://db/development.db }     # → guía de Modelos
```

## Especificación vs. implementación

> La especificación describe **TOML por capas** (`config/doido.toml` + overrides por
> entorno) y **credenciales cifradas con AES-256-GCM** (`config/credentials.toml.enc` +
> `master.key`). Eso está **aplazado**. El camino implementado y probado es **YAML por
> entorno** más overrides `SECTION__KEY`, documentado arriba.

## Véase también

- [Core](@/docs/reference/core.es.md) — la sección `logger` y el `Environment`.
- [Middleware y sesiones](@/docs/reference/middleware.es.md) — la sección `middleware.cors`.
- [Modelos](@/docs/reference/models.es.md) — la sección `database` y el pool de conexiones.
