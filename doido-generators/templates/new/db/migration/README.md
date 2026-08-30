# Migration

A [SeaORM](https://www.sea-ql.org/SeaORM/) migration project for this Doido app.
Imports use `doido::model::sea_orm_migration` — not the upstream crate directly.

This crate is a **library**: it exports a `Migrator`, which the app links and runs
in-process. `doido db migrate` calls the `Migrator` from the app binary (registered
via `.migrator::<migration::Migrator>()` in `src/main.rs`) — there is no separate
migration binary and no `cargo run` subprocess, so migration SQL is logged like any
other statement.

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

Generate a new migration file and register it in `src/lib.rs`:

```sh
doido generate migration CreateUsers
```
