+++
title = "Geradores & CLI"
description = "O binário doido: comandos de runtime, geradores de código, a DSL de campos, injeção automática de rotas e geradores customizados."
weight = 7
+++

> **Especificação de design:** [`docs/06-cli.md`](https://github.com/doido-rs/doido/blob/master/docs/06-cli.md)
> e [`docs/06b-generators.md`](https://github.com/doido-rs/doido/blob/master/docs/06b-generators.md).
> Este guia documenta a API como implementada em `doido-generators`. Para uma tabela rápida
> de comandos, veja [CLI & geradores](@/docs/reference/cli.pt.md).

**Análogo no Rails: o binário `rails` + geradores.** `doido-generators` alimenta
`doido new` e o alias local `cargo doido` — comandos de runtime (`server`, `db`,
`worker`, …) e geradores de código (`generate scaffold`, `generate model`, …).
Uma app gerada inicializa chamando `doido::generators::run(Some(routes))`.

## Visão geral

```rust
// src/main.rs de uma app gerada
#[tokio::main]
async fn main() {
    doido::generators::run(Some(config::routes::router())).await;
}
```

## Comandos de runtime

| Comando | Descrição |
|---------|-----------|
| `cargo doido server` | Inicia o servidor HTTP axum |
| `cargo doido routes` | Imprime a tabela de rotas |
| `cargo doido console` | Console interativo com o contexto da app |
| `cargo doido db <cmd>` | `migrate`, `rollback`, `reset`, `status`, `seed` |
| `cargo doido worker [--once]` | Roda o worker de jobs em background |
| `cargo doido jobs <cmd>` | Inspeciona/retenta/descarta jobs em background |
| `cargo doido credentials <cmd>` | Gerencia credenciais |
| `cargo doido generate <name> …` | Roda um gerador de código |
| `cargo doido destroy <name> …` | Reverte um gerador |
| `doido new <app>` | Cria uma nova aplicação |

```bash
cargo doido db migrate          # roda as migrations pendentes
cargo doido worker --once       # drena a fila e sai
cargo doido routes              # imprime todas as rotas registradas
```

## Criando uma aplicação

`doido new` cria um projeto no estilo Rails; escolha o driver de banco com `--database`.

```bash
doido new blog --database=sqlite   # ou postgres | mysql
cd blog
cargo doido db create && cargo doido db migrate
cargo doido server
```

O `doido new` também cria `db/seed/` — um crate de workspace que roda com
`cargo doido db seed` para inserir fixtures via `app/models/`. Edite
`db/seed/src/main.rs` depois de gerar models.

```bash
cargo doido generate model Post title:string body:text
cargo doido db seed
```

## Geradores de código

Rode `cargo doido generate` sem argumentos para listar todos os geradores registrados. Cada um
escreve arquivos (e alguns injetam rotas). As subseções abaixo trazem um exemplo executável para
cada um; os campos `name:type` recebidos por `model`, `scaffold` e `resource` estão documentados
em [A DSL de campos](#a-dsl-de-campos).

| Gerador | Gera |
|---------|------|
| `model` | `app/models/<name>.rs` + migration |
| `migration` | uma migration isolada |
| `controller` | um `#[controller]` com stubs de action (+ rota) |
| `helper` | um helper de controller em `app/helpers/` |
| `scaffold` | model + migration + controller + views + rota |
| `resource` | model + migration + controller + rota (sem views) |
| `mailer` | um mailer + templates |
| `job` | um job em background |
| `channel` | um canal WebSocket |
| `templates` | templates de view para um controller existente |
| `locale` | um arquivo de locale |
| `generator` | o esqueleto de um novo gerador customizado |
| `storage:install` | tabelas de storage + config |
| `storage:adapter` | o esqueleto de um adapter de storage customizado |

### model

Cria `app/models/<name>.rs` e a migration correspondente.

```bash
cargo doido generate model Post title:string body:text
```

### migration

Uma migration isolada; adicione ou remova colunas com a DSL de campos.

```bash
cargo doido generate migration add_published_to_posts published:boolean
```

### controller

Um `#[controller]` com um stub de action por nome, mais sua rota.

```bash
cargo doido generate controller Pages home about
```

### helper

Um módulo de helper de controller em `app/helpers/`.

```bash
cargo doido generate helper Posts
```

### scaffold

A stack completa de CRUD — model, migration, controller, views e rota — em um comando.

```bash
cargo doido generate scaffold Post title:string:not_null body:text author:references
```

### resource

Como o `scaffold`, mas sem views — a stack para modo API.

```bash
cargo doido generate resource Post title:string body:text
cargo doido generate resource Post title:string --api   # só JSON
```

### mailer

Um mailer mais um template por action.

```bash
cargo doido generate mailer User welcome
```

### job

Um job em background em `app/jobs/`.

```bash
cargo doido generate job SendNewsletter
```

### channel

Um canal WebSocket em `app/channels/`.

```bash
cargo doido generate channel Chat
```

### templates

Ejeta os templates de view embutidos de um controller existente para você customizá-los.

```bash
cargo doido generate templates Posts
```

### locale

Um arquivo de locale i18n inicial (padrão `en`).

```bash
cargo doido generate locale pt
```

### generator

Cria o esqueleto de um novo gerador customizado — veja [Geradores customizados](#geradores-customizados).

```bash
cargo doido generate generator policy
```

### storage:install

Tabelas de storage mais config — veja [Storage](@/docs/reference/storage.pt.md).

```bash
cargo doido generate storage:install
```

### storage:adapter

O esqueleto de um adapter de storage customizado.

```bash
cargo doido generate storage:adapter Cloudinary
```

### Geradores de auth

Quando `doido-auth` está no `Cargo.toml`, três geradores adicionais aparecem em
**Auth (doido-auth)**:

| Gerador | Gera |
|---------|------|
| `auth:install` | Migration + model User, controllers de auth, views, config, rotas |
| `auth:controller` | Controller com `CurrentUser` / guards de auth |
| `auth:scaffold` | Scaffold com auth e ownership por `user_id` |

O caminho mais rápido é `doido new blog --database=sqlite --auth`, que adiciona `doido-auth`
e roda o `auth:install` para você.

### auth:install

Migration + model User, controllers de auth, views, config e rotas.

```bash
cargo doido generate auth:install          # auth HTML com cookie/sessão
cargo doido generate auth:install --api    # endpoints de auth só JSON
```

### auth:controller

Um controller já com guards `CurrentUser` / `require_user`.

```bash
cargo doido generate auth:controller Dashboard
```

### auth:scaffold

Um scaffold com auth e ownership por `user_id`.

```bash
cargo doido generate auth:scaffold Post title:string body:text
```

## A DSL de campos

Os geradores de model, scaffold e resource recebem campos como `name:type[:modifier…]`. Os
tipos mapeiam para colunas de migration; os modificadores adicionam constraints e índices.

```bash
cargo doido generate model Post \
  title:string:not_null \
  slug:string:unique \
  body:text \
  author:references \
  views:integer:index
```

## Injeção automática de rotas

Geradores que produzem um controller (`scaffold`, `resource`, `controller`) fazem o parse
de `config/routes.rs`, inserem a rota correspondente (ex.: `resources!(posts,
PostsController);`) dentro do bloco `routes! { … }` e pulam controllers já registrados — de
modo que um resource gerado fica acessível sem editar as rotas manualmente.

## Revertendo um gerador

`cargo doido destroy` remove o que o `generate` correspondente criou.

```bash
cargo doido generate scaffold Post title:string
cargo doido destroy  scaffold Post           # remove os arquivos gerados (e a rota)
```

## Geradores customizados

O sistema de geradores é um registro extensível. Implemente o trait `Generator` (retornando
`GeneratedFile`s) e registre-o; `cargo doido generate generator <name>` cria um esqueleto para
você.

```rust
use doido::generators::{Generator, GeneratedFile};
use doido::Result;

struct PolicyGenerator;

impl Generator for PolicyGenerator {
    fn name(&self) -> &str { "policy" }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().unwrap_or("application");
        Ok(vec![GeneratedFile {
            path: format!("app/policies/{name}_policy.rs"),
            content: format!("// {name} policy\n"),
        }])
    }
}

// Registre-o, depois rode/liste pelo registro:
let mut registry = doido::generators::default_registry();
registry.register(Box::new(PolicyGenerator));
let files = registry.run("policy", &["post"])?;
let names = registry.list(); // inclui "policy"
```

## Veja também

- [Models](@/docs/reference/models.pt.md) — o que `generate model`/`migration` produzem.
- [Controllers & roteamento](@/docs/reference/controllers.pt.md) — o bloco `routes!` que os geradores editam.
- [Helpers de controller](@/docs/reference/helpers.pt.md) — o que `generate helper` produz.
- [Jobs](@/docs/reference/jobs.pt.md), [Mailer](@/docs/reference/mailer.pt.md), [Cable](@/docs/reference/cable.pt.md) — seus geradores.
- [Auth](@/docs/reference/auth.pt.md) — `auth:install`, estratégias e extractors.
