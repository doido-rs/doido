+++
title = "Introducing Boot-time Initializers in Doido"
description = "Rails-style config/initializers/ modules, dynamic startup configuration, Doido builder hooks, and the initializer generator."
date = 2026-11-18

[taxonomies]
tags = ["release", "config", "generators", "docs"]
+++

Doido now ships built-in support for **`config/initializers/`** — the framework's analogue to Rails `config/initializers/*`. Every newly scaffolded application includes a dedicated initializers module and boot-time hook to dynamically configure application settings, register custom inflection rules, validate runtime requirements, and initialize third-party services.

## The `config/initializers/` Module

Applications created with `doido new` now include `config/initializers/mod.rs`:

```rust
//! Application initializers run on startup.
//!
//! Rails analogue: `config/initializers/*`.
use doido::controller::initializers::Initializers;
use doido::Result;

// @generated-modules

/// Run all application initializers and set up dynamic configurations on startup.
pub fn init() -> Result<()> {
    registry().run_all()
}

/// Alias for [`init`].
pub fn run() -> Result<()> {
    init()
}

/// Builds and returns the [`Initializers`] registry for the application.
pub fn registry() -> Initializers {
    let mut inits = Initializers::new();
    register_initializers(&mut inits);
    inits
}

fn register_initializers(inits: &mut Initializers) {
    // @generated-initializers
}
```

The application's `src/main.rs` binds `initializers::registry()` into the `Doido` builder:

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

## How It Boots

When `doido server`, `doido worker`, or other CLI commands run:
1. Environment variables and `.env` are prepared (`startup::prepare()`).
2. Per-environment YAML configuration is loaded (`config/<env>.yml`).
3. Tracing subscriber and inflections are initialized.
4. Any registered `initializers` run sequentially in registration order.
5. If any initializer returns an `Err`, boot is stopped immediately with exit code `1`.

## Scaffolding an Initializer

You can generate initializers using the CLI:

```bash
cargo doido generate initializer cors
```

This creates:
- `config/initializers/cors.rs` with a `pub fn init() -> Result<()>` hook.
- Updates `config/initializers/mod.rs` by declaring `pub mod cors;` and registering `inits.register("cors", cors::init);`.
- `tests/cors_initializer_test.rs` with an automated unit test.

To undo the generator:

```bash
cargo doido destroy initializer cors
```

This removes `config/initializers/cors.rs` and the test while preserving `config/initializers/mod.rs`.

## Example: Custom Inflections and Client Setup

An initializer can configure custom rules or initialize clients dynamically:

```rust
// config/initializers/custom_rules.rs
use doido::Result;

pub fn init() -> Result<()> {
    // Dynamically register acronyms or pluralization rules at boot
    doido::core::inflector::add_acronym("API");
    doido::core::inflector::add_acronym("UUID");
    doido_core::tracing::info!("initialized custom domain acronyms");
    Ok(())
}
```

## Read the Full Guide

For complete details on configuration, boot sequences, and encrypted credentials, see the [Configuration reference](/docs/reference/configuration/) and [Generators reference](/docs/reference/generators/).
