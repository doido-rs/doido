+++
title = "Helpers de controller"
description = "Módulos auxiliares em app/helpers/ — macro #[helper], lógica compartilhada para controllers e o gerador helper."
weight = 5
+++

> **Implementação:** `doido-controller` (trait `Helper` + macro `#[helper]`).
> Este guia documenta a API como implementada hoje.

**Equivalente Rails: helpers de controller** (`app/helpers/`). Helpers são módulos
Rust comuns que concentram **lógica compartilhada importada pelos controllers** —
formatação, transformações pequenas, checagens de autorização que não cabem em
uma action só, e outras utilidades reutilizáveis.

**Não** confunda com os [helpers de view](@/docs/reference/views.pt.md) do
`doido-view` (`link_to`, `form_tag`, …), que montam HTML para templates. Helpers
de controller ficam em `app/helpers/` e são importados explicitamente no código
do controller.

## Resumo

```rust
use doido::controller::helper;
```

## Estrutura

Toda app gerada inclui um diretório central de helpers:

```
app/
├── controllers/
├── helpers/
│   ├── mod.rs                  ← registro (marcador @generated-helpers)
│   └── application_helper.rs   ← helper padrão da aplicação
└── models/
```

O `src/main.rs` registra o módulo com `#[path = "../app/helpers/mod.rs"] mod helpers;`
para que os controllers importem helpers como `crate::helpers::…`.

## Definindo um helper

Marque uma struct com `#[helper]`. A macro implementa a trait `Helper` e adiciona
`helper_name()` — o nome snake_case derivado da struct (`PostsHelper` →
`"posts_helper"`), alinhado ao arquivo `app/helpers/posts_helper.rs`.

```rust
use doido::controller::helper;

#[helper]
pub struct PostsHelper;

impl PostsHelper {
    pub fn format_title(title: &str) -> String {
        title.trim().to_uppercase()
    }

    pub fn excerpt(body: &str, max_len: usize) -> String {
        if body.len() <= max_len {
            body.to_string()
        } else {
            format!("{}…", &body[..max_len])
        }
    }
}
```

## Usando um helper no controller

Importe o helper no topo do arquivo do controller e chame suas funções associadas
em qualquer action:

```rust
use crate::helpers::PostsHelper;
use doido::controller::{controller, Context, Response};
use serde_json::json;

pub struct PostsController;

#[controller]
impl PostsController {
    async fn index(ctx: Context) -> Response {
        let title = PostsHelper::format_title("hello");
        ctx.json(json!({ "title": title }))
    }
}
```

O `HelloController` gerado usa o mesmo padrão com `ApplicationHelper`:

```rust
use crate::helpers::ApplicationHelper;

#[controller]
impl HelloController {
    pub async fn index(ctx: Context) -> Response {
        ctx.json(json!({
            "message": ApplicationHelper::greet("world")
        }))
    }
}
```

`GET /` responde com:

```json
{ "message": "Hello, world!" }
```

## Gerando um helper

```bash
cargo doido generate helper Posts
```

Isso cria:

| Caminho | Propósito |
|---------|-----------|
| `app/helpers/posts_helper.rs` | stub `PostsHelper` com `#[helper]` |
| `app/helpers/mod.rs` | Registra `pub mod posts_helper;` e `pub use posts_helper::PostsHelper;` |
| `tests/posts_helper_test.rs` | smoke test de `helper_name()` |

O gerador aceita `Posts` ou `PostsHelper` — ambos produzem `PostsHelper` em
`posts_helper.rs` (sem sufixo `_helper` duplicado).

```bash
cargo doido generate helper Posts        # → PostsHelper
cargo doido generate helper PostsHelper  # → PostsHelper (inalterado)
```

## Quando usar um helper

| Use um helper de controller quando… | Prefira outra coisa quando… |
|-------------------------------------|----------------------------|
| A lógica é reutilizada em vários controllers | A lógica pertence a um model → coloque no model |
| Você precisa de função pura sem contexto HTTP | Precisa de session/request → use filtro ou inline na action |
| Quer unidade nomeada e testável separada das actions | Está montando HTML para templates → use [helpers de view](@/docs/reference/views.pt.md) |

## Testes

Helpers são módulos Rust comuns — teste diretamente, sem HTTP:

```rust
use crate::helpers::PostsHelper;

#[test]
fn format_title_uppercases_and_trims() {
    assert_eq!(PostsHelper::format_title("  hi  "), "HI");
}

#[test]
fn helper_name_matches_file_convention() {
    assert_eq!(PostsHelper::helper_name(), "posts_helper");
}
```

Testes de integração também podem montar um controller que chama o helper e
verificar a resposta HTTP, como em qualquer outra action.

## Veja também

- [Controllers & roteamento](@/docs/reference/controllers.pt.md) — actions, filtros e `Context`.
- [Views](@/docs/reference/views.pt.md) — helpers HTML para templates (distintos dos helpers de controller).
- [Geradores & CLI](@/docs/reference/generators.pt.md) — `cargo doido generate helper`.
