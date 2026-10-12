# Migration

SeaORM migrations for this Doido app, compiled into the app binary as the
`db/migration` module. Imports use `doido::model::sea_orm_migration` — not the
upstream crate directly.

`doido db migrate` calls `migration::Migrator` in-process (registered via
`.migrator::<migration::Migrator>()` in `src/main.rs`) — there is no separate
migration crate and no `cargo run` subprocess, so migration SQL is logged like
any other statement.

## Running migrations

From the application root:

```sh
# Apply all pending migrations
doido db migrate

# Roll back the last migration
doido db migrate down

# Migration status
doido db migrate status
```

## Adding migrations

Generate a new migration file and register it in `mod.rs`:

```sh
doido generate migration CreateUsers
```

Generated migrations use `doido::model::migration` helpers (`create_table`,
`alter_table`, `create_index`, `update_where`, …). Do not call
`execute_unprepared` or embed hand-written SQL — extend the framework DSL in
`doido-model` when you need new DDL/DML patterns.

`doido db migrate --without-entities` skips re-exporting SeaORM entities after
schema changes (same as SeaORM CLI).
