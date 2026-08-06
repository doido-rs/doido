+++
title = "Helpers de controlador"
description = "Módulos auxiliares en app/helpers/ — macro #[helper], lógica compartida para controladores y el generador helper."
weight = 5
+++

> **Implementación:** `doido-controller` (trait `Helper` + macro `#[helper]`).
> Esta guía documenta la API tal como está implementada hoy.

**Equivalente Rails: helpers de controlador** (`app/helpers/`). Los helpers son
módulos Rust que concentran **lógica compartida importada por los controladores** —
formateo, transformaciones pequeñas, comprobaciones de autorización que no caben
en una sola acción, y otras utilidades reutilizables.

**No** confundir con los [helpers de vista](@/docs/reference/views.es.md) de
`doido-view` (`link_to`, `form_tag`, …), que construyen HTML para plantillas. Los
helpers de controlador viven en `app/helpers/` y se importan explícitamente en el
código del controlador.

## Resumen

```rust
use doido::controller::helper;
```

## Estructura

Toda app generada incluye un directorio central de helpers:

```
app/
├── controllers/
├── helpers/
│   ├── mod.rs                  ← registro (marcador @generated-helpers)
│   └── application_helper.rs   ← helper por defecto de la app
└── models/
```

`src/main.rs` registra el módulo con `#[path = "../app/helpers/mod.rs"] mod helpers;`
para que los controladores importen helpers como `crate::helpers::…`.

## Definir un helper

Marca una struct con `#[helper]`. La macro implementa la trait `Helper` y añade
`helper_name()` — el nombre snake_case derivado de la struct (`PostsHelper` →
`"posts_helper"`), alineado con el archivo `app/helpers/posts_helper.rs`.

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

## Usar un helper en un controlador

Importa el helper al inicio del archivo del controlador y llama sus funciones
asociadas desde cualquier acción:

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

El `HelloController` generado usa el mismo patrón con `ApplicationHelper`:

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

`GET /` responde con:

```json
{ "message": "Hello, world!" }
```

## Generar un helper

```bash
cargo doido generate helper Posts
```

Esto crea:

| Ruta | Propósito |
|------|-----------|
| `app/helpers/posts_helper.rs` | stub `PostsHelper` con `#[helper]` |
| `app/helpers/mod.rs` | Registra `pub mod posts_helper;` y `pub use posts_helper::PostsHelper;` |
| `tests/posts_helper_test.rs` | smoke test de `helper_name()` |

El generador acepta `Posts` o `PostsHelper` — ambos producen `PostsHelper` en
`posts_helper.rs` (sin sufijo `_helper` duplicado).

```bash
cargo doido generate helper Posts        # → PostsHelper
cargo doido generate helper PostsHelper  # → PostsHelper (sin cambios)
```

## Cuándo usar un helper

| Usa un helper de controlador cuando… | Prefiere otra cosa cuando… |
|--------------------------------------|----------------------------|
| La lógica se reutiliza en varios controladores | La lógica pertenece a un model → ponla en el model |
| Necesitas una función pura sin contexto HTTP | Necesitas session/request → usa filtro o inline en la acción |
| Quieres una unidad con nombre y testeable separada de las acciones | Construyes HTML para plantillas → usa [helpers de vista](@/docs/reference/views.es.md) |

## Pruebas

Los helpers son módulos Rust normales — pruébalos directamente, sin HTTP:

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

Las pruebas de integración también pueden montar un controlador que llama al
helper y verificar la respuesta HTTP, igual que con cualquier otra acción.

## Ver también

- [Controladores y enrutamiento](@/docs/reference/controllers.es.md) — acciones, filtros y `Context`.
- [Views](@/docs/reference/views.es.md) — helpers HTML para plantillas (distintos de los helpers de controlador).
- [Generadores y CLI](@/docs/reference/generators.es.md) — `cargo doido generate helper`.
