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

### 1. Re-exports

```rust
// users import from doido_model, not sea_orm directly
pub use sea_orm::*;
```

All sea-orm traits, macros, types, and query builders are available through `doido_model`.

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

Users define models exactly as sea-orm documents:

```rust
// models/post.rs — pure sea-orm, no doido magic
use doido_model::*;  // re-exports sea_orm::*

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
pub enum Relation {
    #[sea_orm(has_many = "super::comment::Entity")]
    Comment,
}

impl Related<super::comment::Entity> for Entity {
    fn to() -> RelationDef { Relation::Comment.def() }
}

impl ActiveModelBehavior for ActiveModel {}
```

Queries follow sea-orm conventions:
```rust
let posts = Entity::find()
    .filter(Column::Published.eq(true))
    .all(&ctx.db)
    .await?;
```

## Migrations

- Use sea-orm CLI and migration crate (`sea-orm-migration`) directly
- `doido-cli` wraps `sea-orm-cli` commands under `doido db migrate / rollback / status`
- Migration files live in `db/migrations/` by convention

## Open Questions (remaining)

- [ ] Should `doido_model` expose a convenience `Model::find_by_id(db, id)` shorthand, or leave that to sea-orm's `Entity::find_by_id(id).one(db)`?

## TDD Surface

- Test `connection()` returns a valid `DatabaseConnection` after `setup()`
- Test `testing::setup_db()` returns a working in-memory SQLite connection
- Test `testing::run_migrations(db)` applies all migrations cleanly
- Test `testing::seed(db, rows)` inserts and the rows are queryable
- Integration test: controller action uses `ctx.db` to query via sea-orm, results correct
