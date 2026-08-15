# Seed

Rust seed runner for this Doido app. Unlike `db/migration/` (many versioned
migrations), `db/seed/` is a **single** executable that inserts fixture data
using the SeaORM models in `app/models/`.

Imports use `doido::model` — not upstream SeaORM directly.

## Running seeds

From the application root:

```sh
# Via the Doido CLI (recommended)
doido db seed

# Or directly
cargo run --manifest-path db/seed/Cargo.toml
```

The runner reads `DATABASE_URL` from the environment, or falls back to
`config/<env>.yml` (`database.url`).

## Default user

| Field    | Value                    |
|----------|--------------------------|
| Email    | `admin@economizei.local` |
| Password | `password`               |
