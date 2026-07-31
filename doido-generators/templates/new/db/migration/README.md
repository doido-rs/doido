# Migration

A [SeaORM](https://www.sea-ql.org/SeaORM/) migration project for this Doido app.
Imports use `doido::model::sea_orm_migration` — not the upstream crate directly.

## Running migrations

From the application root:

```sh
# Apply all pending migrations
cargo run --manifest-path db/migration/Cargo.toml -- up

# Roll back the last migration
cargo run --manifest-path db/migration/Cargo.toml -- down

# Or via the Doido CLI (uses `doido::model::sea_orm_cli` under the hood)
doido db migrate
doido db rollback
```

## Adding migrations

Generate a new migration file and register it in `src/lib.rs`:

```sh
doido generate migration CreateUsers
```
