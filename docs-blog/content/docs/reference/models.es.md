+++
title = "Modelos"
description = "Entidades sea-orm, el pool de conexiones, migraciones al estilo Rails, validaciones, contraseñas seguras y helpers de pruebas."
weight = 3
+++

> **Especificación de diseño:** [`docs/03-model.md`](https://github.com/doido-rs/doido/blob/master/docs/03-model.md).
> Esta guía documenta la API tal como está implementada en `doido-model`.

**Análogo en Rails: Active Record.** `doido-model` reexporta [sea-orm](https://www.sea-ql.org/SeaORM/)
por completo — defines entidades y haces queries exactamente como documenta sea-orm, sin
wrapper — y añade el pegamento del framework: un pool de conexiones global desde la config,
helpers de migración al estilo Rails, validaciones, callbacks, normalización, contraseñas
seguras, asociaciones, serialización, factories y una base de datos de pruebas en memoria.

## Vistazo general

```rust
use doido::model::{connect, connect_with_url, pool, TestDb};
use doido::model::{ActiveModelTrait, EntityTrait, ColumnTrait, QueryFilter, Set};
use doido::model::migration::{create_table, alter_table, add_index, add_foreign_key};
```

## Definir un modelo

Los modelos son entidades sea-orm puras. `doido-model` reexporta todo lo que sea-orm
necesita, así que no hay nada específico de Doido que aprender aquí.

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

Haz queries directamente con sea-orm (no hay, intencionalmente, un `Model::find_by_id`):

```rust
use doido::model::{EntityTrait, ColumnTrait, QueryFilter};

let published = post::Entity::find()
    .filter(post::Column::Published.eq(true))
    .all(pool::pool())
    .await?;

let one = post::Entity::find_by_id(1).one(pool::pool()).await?;
```

## El pool de conexiones

`connect()` abre una conexión desde la sección `database` de la config
(`connect_with_url()` para una URL explícita); `pool::set_pool()` la instala como el pool
global del proceso, leído con `pool::pool()` (hace panic si no está definido) o
`pool::try_pool()` (devuelve `Option`). Los controladores alcanzan el mismo pool vía
[`ctx.db()`](@/docs/reference/controllers.es.md).

```rust
// En el arranque:
let conn = doido::model::connect().await?;   // lee config/<env>.yml → [database]
doido::model::pool::set_pool(conn).ok();

// En cualquier lugar después:
let db = doido::model::pool::pool();          // &'static DatabaseConnection
```

```yaml
# config/development.yml
database:
  url: sqlite://db/development.db
  # max_connections: 10
  # connect_timeout: 5
```

## Migraciones

Los helpers al estilo Rails envuelven el `SchemaManager` de sea-orm. Cada uno es una
función libre que recibe el manager primero; `create_table`/`alter_table` reciben una
closure sobre un builder de columnas con helpers tipados (`string`, `text`, `integer`,
`big_integer`, `float`, …, además de `not_null`, `unique_key`, `timestamps`). Llámalos
directamente desde el `up`/`down` de una migración.

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

## Validaciones

Implementa `Validate` para recolectar errores en un acumulador `Errors` con helpers como
`presence` y `length`; `is_valid()` y `full_messages()` vienen gratis.

```rust
use doido::model::validation::{Validate, Errors};

struct Post { title: String, body: String }

impl Validate for Post {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e.length("body", &self.body, Some(10), None); // mínimo 10, sin máximo
        e
    }
}

let post = Post { title: "".into(), body: "short".into() };
assert!(!post.is_valid());
let messages = post.validate().full_messages();
```

## Contraseñas seguras

Hash bcrypt con un trait al estilo `has_secure_password`. `hash_password` usa el coste por
defecto; `verify_password` verifica un candidato; `generate_token` genera un token
aleatorio.

```rust
use doido::model::password::{hash_password, verify_password, generate_token};

let digest = hash_password("s3cret")?;
assert!(verify_password("s3cret", &digest));
assert!(!verify_password("wrong", &digest));

let reset_token = generate_token();
```

## Normalización

Normaliza valores de atributos antes de persistir con un `Normalizer` combinable
(`strip`, `downcase`, `upcase`, `squish`, `custom`).

```rust
use doido::model::normalization::Normalizer;

let email = Normalizer::new().strip().downcase();
assert_eq!(email.apply("  Foo@Bar.COM  "), "foo@bar.com");

let squished = Normalizer::new().squish();
assert_eq!(squished.apply("  hello   world  "), "hello world");
```

## Asociaciones

Consulta relaciones nativamente mediante `Related`/`DeriveRelation` de sea-orm.
`doido-model` también ofrece descriptores de asociación (`belongs_to`, `has_one`,
`has_many`, `has_and_belongs_to_many`, además de `PolymorphicAssociation` y la nomenclatura
de `join_table`) usados por los generadores y por los attachments polimórficos.

```rust
use doido::model::association::{Association, join_table};

let a = Association::has_many("Author", "posts");
let b = Association::belongs_to("author");
let name = join_table("authors", "books"); // "authors_books"
```

## Serialización

Convierte cualquier modelo `Serialize` en JSON, opcionalmente enmascarando o
seleccionando columnas — útil para respuestas de API.

```rust
use doido::model::serialization::{as_json, as_json_only, as_json_except};

let full = as_json(&user);
let public = as_json_only(&user, &["id", "name"]);       // solo estas claves
let safe = as_json_except(&user, &["password_digest"]);  // oculta claves sensibles
```

## Factories

Construye registros de prueba con valores de secuencia únicos.

```rust
use doido::model::factory::{sequence, Factory};

impl Factory for User {
    fn build() -> Self {
        let n = sequence();
        User { email: format!("user{n}@example.com"), name: "Test User".into() }
    }
}

let user = User::build();
let three = User::build_list(3); // registros distintos
```

## Múltiples bases de datos

`Databases` mantiene una conexión de escritura y una réplica de lectura opcional,
seleccionadas por `Role`.

```rust
use doido::model::databases::{Databases, Role};

let dbs = Databases::new(writing_conn).with_reading(replica_conn);
let read = dbs.connection(Role::Reading); // usa la de escritura cuando no hay réplica
```

## Base de datos de pruebas

`TestDb` levanta una conexión SQLite en memoria para pruebas rápidas y aisladas.

```rust
use doido::model::TestDb;

#[tokio::test]
async fn creates_a_user() {
    let db = TestDb::new().await.unwrap();
    let conn = db.conn();
    // ejecuta migraciones, inserta y verifica contra `conn`
}
```

## Especificación vs. implementación

> Las queries usan sea-orm nativamente — **no** hay atajo `Model::find_by_id(db, id)`; usa
> `Entity::find_by_id(id).one(db)`. Callbacks, scopes, seeds, transacciones, enums y tasks
> tienen cada uno su módulo (`doido::model::{callbacks, scope, seeds, transaction, enums,
> tasks}`) reflejando sus equivalentes de Active Record.

## Véase también

- [Configuración](@/docs/reference/configuration.es.md) — la sección `database`.
- [Controladores y enrutamiento](@/docs/reference/controllers.es.md) — `ctx.db()` dentro de las actions.
- [Generadores y CLI](@/docs/reference/generators.es.md) — `cargo doido generate model` y `cargo doido db migrate`.
