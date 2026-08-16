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

## Adding seed data

Edit `src/main.rs` and use your app models, for example:

```rust
use doido::model::sea_orm::{ActiveModelTrait, EntityTrait, Set};
use models::user::{ActiveModel, Entity};

if Entity::find().one(&db).await?.is_none() {
    ActiveModel {
        email: Set("admin@example.com".into()),
        ..Default::default()
    }
    .insert(&db)
    .await?;
}
```

After `doido generate model …`, the new model module is already available through
the `models` import wired in `src/main.rs`.
