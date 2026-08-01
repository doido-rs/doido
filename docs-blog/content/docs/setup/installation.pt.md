+++
title = "Instalação"
description = "Pré-requisitos e como instalar a CLI do Doido."
weight = 2
+++

## Pré-requisitos

- **Rust 1.95.0 ou mais novo** — apenas se você compila apps ou a CLI a partir do
  código-fonte. Binários pré-compilados do GitHub Releases **não** exigem Rust.
  O workspace fixa a toolchain em
  [`rust-toolchain.toml`](https://github.com/doido-rs/doido/blob/master/rust-toolchain.toml);
  instalando via [rustup](https://rustup.rs) ela é detectada automaticamente.
- Um driver de banco para o seu alvo: SQLite funciona de imediato; PostgreSQL ou
  MySQL exigem as bibliotecas cliente usuais.

> **Status:** o Doido está em desenvolvimento inicial (`0.0.x`). As APIs ainda não
> são estáveis.

## Instale a CLI

### Linux e macOS (recomendado)

Baixe e execute o instalador de release com `curl`. Ele detecta SO/arquitetura,
busca o binário correspondente no GitHub Releases, instala em `~/.local/bin` e
mostra uma dica de PATH quando necessário:

```bash
curl -fsSL https://github.com/doido-rs/doido/releases/latest/download/install.sh | bash
```

Fixar uma versão:

```bash
curl -fsSL https://github.com/doido-rs/doido/releases/download/v0.0.9/install.sh | DOIDO_VERSION=0.0.9 bash
```

Diretório de instalação personalizado:

```bash
curl -fsSL https://github.com/doido-rs/doido/releases/latest/download/install.sh | DOIDO_INSTALL_DIR=$HOME/bin bash
```

### Windows

No PowerShell:

```powershell
irm https://github.com/doido-rs/doido/releases/latest/download/install.ps1 | iex
```

Fixar uma versão:

```powershell
$env:DOIDO_VERSION = "0.0.9"
irm https://github.com/doido-rs/doido/releases/download/v0.0.9/install.ps1 | iex
```

O script instala `doido.exe` em `%USERPROFILE%\.local\bin` e adiciona essa pasta ao
`PATH` do usuário quando ela ainda não estiver lá.

### Via crates.io

Assim que publicado no crates.io:

```bash
cargo install doido
```

### A partir do código-fonte

Durante o desenvolvimento do framework:

```bash
git clone https://github.com/doido-rs/doido
cd doido
cargo install --path doido
```

## Verifique a instalação

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

Continue em **[Primeiros passos](@/docs/tutorials/getting-started.pt.md)**.
