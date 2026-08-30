+++
title = "Primeiros passos"
description = "Crie, rode e entenda sua primeira aplicação Doido."
weight = 1
+++

O Doido segue convenções no estilo Rails: o CLI global `doido` cria novas
aplicações; dentro do projeto, `cargo doido` roda o servidor, gerencia o banco
de dados e executa os geradores de código.

## Crie uma aplicação

```bash
# Cria uma nova aplicação (sqlite por padrão; --database=postgres|mysql)
doido new blog
cd blog

# Ou crie com autenticação (sessões + registro no estilo Devise)
doido new blog --database=sqlite --auth
cd blog

# Configura o banco e roda as migrations pendentes
cargo doido db create
cargo doido db migrate

# Opcional: insere dados de fixture (edite db/seeds.rs antes)
# cargo doido db seed

# Sobe o servidor HTTP em http://0.0.0.0:3000
cargo doido server
```

Com `--auth`, a app inclui `doido-auth`, model User, rotas de sign-in/sign-up e
controllers de auth. Veja a [referência de Auth](/pt/docs/reference/auth/) para JWT,
OAuth, 2FA e extractors.

`GET /` responde com JSON vindo do `HelloController` gerado, que delega ao
`ApplicationHelper`:

```json
{ "message": "Hello, world!" }
```

## Uma amostra do código

Um controller é um bloco `impl` comum anotado com `#[controller]`. A action hello
gerada importa um helper de `app/helpers/`:

```rust
use crate::helpers::ApplicationHelper;
use doido::controller::{controller, Context, Response};
use serde_json::json;

pub struct HelloController;

#[controller]
impl HelloController {
    pub async fn index(ctx: Context) -> Response {
        ctx.json(json!({
            "message": ApplicationHelper::greet("world")
        }))
    }
}
```

Helpers são structs anotadas com `#[helper]` — funções compartilhadas que os
controllers importam:

```rust
use doido::controller::helper;

#[helper]
pub struct ApplicationHelper;

impl ApplicationHelper {
    pub fn greet(name: &str) -> String {
        format!("Hello, {name}!")
    }
}
```

Gere mais helpers com `cargo doido generate helper Posts`. Veja
[Helpers de controller](@/docs/reference/helpers.pt.md) para o guia completo.

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
│   ├── helpers/             ← helpers de controller (#[helper])
│   ├── models/
│   └── views/
├── db/
│   ├── migration/           ← crate de migrations do SeaORM (linkado no binário da app)
│   ├── seeds.rs             ← seeder no binário (usa app/models)
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

- **[Instalação](@/docs/setup/installation.pt.md)** — pré-requisitos e como instalar a CLI.
- **[CLI & geradores](@/docs/reference/cli.pt.md)** — todos os comandos de runtime e geradores de código.
- **[Controllers & roteamento](@/docs/reference/controllers.pt.md)** — o guia de requisição/resposta.
- **[Helpers de controller](@/docs/reference/helpers.pt.md)** — lógica compartilhada em `app/helpers/`.
