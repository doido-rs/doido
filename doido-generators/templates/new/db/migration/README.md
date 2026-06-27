# Migration

A [SeaORM](https://www.sea-ql.org/SeaORM/) migration project for this Doido app.

## Running migrations

From the application root:

```sh
# Apply all pending migrations
cargo run --manifest-path db/migration/Cargo.toml -- up

# Roll back the last migration
cargo run --manifest-path db/migration/Cargo.toml -- down
```

Or, with the SeaORM CLI installed:

```sh
sea-orm-cli migrate up
sea-orm-cli migrate down
```

## Adding migrations

Generate a new migration file and register it in `src/lib.rs`:

```sh
doido generate migration CreateUsers
```
