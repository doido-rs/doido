+++
title = "Models"
description = "Entidades sea-orm, o pool de conexões, migrations no estilo Rails, validações, senhas seguras e helpers de teste."
weight = 3
+++

> **Especificação de design:** [`docs/03-model.md`](https://github.com/doido-rs/doido/blob/master/docs/03-model.md).
> Este guia documenta a API como implementada em `doido-model`.

**Análogo no Rails: Active Record.** `doido-model` reexporta o [sea-orm](https://www.sea-ql.org/SeaORM/)
por completo — você define entidades e faz queries exatamente como o sea-orm documenta,
sem wrapper — e adiciona a cola do framework: um pool de conexões global vindo da config,
helpers de migration no estilo Rails, validações, callbacks, normalização, senhas seguras,
associações, serialização, factories e um banco de dados de teste em memória.

## Visão geral

```rust
use doido::model::{connect, connect_with_url, pool, TestDb};
use doido::model::{ActiveModelTrait, EntityTrait, ColumnTrait, QueryFilter, Set};
use doido::model::migration::{create_table, alter_table, add_index, add_foreign_key};
```

## Definindo um model

Models são entidades sea-orm puras. `doido-model` reexporta tudo o que o sea-orm precisa,
então não há nada específico do Doido para aprender aqui.

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
```

Faça queries direto com o sea-orm (não existe, intencionalmente, um `Model::find_by_id`):

```rust
use doido::model::{EntityTrait, ColumnTrait, QueryFilter};

let published = post::Entity::find()
    .filter(post::Column::Published.eq(true))
    .all(pool::pool())
    .await?;

let one = post::Entity::find_by_id(1).one(pool::pool()).await?;
```

## O pool de conexões

`connect()` abre uma conexão a partir da seção `database` da config (`connect_with_url()`
para uma URL explícita); `pool::set_pool()` a instala como o pool global do processo, lido
de volta com `pool::pool()` (dá panic se não definido) ou `pool::try_pool()` (retorna
`Option`). Os controllers alcançam o mesmo pool via
[`ctx.db()`](@/docs/reference/controllers.pt.md).

```rust
// No boot:
let conn = doido::model::connect().await?;   // lê config/<env>.yml → [database]
doido::model::pool::set_pool(conn).ok();

// Em qualquer lugar depois:
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

Helpers no estilo Rails envolvem o `SchemaManager` do sea-orm. Cada um é uma função livre
que recebe o manager primeiro; `create_table`/`alter_table` recebem uma closure sobre um
builder de colunas com helpers tipados (`string`, `text`, `integer`, `big_integer`,
`float`, …, além de `not_null`, `unique_key`, `timestamps`). Chame-os direto do `up`/`down`
de uma migration.

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

## Seeds

Toda app criada com `doido new` inclui um membro de workspace `db/seed/` — um
binário Rust único (não são migrations versionadas) que insere dados de fixture
usando os models SeaORM em `app/models/`. Edite `db/seed/src/main.rs` e rode:

```bash
cargo doido db seed
# ou: cargo run --manifest-path db/seed/Cargo.toml
```

O runner lê `DATABASE_URL` do ambiente, ou cai para `config/<env>.yml`
(`database.url`). Depois de `cargo doido generate model …`, o novo model já está
disponível pelo módulo `models` ligado em `db/seed/src/main.rs`.

Veja `db/seed/README.md` na app gerada para um exemplo completo.

## Validações

Implemente `Validate` para coletar erros em um acumulador `Errors` com helpers como
`presence` e `length`; `is_valid()` e `full_messages()` vêm de graça.

```rust
use doido::model::validation::{Validate, Errors};

struct Post { title: String, body: String }

impl Validate for Post {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e.length("body", &self.body, Some(10), None); // mínimo 10, sem máximo
        e
    }
}

let post = Post { title: "".into(), body: "short".into() };
assert!(!post.is_valid());
let messages = post.validate().full_messages();
```

## Senhas seguras

Hash bcrypt com um trait no estilo `has_secure_password`. `hash_password` usa o custo
padrão; `verify_password` verifica um candidato; `generate_token` gera um token aleatório.

```rust
use doido::model::password::{hash_password, verify_password, generate_token};

let digest = hash_password("s3cret")?;
assert!(verify_password("s3cret", &digest));
assert!(!verify_password("wrong", &digest));

let reset_token = generate_token();
```

## Normalização

Normalize valores de atributos antes de persistir com um `Normalizer` combinável
(`strip`, `downcase`, `upcase`, `squish`, `custom`).

```rust
use doido::model::normalization::Normalizer;

let email = Normalizer::new().strip().downcase();
assert_eq!(email.apply("  Foo@Bar.COM  "), "foo@bar.com");

let squished = Normalizer::new().squish();
assert_eq!(squished.apply("  hello   world  "), "hello world");
```

## Associações

Consulte relações nativamente pelo `Related`/`DeriveRelation` do sea-orm. `doido-model`
também oferece descritores de associação (`belongs_to`, `has_one`, `has_many`,
`has_and_belongs_to_many`, além de `PolymorphicAssociation` e a nomenclatura de
`join_table`) usados pelos geradores e por attachments polimórficos.

```rust
use doido::model::association::{Association, join_table};

let a = Association::has_many("Author", "posts");
let b = Association::belongs_to("author");
let name = join_table("authors", "books"); // "authors_books"
```

## Serialização

Transforme qualquer model `Serialize` em JSON, opcionalmente mascarando ou selecionando
colunas — útil para respostas de API.

```rust
use doido::model::serialization::{as_json, as_json_only, as_json_except};

let full = as_json(&user);
let public = as_json_only(&user, &["id", "name"]);       // apenas estas chaves
let safe = as_json_except(&user, &["password_digest"]);  // esconde chaves sensíveis
```

## Factories

Construa registros de teste com valores de sequência únicos.

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

## Múltiplos bancos de dados

`Databases` mantém uma conexão de escrita e uma réplica de leitura opcional, selecionadas
por `Role`.

```rust
use doido::model::databases::{Databases, Role};

let dbs = Databases::new(writing_conn).with_reading(replica_conn);
let read = dbs.connection(Role::Reading); // usa a de escrita quando não há réplica
```

## Banco de dados de teste

`TestDb` sobe uma conexão SQLite em memória para testes rápidos e isolados.

```rust
use doido::model::TestDb;

#[tokio::test]
async fn creates_a_user() {
    let db = TestDb::new().await.unwrap();
    let conn = db.conn();
    // rode migrations, insira e verifique contra `conn`
}
```

## Especificação vs. implementação

> As queries usam o sea-orm nativamente — **não** existe atalho `Model::find_by_id(db, id)`;
> use `Entity::find_by_id(id).one(db)`. Callbacks, scopes, seeds, transações, enums e tasks
> têm cada um seu módulo (`doido::model::{callbacks, scope, seeds, transaction, enums,
> tasks}`) espelhando seus equivalentes no Active Record.

## Veja também

- [Configuração](@/docs/reference/configuration.pt.md) — a seção `database`.
- [Controllers & roteamento](@/docs/reference/controllers.pt.md) — `ctx.db()` dentro das actions.
- [Geradores & CLI](@/docs/reference/generators.pt.md) — `cargo doido generate model`, `cargo doido db migrate` e `cargo doido db seed`.
