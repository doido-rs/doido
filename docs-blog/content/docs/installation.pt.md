+++
title = "Instalação"
description = "Pré-requisitos e como instalar a CLI do Doido."
weight = 2
+++

## Pré-requisitos

- **Rust 1.95.0 ou mais novo.** O workspace fixa a toolchain em
  [`rust-toolchain.toml`](https://github.com/doido-rs/doido/blob/master/rust-toolchain.toml);
  instalando via [rustup](https://rustup.rs) ela é detectada automaticamente.
- Um driver de banco para o seu alvo: SQLite funciona de imediato; PostgreSQL ou
  MySQL exigem as bibliotecas cliente usuais.

> **Status:** o Doido está em desenvolvimento inicial (`0.0.x`). As APIs ainda não
> são estáveis.

## Instale a CLI

Enquanto os crates estão sendo publicados, instale o binário `doido` a partir do
código-fonte:

```bash
git clone https://github.com/doido-rs/doido
cd doido
cargo install --path doido
```

Assim que publicado no crates.io, isto vira:

```bash
cargo install doido
```

Verifique a instalação:

```bash
doido --help
```

## Crie sua primeira app

```bash
doido new blog --database=sqlite
cd blog
doido db create && doido db migrate
doido server
```

Continue em **[Primeiros passos](@/docs/getting-started.pt.md)**.
