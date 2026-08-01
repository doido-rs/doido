+++
title = "Core"
description = "Inflexiones, el modelo de errores, logging, entorno y utilidades compartidas sobre las que se construyen todos los demás crates."
weight = 1
+++

> **Especificación de diseño:** [`docs/11-core.md`](https://github.com/doido-rs/doido/blob/master/docs/11-core.md).
> Esta guía documenta la API tal como está implementada en `doido-core`.

**Análogo en Rails: Active Support.** `doido-core` es la base de la que dependen todos
los demás crates de Doido — y él no depende de ninguno de ellos. Provee el inflector de
strings, el modelo de errores de la aplicación, la configuración de logging, la detección
de entorno y un puñado de extensiones ergonómicas (helpers de string/tiempo,
notificaciones, un reloj de pruebas). También reexporta los crates del ecosistema que
todo el framework comparte, para que los crates downstream dependan solo de `doido-core`.

## Vistazo general

```rust
use doido::Result;
use doido::core::{Inflector, Environment, init_logger, load_inflections};

// Reexportado para crates downstream (depende de doido-core, no de estos directamente):
use doido::core::{anyhow, async_trait, serde, thiserror, tracing};
```

## Inflexión de strings

`Inflector` es una fachada estática que ofrece todas las transformaciones al estilo Rails.
Impulsa los generadores (nombres de tabla, nombres de clase, helpers de ruta) y está
disponible en cualquier parte de tu app.

```rust
use doido::core::Inflector;

Inflector::pluralize("post");        // "posts"
Inflector::singularize("comments");  // "comment"
Inflector::camelize("blog_post");    // "BlogPost"
Inflector::camelize_lower("blog_post"); // "blogPost"
Inflector::underscore("BlogPost");   // "blog_post"
Inflector::dasherize("blog_post");   // "blog-post"
Inflector::humanize("author_id");    // "Author"
Inflector::tableize("BlogPost");     // "blog_posts"
Inflector::classify("blog_posts");   // "BlogPost"
Inflector::foreign_key("Author");    // "author_id"
Inflector::constantize("blog_post"); // "BlogPost"
```

## Reglas de inflexión personalizadas

Los valores por defecto cubren el inglés común. Sobrescríbelos o extiéndelos para
plurales irregulares, sustantivos incontables y siglas — desde un archivo YAML o de forma
programática. Es el mismo mecanismo que respetan los generadores, así que
`config/inflection.yaml` mantiene los nombres generados consistentes con el lenguaje de
tu dominio.

Carga las reglas desde `config/inflection.yaml` en el arranque (devuelve `Ok(false)`
cuando el archivo no existe, así que es seguro llamarlo incondicionalmente):

```rust
// Carga config/inflection.yaml si existe; Ok(true) cuando se aplica, Ok(false) cuando falta.
doido::core::load_inflections("config/inflection.yaml")?;
```

```yaml
# config/inflection.yaml
irregular:
  - [person, people]
  - [mouse, mice]
uncountable:
  - equipment
  - information
acronym:
  - API
  - HTTP
```

O configura las reglas en código con `init_inflections`, usando el builder `Inflections`
(`irregular`, `uncountable`, `acronym`, `plural`, `singular`):

```rust
doido::core::init_inflections(|i| {
    i.irregular("person", "people");
    i.uncountable("equipment");
    i.acronym("API");
});
```

## El modelo de errores

El código de aplicación usa un único alias `Result<T>` construido sobre `anyhow`; cada
crate define sus propios errores tipados con `thiserror`. `anyhow`, `bail` y el trait
`Context` (`AnyhowContext`) se reexportan, así que `?` y el contexto de error funcionan
sin configuración.

```rust
use doido::Result;
use doido::core::anyhow::Context;

fn load_settings() -> Result<Settings> {
    let raw = std::fs::read_to_string("config/settings.toml")
        .context("settings file is missing")?;      // agrega contexto a cualquier error
    let settings = toml::from_str(&raw)?;            // ? convierte en anyhow::Error
    Ok(settings)
}
```

Para un crate de biblioteca, define un error tipado con el `thiserror` reexportado:

```rust
use doido::core::thiserror;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("key not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

## Logging

Una sola llamada instala el subscriber global de `tracing`. Los niveles, el formato de
salida, las directivas de filtro y la redirección a archivo vienen de la sección `logger`
de `config/<env>.yml` vía `LoggerConfig`; `init_logger()` usa `RUST_LOG` (o valores por
defecto sensatos) y es idempotente.

```rust
// Forma más simple — respeta RUST_LOG, con fallback a valores por defecto sensatos.
doido::core::init_logger();

// O contrólalo desde la config (config/<env>.yml → [logger]):
use doido::core::LoggerConfig;
let cfg = LoggerConfig { level: "debug".into(), ..Default::default() };
doido::core::logger::init_with_config(&cfg);
```

```yaml
# config/development.yml
logger:
  level: debug          # trace | debug | info | warn | error
  format: verbose       # compact (por defecto) | verbose | json_response
  sql: true             # registra cada sentencia SQL
  # file: log/development.log   # redirige la salida (ANSI desactivado)
  # directives: "info,my_app=debug,sqlx=warn"  # override completo del EnvFilter
```

`LogFormat` selecciona el renderizador: `Compact` (una línea, por defecto), `Verbose`
(multilínea con todos los campos y la ubicación en el código) y `JsonResponse` (un objeto
JSON por respuesta HTTP — logs de acceso y métricas de latencia).

## Entorno

`Environment` distingue `Development`, `Test` y `Production`, resuelto a partir del
entorno del proceso.

```rust
use doido::core::Environment;

match Environment::get_env() {
    Environment::Development => { /* errores verbosos, hot reload */ }
    Environment::Test => { /* backends en memoria */ }
    Environment::Production => { /* estricto, cacheado */ }
}

let name = Environment::get_env().as_str(); // "development" | "test" | "production"
```

## Helpers de string y tiempo

`core_ext` agrega traits de extensión ergonómicos (`Blank` para comprobaciones de vacío,
`StringExt` para transformaciones comunes), y `time_ext` ofrece helpers de tiempo
relativo al estilo Rails.

```rust
use doido::core::core_ext::Blank;
use doido::core::time_ext::{days_ago, beginning_of_day};

"".is_blank();                 // true
"  ".is_blank();               // true (solo espacios)
let since = days_ago(7);       // DateTime<Utc> de hace siete días
let start = beginning_of_day(chrono::Utc::now());
```

## Instrumentación y notificaciones

`trace` provee helpers finos y consistentes de eventos estructurados usados por todo el
framework (peticiones, jobs, queries, correo), y `notifications` ofrece un pub/sub ligero
para instrumentación en proceso.

```rust
use doido::core::notifications::Notifications;

let notifications = Notifications::new();
notifications.subscribe("post.created", |payload| {
    tracing::info!(%payload, "a post was created");
});
notifications.instrument("post.created", "{\"id\":1}");
```

## Reloj de pruebas

`TestClock` congela y avanza el tiempo en las pruebas, haciendo determinista la lógica
dependiente del tiempo.

```rust
use doido::core::test_time::TestClock;

let clock = TestClock::new(chrono::Utc::now());
clock.travel(chrono::Duration::hours(2)); // avanza en el tiempo
let later = clock.now();
```

## Véase también

- [Configuración](@/docs/reference/configuration.es.md) — de dónde vienen `logger` y otros ajustes.
- [Modelos](@/docs/reference/models.es.md) — el logging de `sql` fluye hacia los logs de query de sea-orm.
- [Generadores y CLI](@/docs/reference/generators.es.md) — consumidores del inflector.
