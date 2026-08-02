+++
title = "Generators & CLI"
description = "The doido binary: runtime commands, code generators, the field DSL, route auto-injection, and custom generators."
weight = 7
aliases = ['/docs/guides/generators/']

+++

> **Design spec:** [`docs/06-cli.md`](https://github.com/doido-rs/doido/blob/master/docs/06-cli.md)
> and [`docs/06b-generators.md`](https://github.com/doido-rs/doido/blob/master/docs/06b-generators.md).
> This guide documents the API as implemented in `doido-generators`. For a quick command
> table see [CLI & generators](@/docs/reference/cli.md).

**Rails analogue: the `rails` binary + generators.** `doido-generators` powers
`doido new` and the project-local `cargo doido` alias — runtime commands
(`server`, `db`, `worker`, …) and code generators (`generate scaffold`,
`generate model`, …). A generated app boots by calling
`doido::generators::run(Some(routes))`.

## At a glance

```rust
// src/main.rs of a generated app
#[tokio::main]
async fn main() {
    doido::generators::run(Some(config::routes::router())).await;
}
```

## Runtime commands

| Command | Description |
|---------|-------------|
| `cargo doido server` | Start the axum HTTP server |
| `cargo doido routes` | Print the route table |
| `cargo doido console` | Interactive console with app context |
| `cargo doido db <cmd>` | `migrate`, `rollback`, `reset`, `status`, `seed` |
| `cargo doido worker [--once]` | Run the background job worker |
| `cargo doido jobs <cmd>` | Inspect/retry/discard background jobs |
| `cargo doido credentials <cmd>` | Manage credentials |
| `cargo doido generate <name> …` | Run a code generator |
| `cargo doido destroy <name> …` | Reverse a generator |
| `doido new <app>` | Create a new application |

```bash
cargo doido db migrate          # run pending migrations
cargo doido worker --once       # drain the queue and exit
cargo doido routes              # print every registered route
```

## Creating an application

`doido new` scaffolds a Rails-style project; pick the database driver with `--database`.

```bash
doido new blog --database=sqlite   # or postgres | mysql
cd blog
cargo doido db create && cargo doido db migrate
cargo doido server
```

## Code generators

Run `cargo doido generate` with no arguments to list every registered generator. Each writes
files (and some inject routes):

| Generator | Generates |
|-----------|-----------|
| `model` | `app/models/<name>.rs` + migration |
| `migration` | a standalone migration |
| `controller` | a `#[controller]` with action stubs (+ route) |
| `scaffold` | model + migration + controller + views + route |
| `resource` | model + migration + controller + route (no views) |
| `mailer` | a mailer + templates |
| `job` | a background job |
| `channel` | a WebSocket channel |
| `templates` | view templates for an existing controller |
| `locale` | a locale file |
| `generator` | a new custom generator skeleton |
| `storage:install` | storage tables + config |
| `storage:adapter` | a custom storage adapter skeleton |

```bash
cargo doido generate model Post title:string body:text
cargo doido generate scaffold Post title:string body:text     # full CRUD stack
cargo doido generate controller Pages home about
cargo doido generate mailer User welcome
```

## The field DSL

Model, scaffold, and resource generators take fields as `name:type[:modifier…]`. Types map
to migration columns; modifiers add constraints and indexes.

```bash
cargo doido generate model Post \
  title:string:not_null \
  slug:string:unique \
  body:text \
  author:references \
  views:integer:index
```

## Route auto-injection

Generators that produce a controller (`scaffold`, `resource`, `controller`) parse
`config/routes.rs`, insert the matching route (e.g. `resources!(posts, PostsController);`)
into the `routes! { … }` block, and skip controllers already registered — so a generated
resource is reachable without editing routes by hand.

## Reversing a generator

`cargo doido destroy` removes what the matching `generate` created.

```bash
cargo doido generate scaffold Post title:string
cargo doido destroy  scaffold Post           # remove the generated files (and route)
```

## Custom generators

The generator system is an extensible registry. Implement the `Generator` trait (returning
`GeneratedFile`s) and register it; `cargo doido generate generator <name>` scaffolds one for you.

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

// Register it, then run/list through the registry:
let mut registry = doido::generators::default_registry();
registry.register(Box::new(PolicyGenerator));
let files = registry.run("policy", &["post"])?;
let names = registry.list(); // includes "policy"
```

## See also

- [Models](@/docs/reference/models.md) — what `generate model`/`migration` produce.
- [Controllers & routing](@/docs/reference/controllers.md) — the `routes!` block generators edit.
- [Jobs](@/docs/reference/jobs.md), [Mailer](@/docs/reference/mailer.md), [Cable](@/docs/reference/cable.md) — their generators.
