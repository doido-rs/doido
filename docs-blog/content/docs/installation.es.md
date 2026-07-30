+++
title = "Instalación"
description = "Requisitos previos e instalación de la CLI de Doido."
weight = 2
+++

## Requisitos previos

- **Rust 1.95.0 o superior** — solo si compilas apps o la CLI desde el código
  fuente. Los binarios precompilados de GitHub Releases **no** requieren Rust.
  El workspace fija la toolchain en
  [`rust-toolchain.toml`](https://github.com/doido-rs/doido/blob/master/rust-toolchain.toml);
  al instalar con [rustup](https://rustup.rs) se detecta automáticamente.
- Un driver de base de datos para tu objetivo: SQLite funciona de inmediato;
  PostgreSQL o MySQL requieren las bibliotecas cliente habituales.

> **Estado:** Doido está en desarrollo temprano (`0.0.x`). Las APIs aún no son
> estables.

## Instala la CLI

### Linux y macOS (recomendado)

Descarga y ejecuta el instalador de release con `curl`. Detecta SO/arquitectura,
obtiene el binario correspondiente de GitHub Releases, lo instala en `~/.local/bin`
y muestra una pista de PATH cuando hace falta:

```bash
curl -fsSL https://github.com/doido-rs/doido/releases/latest/download/install.sh | bash
```

Fijar una versión:

```bash
curl -fsSL https://github.com/doido-rs/doido/releases/download/v0.0.9/install.sh | DOIDO_VERSION=0.0.9 bash
```

Directorio de instalación personalizado:

```bash
curl -fsSL https://github.com/doido-rs/doido/releases/latest/download/install.sh | DOIDO_INSTALL_DIR=$HOME/bin bash
```

### Windows

En PowerShell:

```powershell
irm https://github.com/doido-rs/doido/releases/latest/download/install.ps1 | iex
```

Fijar una versión:

```powershell
$env:DOIDO_VERSION = "0.0.9"
irm https://github.com/doido-rs/doido/releases/download/v0.0.9/install.ps1 | iex
```

El script instala `doido.exe` en `%USERPROFILE%\.local\bin` y añade esa carpeta al
`PATH` del usuario si aún no está.

### Desde crates.io

Una vez publicado en crates.io:

```bash
cargo install doido
```

### Desde el código fuente

Mientras desarrollas el framework:

```bash
git clone https://github.com/doido-rs/doido
cd doido
cargo install --path doido
```

## Verifica la instalación

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
