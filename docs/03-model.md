# doido-model — Spec

Rails analogue: **Active Record** (thin abstraction, not a full replacement)

> **Status (2026-07-28): mostly done.** sea-orm re-export, connection pool, Rails-style
> schema builders, and db tasks (`seeds`, reset/setup/prepare, `schema` dump/load, migrate
> rollback/redo) are implemented, plus `TestDb`. Note: the pool is initialized from **YAML**
> config, not `doido-config` (spec 05). **Open:** the spec's `testing::run_migrations(db)` /
> `testing::seed(db, entities)` convenience helpers below are not present (only `TestDb`).
> See [ARCHITECTURE.md](ARCHITECTURE.md).

## Decisions (resolved in interview)

- **doido-model does NOT wrap sea-orm** — it re-exports sea-orm's full interface
- Users work with sea-orm natively: `EntityTrait`, `ActiveModelTrait`, `DeriveEntityModel`, relations, migrations — all as sea-orm intends
- Doido's only addition: framework integration glue (connection pool from `doido-config`, test helpers)

## What doido-model Provides

### 1. Re-exports (mandatory import path)

Workspace and generated code must **not** depend on `sea_orm`, `sea_orm_migration`, or
`sea_orm_cli` directly. Import through `doido-model`:

```rust
use doido_model::sea_orm::{EntityTrait, DatabaseConnection, …};
use doido_model::sea_orm_migration::prelude::*;
use doido_model::sea_orm_cli::{Commands, …};  // feature `cli` on doido-model
```

Inside `doido-model` itself, use `crate::sea_orm` / `crate::sea_orm_migration`.

All sea-orm traits, macros, types, and query builders are available through those paths.
Enable the `sqlite` / `postgres` / `mysql` feature matching your database (on
`doido-model` or the meta `doido` crate).

### 2. Framework Integration

- `doido_model::connection()` — returns the app's shared `DatabaseConnection` (initialized by `doido-config`)
- `doido_model::setup(config)` — called at app boot to connect and store the pool
- `Context.db` in controllers is a `&DatabaseConnection` provided by this module

### 3. Test Helpers (`doido_model::testing`)

- `testing::setup_db()` — spins up an in-memory SQLite connection for tests
- `testing::run_migrations(db)` — runs all pending migrations on a test DB
- `testing::seed(db, entities)` — inserts fixture rows
- No mocking — real DB, real queries, SQLite in-process

## Sea-ORM Native Workflow (unchanged)

Generated apps split models into two layers:

| Path | Purpose | Overwritten on `doido db migrate`? |
|------|---------|-------------------------------------|
| `app/models/_entities/<name>.rs` | SeaORM entity (`Model`, `Entity`, `Column`, …) | **Yes** — exported from the database after every schema-changing migrate |
| `app/models/<name>.rs` | App extensions (validations, auth traits, custom methods) | **No** — created once by generators; safe to edit forever |

`doido db migrate` (and `doido db generate entity`) writes into `app/models/_entities/`.
The sibling `app/models/<name>.rs` re-exports the entity and holds anything you add by hand:

```rust
// app/models/post.rs — safe to edit
pub use super::_entities::post::*;

use doido::model::sea_orm::ActiveModelBehavior;

impl ActiveModelBehavior for ActiveModel {}

use doido_model::validation::{Errors, Validate};

impl Validate for Model {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e
    }
}
```

The generated entity (always rewritten) lives at `app/models/_entities/post.rs`:

```rust
// app/models/_entities/post.rs — regenerated on migrate
use doido::model::sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub title: String,
    pub body: String,
    pub published: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
```

Queries follow sea-orm conventions (import through `app/models/post.rs` or `models::post`):
```rust
let posts = Entity::find()
    .filter(Column::Published.eq(true))
    .all(&ctx.db)
    .await?;
```

## Migrations

- Migration crates depend on **`doido-model`** only (with the database feature); use
  `doido_model::sea_orm_migration::prelude::*` — not a direct `sea-orm-migration` dep
- `doido db` embeds the SeaORM CLI via `doido_model::sea_orm_cli` (feature `cli`)
- Migration files live in `db/migration/` by convention
- After every schema-changing `doido db migrate`, entities are re-exported to
  `app/models/_entities/` and missing extension stubs are created under `app/models/`
  (see `doido_model::entities`)

## Schema ER diagram export

`doido db schema diagram` introspects the live database via `sea-schema`, maps the result
into an engine-agnostic [`SchemaDesign`](doido-model/src/schema_design/model.rs) model
(tables, columns, primary keys, foreign keys, indexes, constraints), and writes a
self-contained HTML ER diagram (default: `db/schema.html`).

```sh
doido db schema diagram
doido db schema diagram --output docs/er.html
doido db schema diagram --ignore-table audit_logs
```

The HTML shows table/column names with **PK** / **FK** badges; hover tooltips expose
column types, nullability, defaults, indexes, and foreign-key actions. A
`<script id="doido-schema-design">` block embeds the full `SchemaDesign` JSON for tooling
and e2e validation.

Programmatic use (feature `cli` on `doido-model`):

```rust
use doido_model::{introspect_from_url, export_html, resolve_ignore_tables};

let ignore = resolve_ignore_tables(&[]);
let design = introspect_from_url(&database_url, None, &ignore).await?;
let html = export_html(&design)?;
```

## Open Questions (remaining)

- [ ] Should `doido_model` expose a convenience `Model::find_by_id(db, id)` shorthand, or leave that to sea-orm's `Entity::find_by_id(id).one(db)`?

## TDD Surface

- Test `connection()` returns a valid `DatabaseConnection` after `setup()`
- Test `testing::setup_db()` returns a working in-memory SQLite connection
- Test `testing::run_migrations(db)` applies all migrations cleanly
- Test `testing::seed(db, rows)` inserts and the rows are queryable
- Integration test: controller action uses `ctx.db` to query via sea-orm, results correct
