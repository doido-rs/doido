+++
title = "Installation"
description = "Prerequisites and installing the Doido CLI."
weight = 2
+++

## Prerequisites

- **Rust 1.95.0 or newer.** The workspace pins its toolchain in
  [`rust-toolchain.toml`](https://github.com/doido-rs/doido/blob/master/rust-toolchain.toml);
  installing via [rustup](https://rustup.rs) will pick it up automatically.
- A database driver for your target: SQLite works out of the box; PostgreSQL or
  MySQL require the usual client libraries.

> **Status:** Doido is in early development (`0.0.x`). APIs are not yet stable.

## Install the CLI

While the crates are being published, install the `doido` binary from source:

```bash
git clone https://github.com/doido-rs/doido
cd doido
cargo install --path doido
```

Once released on crates.io, this becomes:

```bash
cargo install doido
```

Verify the install:

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
