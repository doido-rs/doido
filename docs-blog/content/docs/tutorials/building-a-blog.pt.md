+++
title = "Construindo um blog"
description = "Construa um blog renderizado no servidor com um scaffold, autoria protegida por login e comentários de leitores."
weight = 2
+++

Este tutorial constrói um blog pequeno, mas completo, sobre a base do [Primeiros
passos](@/docs/tutorials/getting-started.pt.md). Ao final você terá:

- uma página inicial **pública** que lista os posts publicados e uma página para ler um post,
- **comentários** que qualquer leitor pode deixar em um post,
- **autoria protegida por login** — escrever e publicar posts é protegido pelo
  [`doido-auth`](@/docs/reference/auth.pt.md), e cada post pertence ao seu autor.

É uma app HTML pura (sem API). Nós usamos **geradores**: o `scaffold` cria o recurso Post
inteiro em um comando, e o `generate controller` nos dá o endpoint de comentários. Você só
edita à mão as *customizações* — os geradores escrevem (e conectam) os esqueletos.

> Cada comando e bloco de código abaixo é executado pelo próprio e2e de release do Doido
> (`doido-generators/tests/e2e/scenarios/blog_tutorial.rs`), então o tutorial continua
> executável. Veja o [padrão de tutoriais](@/docs/reference/generators.pt.md).

Este é o mapa de rotas que estamos construindo:

| Método | Caminho | Quem | Propósito |
|--------|---------|------|-----------|
| GET | `/` | todos | lista os posts publicados |
| GET | `/posts/{id}` | todos | ler um post + seus comentários |
| POST | `/posts/{post_id}/comments` | todos | deixar um comentário |
| GET/POST/… | `/posts/new`, `/posts`, `/posts/{id}/edit`… | autor logado | gerenciar posts |
| GET/POST | `/users/sign_in`, `/users/sign_up` | autor | auth (gerado por `--auth`) |

## Criar a app

Crie uma nova aplicação já com autenticação embutida e prepare o banco:

```bash
# --auth adiciona o doido-auth e roda o auth:install (model User, controllers de sign-in/up + rotas)
doido new blog --database=sqlite --auth
cd blog

cargo doido db create
cargo doido db migrate      # cria a tabela users

cargo doido server          # http://0.0.0.0:3000 — sign-in/up já funcionam
```

O `--auth` te dá um model `User`, um `SessionsController` e um `RegistrationsController`, e
rotas de sign-in / sign-up / sign-out sob `/users`. Vamos nos apoiar nelas para proteger a
autoria. Veja a [referência de Auth](@/docs/reference/auth.pt.md) para o quadro completo.

## Scaffold do recurso Post

Um post tem um título, um corpo, uma flag de publicado e um autor (o `User` logado). Em vez de
escrever o model, o controller, as views e a rota à mão, **faça o scaffold do recurso inteiro**
em um comando:

```bash
cargo doido generate scaffold Post \
  title:string:not_null \
  body:text:not_null \
  published:boolean:not_null \
  user:references
cargo doido db migrate
```

Esse único comando escreveu:

- `app/models/post.rs` — uma entidade [sea-orm](@/docs/reference/models.pt.md) (com uma chave
  estrangeira `user_id` do tipo `i64`, vinda de `user:references`),
- uma migration para a tabela `posts`,
- `app/controllers/posts_controller.rs` — um controller CRUD completo,
- `app/views/posts/{index,show,new,edit,_form}.html.tera`,
- e **injetou a rota** `resources!(posts, PostsController);` em `config/routes.rs`.

Como o gerador injeta a rota *junto* com o controller, a rota nunca aponta para um controller que
ainda não existe. Agora transforme isso num blog customizando o que o scaffold produziu.

### Customizar o model

Abra `app/models/post.rs` e adicione uma pequena validação para rejeitar posts em branco. O trait
[`Validate`](@/docs/reference/models.pt.md) do Doido acumula os erros — o resto do arquivo é
exatamente o que o scaffold gerou:

```rust
// app/models/post.rs
#![allow(dead_code)]

use doido::model::sea_orm;
use doido::model::sea_orm::entity::prelude::*;
use doido::model::validation::{Errors, Validate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
    pub user_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Validate for Model {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e.length("body", &self.body, Some(10), None); // pelo menos 10 caracteres
        e
    }
}
```

Mantemos `Relation` vazio e consultamos os comentários com um filtro explícito abaixo, o que
evita o descasamento entre a chave primária `i32` e a estrangeira `i64` que um `has_many` ingênuo
enfrentaria.

### Customizar o controller

O controller do scaffold expõe as sete actions REST. Reescreva
`app/controllers/posts_controller.rs` para que a leitura seja pública, a autoria seja protegida
por login e cada novo post pertença ao autor logado:

```rust
// app/controllers/posts_controller.rs
use crate::helpers::PostsHelper;
use crate::models::{comment, post};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use doido::model::serialization::as_json;
use serde::Deserialize;
use serde_json::json;

/// Strong params para criar/atualizar um post. O autor (`user_id`) vem da
/// sessão, nunca do formulário.
#[derive(Deserialize)]
pub struct PostForm {
    pub title: String,
    pub body: String,
    pub published: Option<String>,
}

pub struct PostsController;

/// Interrompe e redireciona para o sign-in a menos que alguém esteja logado. O
/// sign-in guarda o id do usuário na sessão sob "user_id" (veja o doido-auth).
async fn require_login(ctx: &mut Context) -> Result<(), Response> {
    if ctx.session().get::<i64>("user_id").is_none() {
        return Err(ctx.redirect_to("/users/sign_in"));
    }
    Ok(())
}

#[controller]
impl PostsController {
    /// GET /posts — público: só os posts publicados.
    pub async fn index(ctx: Context) -> doido::Result<Response> {
        let posts = post::Entity::find()
            .filter(post::Column::Published.eq(true))
            .all(ctx.db())
            .await?;
        Ok(ctx.render(
            "posts/index",
            json!({
                "posts": as_json(&posts),
                "summary": PostsHelper::index_count(posts.len()),
            }),
        ))
    }

    /// GET /posts/{id} — público: o post e seus comentários.
    pub async fn show(ctx: Context) -> doido::Result<Response> {
        let id = parse_id(&ctx);
        let Some(post) = post::Entity::find_by_id(id).one(ctx.db()).await? else {
            return Ok(ctx.status(404));
        };
        let comments = comment::Entity::find()
            .filter(comment::Column::PostId.eq(i64::from(post.id)))
            .all(ctx.db())
            .await?;
        Ok(ctx.render(
            "posts/show",
            json!({ "post": as_json(&post), "comments": as_json(&comments) }),
        ))
    }

    /// GET /posts/new — a autoria é protegida por login.
    #[before_action(require_login)]
    pub async fn new(ctx: Context) -> Response {
        ctx.render("posts/new", json!({}))
    }

    /// POST /posts — cria um post pertencente ao autor logado.
    #[before_action(require_login)]
    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let author_id = ctx.session().get::<i64>("user_id").unwrap();
        let form: PostForm = ctx.form().await?;
        let record = post::ActiveModel {
            title: Set(form.title),
            body: Set(form.body),
            published: Set(form.published.is_some()),
            user_id: Set(author_id),
            ..Default::default()
        };
        record.insert(ctx.db()).await?;
        Ok(ctx.redirect_to("/posts"))
    }

    /// GET /posts/{id}/edit — protegido por login.
    #[before_action(require_login)]
    pub async fn edit(ctx: Context) -> doido::Result<Response> {
        let id = parse_id(&ctx);
        let post = post::Entity::find_by_id(id).one(ctx.db()).await?;
        Ok(ctx.render("posts/edit", json!({ "post": as_json(&post) })))
    }

    /// PATCH/PUT /posts/{id} — protegido por login.
    #[before_action(require_login)]
    pub async fn update(mut ctx: Context) -> doido::Result<Response> {
        let id = parse_id(&ctx);
        let form: PostForm = ctx.form().await?;
        if let Some(existing) = post::Entity::find_by_id(id).one(ctx.db()).await? {
            let mut record: post::ActiveModel = existing.into();
            record.title = Set(form.title);
            record.body = Set(form.body);
            record.published = Set(form.published.is_some());
            record.update(ctx.db()).await?;
        }
        Ok(ctx.redirect_to("/posts"))
    }

    /// DELETE /posts/{id} — protegido por login.
    #[before_action(require_login)]
    pub async fn destroy(ctx: Context) -> doido::Result<Response> {
        let id = parse_id(&ctx);
        post::Entity::delete_by_id(id).exec(ctx.db()).await?;
        Ok(ctx.redirect_to("/posts"))
    }
}

fn parse_id(ctx: &Context) -> i32 {
    ctx.param("id").and_then(|v| v.parse().ok()).unwrap_or_default()
}
```

O `#[before_action(require_login)]` roda o guard antes da action; retornar `Err(response)`
interrompe a requisição. `PostsHelper` é o helper que o scaffold gerou junto com o controller.

### Customizar as views

As views do scaffold [estendem](@/docs/reference/views.pt.md) o layout gerado
`app/views/layouts/application.html.tera`, que renderiza o conteúdo com
`{% block content %}{% endblock %}`. O JSON que você passa para `ctx.render` vira o contexto do
template. Substitua os templates de index e show por uma marcação com cara de blog (deixe `new`,
`edit` e `_form` como o scaffold os escreveu):

```html
{# app/views/posts/index.html.tera #}
{% extends "layouts/application.html.tera" %}
{% block content %}
<h1>Blog</h1>
<p>{{ summary }}</p>
{% for post in posts %}
  <article>
    <h2><a href="/posts/{{ post.id }}">{{ post.title }}</a></h2>
  </article>
{% endfor %}
{% endblock %}
```

```html
{# app/views/posts/show.html.tera #}
{% extends "layouts/application.html.tera" %}
{% block content %}
<article>
  <h1>{{ post.title }}</h1>
  <p>{{ post.body }}</p>
</article>
<section>
  <h2>Comentários</h2>
  {% for comment in comments %}
    <p><strong>{{ comment.author_name }}</strong>: {{ comment.body }}</p>
  {% endfor %}
  <form method="post" action="/posts/{{ post.id }}/comments">
    <input type="text" name="author_name" required>
    <textarea name="body" required></textarea>
    <button type="submit">Comentar</button>
  </form>
</section>
{% endblock %}
```

## O model Comment

Um comentário pertence a um post e carrega o nome e a mensagem do leitor. Não é preciso login
para comentar, então guardamos um nome livre. Um comentário não tem telas de CRUD próprias, então
um gerador `model` simples basta:

```bash
cargo doido generate model Comment \
  post:references \
  author_name:string:not_null \
  body:text:not_null
cargo doido db migrate
```

O `app/models/comment.rs` gerado (um `post_id` `i64`, `author_name`, `body` e um `Relation`
vazio) não precisa de mudanças — o `show` acima já carrega os comentários de um post com um filtro
explícito por `comment::Column::PostId`.

## O controller de comentários

Comentários precisam de uma única action — create — então use o **gerador de controller**:

```bash
cargo doido generate controller Comments
```

Isso escreveu `app/controllers/comments_controller.rs` (um stub `index` conectado ao
`CommentsHelper`) e injetou `get!("/comments", CommentsController::index);` nas rotas. Adicione
uma action `create` que lê o formulário e insere um comentário para o post da URL:

```rust
// app/controllers/comments_controller.rs
use crate::helpers::CommentsHelper;
use crate::models::comment;
use doido::controller::{controller, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct CommentForm {
    pub author_name: String,
    pub body: String,
}

pub struct CommentsController;

#[controller]
impl CommentsController {
    /// GET /comments — o stub do gerador, mantido para o `CommentsHelper` continuar conectado.
    pub async fn index(ctx: doido::controller::Context) -> Response {
        ctx.json(json!({ "comments": CommentsHelper::index_count(0) }))
    }

    /// POST /posts/{post_id}/comments — não exige login para comentar.
    pub async fn create(mut ctx: doido::controller::Context) -> doido::Result<Response> {
        let post_id: i64 = ctx
            .param("post_id")
            .and_then(|v| v.parse().ok())
            .unwrap_or_default();
        let form: CommentForm = ctx.form().await?;
        let record = comment::ActiveModel {
            post_id: Set(post_id),
            author_name: Set(form.author_name),
            body: Set(form.body),
            ..Default::default()
        };
        record.insert(ctx.db()).await?;
        Ok(ctx.redirect_to(format!("/posts/{post_id}")))
    }
}
```

## Rotas

Como os geradores injetaram uma rota com cada controller, o `config/routes.rs` já conhece o
`PostsController` e o `CommentsController` — nenhuma rota nomeia um controller que não exista. Duas
edições concluem a ligação: aponte a página inicial para o blog (movendo o endpoint de demonstração
do Doido para o lado) e adicione a rota aninhada de comentário ao lado do stub que o gerador de
controller deixou:

```rust
// config/routes.rs
use crate::controllers::CommentsController;
use crate::controllers::HelloController;
use crate::controllers::PostsController;
use doido::controller::{axum, routes};

pub fn router() -> axum::Router {
    routes! {
        root!(PostsController::index);                              // GET / — o blog
        get!("/hello", HelloController::index);                    // demo do Doido, movida para o lado

        resources!(posts, PostsController);                        // CRUD de /posts (do scaffold)
        get!("/comments", CommentsController::index);              // stub do `generate controller`
        post!("/posts/{post_id}/comments", CommentsController::create);

        // as rotas sign-in / sign-up / sign-out sob /users foram injetadas pelo --auth — deixe-as.
    }
}
```

Repare no parâmetro de caminho no estilo axum `{post_id}` — leia-o na action com
`ctx.param("post_id")`. Veja [Controllers & roteamento](@/docs/reference/controllers.pt.md) para
a DSL completa.

## Rodar

```bash
cargo doido server
```

Agora percorra o fluxo inteiro:

1. Visite `/users/sign_up` e registre a conta do autor.
2. Vá para `/posts/new`, escreva um post e publique.
3. Abra `/` — o post publicado aparece. Clique até `/posts/{id}`.
4. Deixe um comentário; ele aparece sob o post.

Fazer sign-out (`DELETE /users/sign_out`) e revisitar `/posts/new` te joga de volta para a
página de sign-in — é o `require_login` fazendo seu trabalho.

## Testes

Os geradores deixam stubs de teste (`tests/post_model_test.rs`, `tests/posts_controller_test.rs`);
rode-os com `cargo test`. Validações são lógica pura, então não precisam de banco:

```rust
// tests/post_validation_test.rs
#[path = "../app/models/mod.rs"]
mod models;

use doido::model::validation::Validate;
use models::post::Model;

#[test]
fn rejects_a_blank_post() {
    let post = Model {
        id: 0,
        title: String::new(),      // faltando
        body: "too short".into(),  // < 10 caracteres
        published: false,
        user_id: 1,
    };

    assert!(!post.is_valid());
    // post.validate().full_messages() lista os erros legíveis.
}
```

Para testes de requisição e de auth (montar o router real, dirigir uma sessão logada), veja a
[referência de Testes](@/docs/reference/auth.pt.md#testes). O fluxo completo acima — os comandos
de gerador exatos mais estas customizações — também roda como um e2e de release
(`blog_tutorial`), então este tutorial não apodrece em silêncio.

## Próximos passos

- **[Geradores & CLI](@/docs/reference/generators.pt.md)** — cada gerador, incluindo `scaffold`
  e `resource`, e o padrão de tutoriais que estes passos seguem.
- **[Models](@/docs/reference/models.pt.md)** — associações, migrations, validações e factories.
- **[Controllers & roteamento](@/docs/reference/controllers.pt.md)** — filtros, strong parameters e a DSL `routes!`.
- **[Views](@/docs/reference/views.pt.md)** — layouts, partials e helpers de view.
- **[Auth](@/docs/reference/auth.pt.md)** — sessões, JWT, OAuth, 2FA e os extractors.
