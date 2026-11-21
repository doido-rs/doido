+++
title = "Presentamos Initializers de Boot en Doido"
description = "Módulos config/initializers/ al estilo Rails, configuración dinámica al inicio, hooks en el builder Doido y el generador de initializers."
date = 2026-11-18

[taxonomies]
tags = ["release", "config", "generators", "docs"]
+++

Doido ahora incluye soporte nativo para **`config/initializers/`** — el análogo en el framework a `config/initializers/*` de Rails. Cada aplicación recién creada incluye un módulo dedicado de initializers y un hook de inicio para configurar dinámicamente la aplicación, registrar reglas personalizadas de inflexión, validar requisitos en tiempo de ejecución e inicializar servicios externos.

## El Módulo `config/initializers/`

Las aplicaciones creadas con `doido new` ahora incluyen `config/initializers/mod.rs`:

```rust
//! Application initializers run on startup.
//!
//! Rails analogue: `config/initializers/*`.
use doido::controller::initializers::Initializers;
use doido::Result;

// @generated-modules

/// Run all application initializers and set up dynamic configurations on startup.
#[allow(dead_code)]
pub fn init() -> Result<()> {
    registry().run_all()
}

/// Alias for [`init`].
#[allow(dead_code)]
pub fn run() -> Result<()> {
    init()
}

/// Builds and returns the [`Initializers`] registry for the application.
pub fn registry() -> Initializers {
    let mut inits = Initializers::new();
    register_initializers(&mut inits);
    inits
}

#[allow(unused_variables)]
fn register_initializers(inits: &mut Initializers) {
    // @generated-initializers
}
```

El archivo `src/main.rs` de la aplicación conecta `initializers::registry()` al builder `Doido`:

```rust
#[path = "../config/initializers/mod.rs"]
mod initializers;

#[tokio::main]
async fn main() {
    startup::prepare();
    doido::Doido::new()
        .router(routes::router())
        .initializers(initializers::registry())
        .migrator::<migration::Migrator>()
        .seeder(seed::run)
        .run()
        .await;
}
```

## Cómo Funciona el Boot

Cuando se ejecuta `doido server`, `doido worker` u otros comandos de la CLI:
1. Se preparan las variables de entorno y `.env` (`startup::prepare()`).
2. Se carga la configuración YAML por entorno (`config/<env>.yml`).
3. Se inicializa el logger (tracing) y las inflexiones.
4. Todos los `initializers` registrados se ejecutan secuencialmente en el orden de registro.
5. Si algún initializer devuelve un `Err`, el inicio se detiene inmediatamente con exit code `1`.

## Generar un Initializer

Puedes generar initializers usando la CLI:

```bash
cargo doido generate initializer cors
```

Esto crea:
- `config/initializers/cors.rs` con la función `pub fn init() -> Result<()>`.
- Actualiza `config/initializers/mod.rs` declarando `pub mod cors;` y registrando `inits.register("cors", cors::init);`.
- `tests/cors_initializer_test.rs` con una prueba unitaria automatizada.

Para deshacer el generador:

```bash
cargo doido destroy initializer cors
```

Esto elimina `config/initializers/cors.rs` y la prueba correspondiente, preservando `config/initializers/mod.rs`.

## Ejemplo: Inflexiones Personalizadas y Clientes Externos

Un initializer puede registrar reglas o configurar clientes dinámicamente:

```rust
// config/initializers/reglas_personalizadas.rs
use doido::Result;

pub fn init() -> Result<()> {
    // Registra siglas o reglas de pluralización dinámicamente en el boot
    doido::core::inflector::add_acronym("API");
    doido::core::inflector::add_acronym("UUID");
    doido_core::tracing::info!("siglas de dominio inicializadas");
    Ok(())
}
```

## Lee la Guía Completa

Para conocer todos los detalles sobre configuración, secuencias de boot y credenciales cifradas, consulta la [referencia de Configuración](/es/docs/reference/configuration/) y la [referencia de Generadores](/es/docs/reference/generators/).
