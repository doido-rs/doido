+++
title = "Apresentando Initializers de Boot no Doido"
description = "Módulos config/initializers/ no estilo Rails, configuração dinâmica na inicialização, hooks no builder Doido e o gerador de initializers."
date = 2026-11-18

[taxonomies]
tags = ["release", "config", "generators", "docs"]
+++

O Doido agora inclui suporte nativo a **`config/initializers/`** — o análogo no framework ao `config/initializers/*` do Rails. Toda aplicação recém-criada inclui um módulo dedicado de initializers e hook de boot para configurar dinamicamente a aplicação, registrar regras customizadas de inflexão, validar requisitos em tempo de execução e inicializar serviços externos.

## O Módulo `config/initializers/`

Aplicações criadas com `doido new` agora incluem `config/initializers/mod.rs`:

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

O `src/main.rs` da aplicação conecta `initializers::registry()` no builder `Doido`:

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

## Como Funciona o Boot

Quando `doido server`, `doido worker` ou outros comandos da CLI executam:
1. Variáveis de ambiente e `.env` são preparados (`startup::prepare()`).
2. A configuração YAML por ambiente é carregada (`config/<env>.yml`).
3. O logger (tracing) e inflexões são inicializados.
4. Todos os `initializers` registrados rodam sequencialmente na ordem em que foram registrados.
5. Se qualquer initializer retornar um `Err`, a inicialização é interrompida imediatamente com exit code `1`.

## Gerando um Initializer

Você pode gerar initializers usando a CLI:

```bash
cargo doido generate initializer cors
```

Isso cria:
- `config/initializers/cors.rs` com a função `pub fn init() -> Result<()>`.
- Atualiza `config/initializers/mod.rs` declarando `pub mod cors;` e registrando `inits.register("cors", cors::init);`.
- `tests/cors_initializer_test.rs` com um teste unitário automatizado.

Para desfazer a geração:

```bash
cargo doido destroy initializer cors
```

Isso remove `config/initializers/cors.rs` e o teste correspondente, preservando `config/initializers/mod.rs`.

## Exemplo: Inflexões Customizadas e Clientes Externos

Um initializer pode registrar regras ou configurar clientes dinamicamente:

```rust
// config/initializers/regras_customizadas.rs
use doido::Result;

pub fn init() -> Result<()> {
    // Registra acrônimos ou regras de pluralização dinamicamente no boot
    doido::core::inflector::add_acronym("API");
    doido::core::inflector::add_acronym("UUID");
    doido_core::tracing::info!("acrônimos de domínio inicializados");
    Ok(())
}
```

## Leia o Guia Completo

Para detalhes completos sobre configuração, sequência de boot e credenciais criptografadas, veja a [referência de Configuração](/pt/docs/reference/configuration/) e a [referência de Geradores](/pt/docs/reference/generators/).
