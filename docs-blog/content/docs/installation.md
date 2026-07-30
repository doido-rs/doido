+++
title = "Installation"
description = "Prerequisites and installing the Doido CLI."
weight = 2
+++

## Prerequisites

- **Rust 1.95.0 or newer** — only if you build apps from source or compile the CLI
  yourself. Pre-built binaries from GitHub Releases do **not** require Rust.
  The workspace pins its toolchain in
  [`rust-toolchain.toml`](https://github.com/doido-rs/doido/blob/master/rust-toolchain.toml);
  installing via [rustup](https://rustup.rs) will pick it up automatically.
- A database driver for your target: SQLite works out of the box; PostgreSQL or
  MySQL require the usual client libraries.

> **Status:** Doido is in early development (`0.0.x`). APIs are not yet stable.

## Install the CLI

### Linux and macOS (recommended)

Download and run the release installer with `curl`. It detects your OS/architecture,
fetches the matching binary from GitHub Releases, installs it to `~/.local/bin`, and
prints a PATH hint when needed:

```bash
curl -fsSL https://github.com/doido-rs/doido/releases/latest/download/install.sh | bash
```

Pin a specific version:

```bash
curl -fsSL https://github.com/doido-rs/doido/releases/download/v0.0.9/install.sh | DOIDO_VERSION=0.0.9 bash
```

Custom install directory:

```bash
curl -fsSL https://github.com/doido-rs/doido/releases/latest/download/install.sh | DOIDO_INSTALL_DIR=$HOME/bin bash
```

### Windows

In PowerShell:

```powershell
irm https://github.com/doido-rs/doido/releases/latest/download/install.ps1 | iex
```

Pin a version:

```powershell
$env:DOIDO_VERSION = "0.0.9"
irm https://github.com/doido-rs/doido/releases/download/v0.0.9/install.ps1 | iex
```

The script installs `doido.exe` to `%USERPROFILE%\.local\bin` and adds that folder to
your user `PATH` when it is missing.

### From crates.io

Once published on crates.io:

```bash
cargo install doido
```

### From source

While developing the framework itself:

```bash
git clone https://github.com/doido-rs/doido
cd doido
cargo install --path doido
```

## Verify the install

```bash
doido --help
```

## Create your first app

```bash
doido new blog --database=sqlite
cd blog
doido db create && doido db migrate
doido server
```

Continue with **[Getting started](@/docs/getting-started.md)**.
