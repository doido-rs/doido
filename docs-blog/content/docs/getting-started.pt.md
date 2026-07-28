+++
title = "Primeiros passos"
description = "Crie, rode e entenda sua primeira aplicação Doido."
weight = 1
+++

O Doido segue convenções no estilo Rails: um único binário `doido` cria a
aplicação, roda o servidor, gerencia o banco de dados e executa os geradores de
código.

## Crie uma aplicação

```bash
# Cria uma nova aplicação (sqlite por padrão; --database=postgres|mysql)
doido new blog
cd blog

# Configura o banco e roda as migrations pendentes
doido db create
doido db migrate

# Sobe o servidor HTTP em http://0.0.0.0:3000
doido server
```

`GET /` responde com JSON vindo do `HelloController` gerado:

```json
{ "message": "Hello word!" }
```

## Uma amostra do código

Um controller é um bloco `impl` comum anotado com `#[controller]`:

```rust
use doido_controller::controller;
use serde_json::json;

pub struct HelloController;

#[controller]
impl HelloController {
    pub async fn index(ctx: doido_controller::Context) -> doido_controller::Response {
        ctx.json(json!({ "message": "Hello word!" }))
    }
}
```

As rotas são declaradas com a macro `routes!` em `config/routes.rs`:

```rust
use crate::controllers::HelloController;
use doido::controller::{axum, routes};

pub fn router() -> axum::Router {
    routes! {
        get!("/", HelloController::index);
        // resources!(PostsController);   // todas as 7 rotas REST
    }
}
```

## Estrutura do projeto

Uma aplicação gerada segue convenções no estilo Rails:

```
my-app/
├── Cargo.toml
├── src/main.rs              ← delega para doido::run(routes)
├── config/
│   ├── application.toml      ← config base
│   ├── development.yml       ← overrides por ambiente
│   ├── test.yml
│   ├── production.yml
│   ├── routes.rs            ← macro routes!
│   └── inflection.yaml      ← regras de pluralização customizadas
├── app/
│   ├── controllers/
│   ├── models/
│   └── views/
├── db/
│   ├── migration/           ← crate de migrations do SeaORM
│   └── schema/
└── tests/
```

## Configuração

A configuração é em camadas: `config/application.toml` fornece a base, depois
`config/<env>.yml` (development / test / production) faz override por ambiente.
Credenciais criptografadas e variáveis de ambiente `SECTION__KEY` fazem override
por cima.

```yaml
# config/development.yml
server:
  bind: 0.0.0.0
  port: 3000
database:
  url: sqlite://db/development.db
logger:
  level: debug
  format: verbose
cache:
  type: memory
```

## Próximos passos

- **[Instalação](@/docs/installation.pt.md)** — pré-requisitos e como instalar a CLI.
- **[CLI & geradores](@/docs/cli.pt.md)** — todos os comandos de runtime e geradores de código.
- **[Controllers & roteamento](@/docs/guides/controllers.pt.md)** — o guia de requisição/resposta.
