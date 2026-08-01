+++
title = "Models"
description = "sea-orm entities, the connection pool, Rails-style migrations, validations, secure passwords, and test helpers."
weight = 3
aliases = ['/docs/guides/models/']

+++

> **Design spec:** [`docs/03-model.md`](https://github.com/doido-rs/doido/blob/master/docs/03-model.md).
> This guide documents the API as implemented in `doido-model`.

**Rails analogue: Active Record.** `doido-model` re-exports [sea-orm](https://www.sea-ql.org/SeaORM/)
in full — you define entities and run queries exactly as sea-orm documents, with no
wrapper — and layers on framework glue: a global connection pool from config, Rails-style
migration helpers, validations, callbacks, normalization, secure passwords, associations,
serialization, factories, and an in-memory test database.

## At a glance

```rust
use doido::model::{connect, connect_with_url, pool, TestDb};
use doido::model::{ActiveModelTrait, EntityTrait, ColumnTrait, QueryFilter, Set};
use doido::model::migration::{create_table, alter_table, add_index, add_foreign_key};
```

## Defining a model

Models are plain sea-orm entities. `doido-model` re-exports everything sea-orm needs, so
there is nothing Doido-specific to learn here.

```rust
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
pub enum Relation {
    #[sea_orm(has_many = "super::comment::Entity")]
    Comments,
}

impl ActiveModelBehavior for ActiveModel {}
```

Query with sea-orm directly (there is intentionally no `Model::find_by_id` sugar):

```rust
use doido::model::{EntityTrait, ColumnTrait, QueryFilter};

let published = post::Entity::find()
    .filter(post::Column::Published.eq(true))
    .all(pool::pool())
    .await?;

let one = post::Entity::find_by_id(1).one(pool::pool()).await?;
```

## The connection pool

`connect()` opens a connection from the `database` config section (`connect_with_url()`
for an explicit URL); `pool::set_pool()` installs it as the process-global pool, read back
with `pool::pool()` (panics if unset) or `pool::try_pool()` (returns `Option`). Controllers
reach the same pool through [`ctx.db()`](@/docs/reference/controllers.md).

```rust
// At boot:
let conn = doido::model::connect().await?;   // reads config/<env>.yml → [database]
doido::model::pool::set_pool(conn).ok();

// Anywhere afterwards:
let db = doido::model::pool::pool();          // &'static DatabaseConnection
```

```yaml
# config/development.yml
database:
  url: sqlite://db/development.db
  # max_connections: 10
  # connect_timeout: 5
```

## Migrations

Rails-style helpers wrap sea-orm's `SchemaManager`. Each is a free function taking the
manager first; `create_table`/`alter_table` take a closure over a column builder with
typed helpers (`string`, `text`, `integer`, `big_integer`, `float`, …, plus `not_null`,
`unique_key`, `timestamps`). Call them straight from a migration's `up`/`down`.

```rust
use doido::model::migration::{create_table, alter_table, add_index, add_foreign_key, drop_table};

create_table(manager, "users", |t| {
    t.string("email").not_null().unique_key();
    t.string("name");
    t.timestamps();
})
.await?;

alter_table(manager, "users", |t| {
    t.add_column("age", |c| { c.integer(); });
    t.rename_column("name", "full_name");
})
.await?;

add_index(manager, "users", &["email"]).await?;
add_foreign_key(manager, "posts", "user_id", "users", "id").await?;
drop_table(manager, "users").await?;
```

## Validations

Implement `Validate` to collect errors into an `Errors` accumulator with helpers like
`presence` and `length`; `is_valid()` and `full_messages()` come for free.

```rust
use doido::model::validation::{Validate, Errors};

struct Post { title: String, body: String }

impl Validate for Post {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e.length("body", &self.body, Some(10), None); // min 10, no max
        e
    }
}

let post = Post { title: "".into(), body: "short".into() };
assert!(!post.is_valid());
let messages = post.validate().full_messages();
```

## Secure passwords

bcrypt hashing with a `has_secure_password`-style trait. `hash_password` uses the default
cost; `verify_password` checks a candidate; `generate_token` mints a random token.

```rust
use doido::model::password::{hash_password, verify_password, generate_token};

let digest = hash_password("s3cret")?;
assert!(verify_password("s3cret", &digest));
assert!(!verify_password("wrong", &digest));

let reset_token = generate_token();
```

## Normalization

Normalize attribute values before persisting with a composable `Normalizer`
(`strip`, `downcase`, `upcase`, `squish`, `custom`).

```rust
use doido::model::normalization::Normalizer;

let email = Normalizer::new().strip().downcase();
assert_eq!(email.apply("  Foo@Bar.COM  "), "foo@bar.com");

let squished = Normalizer::new().squish();
assert_eq!(squished.apply("  hello   world  "), "hello world");
```

## Associations

Query relations natively through sea-orm's `Related`/`DeriveRelation`. `doido-model` also
offers association descriptors (`belongs_to`, `has_one`, `has_many`,
`has_and_belongs_to_many`, plus `PolymorphicAssociation` and `join_table` naming) used by
generators and polymorphic attachments.

```rust
use doido::model::association::{Association, join_table};

let a = Association::has_many("Author", "posts");
let b = Association::belongs_to("author");
let name = join_table("authors", "books"); // "authors_books"
```

## Serialization

Turn any `Serialize` model into JSON, optionally masking or whitelisting columns — handy
for API responses.

```rust
use doido::model::serialization::{as_json, as_json_only, as_json_except};

let full = as_json(&user);
let public = as_json_only(&user, &["id", "name"]);       // only these keys
let safe = as_json_except(&user, &["password_digest"]);  // hide sensitive keys
```

## Factories

Build test records with unique sequence values.

```rust
use doido::model::factory::{sequence, Factory};

impl Factory for User {
    fn build() -> Self {
        let n = sequence();
        User { email: format!("user{n}@example.com"), name: "Test User".into() }
    }
}

let user = User::build();
let three = User::build_list(3); // distinct records
```

## Multiple databases

`Databases` holds a writing connection and an optional read replica, selected by `Role`.

```rust
use doido::model::databases::{Databases, Role};

let dbs = Databases::new(writing_conn).with_reading(replica_conn);
let read = dbs.connection(Role::Reading); // falls back to writing when no replica
```

## Test database

`TestDb` spins up an in-memory SQLite connection for fast, isolated tests.

```rust
use doido::model::TestDb;

#[tokio::test]
async fn creates_a_user() {
    let db = TestDb::new().await.unwrap();
    let conn = db.conn();
    // run migrations, insert, and assert against `conn`
}
```

## Spec vs. implementation

> Queries use sea-orm natively — there is **no** `Model::find_by_id(db, id)` shorthand;
> use `Entity::find_by_id(id).one(db)`. Callbacks, scopes, seeds, transactions, enums, and
> tasks each have their own module (`doido::model::{callbacks, scope, seeds, transaction,
> enums, tasks}`) mirroring their Active Record counterparts.

## See also

- [Configuration](@/docs/reference/configuration.md) — the `database` section.
- [Controllers & routing](@/docs/reference/controllers.md) — `ctx.db()` inside actions.
- [Generators & CLI](@/docs/reference/generators.md) — `doido generate model` and `doido db migrate`.
