+++
title = "Instalación"
description = "Requisitos previos e instalación de la CLI de Doido."
weight = 2
+++

## Requisitos previos

- **Rust 1.95.0 o superior.** El workspace fija la toolchain en
  [`rust-toolchain.toml`](https://github.com/doido-rs/doido/blob/master/rust-toolchain.toml);
  al instalar con [rustup](https://rustup.rs) se detecta automáticamente.
- Un driver de base de datos para tu objetivo: SQLite funciona de inmediato;
  PostgreSQL o MySQL requieren las bibliotecas cliente habituales.

> **Estado:** Doido está en desarrollo temprano (`0.0.x`). Las APIs aún no son
> estables.

## Instala la CLI

Mientras los crates se publican, instala el binario `doido` desde el código
fuente:

```bash
git clone https://github.com/doido-rs/doido
cd doido
cargo install --path doido
```

Una vez publicado en crates.io, esto se convierte en:

```bash
cargo install doido
```

Verifica la instalación:

```bash
doido --help
```

## Crea tu primera app

```bash
doido new blog --database=sqlite
cd blog
doido db create && doido db migrate
doido server
```

Continúa en **[Primeros pasos](@/docs/getting-started.es.md)**.
