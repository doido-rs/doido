+++
title = "Controllers & roteamento"
description = "Defina rotas, escreva controllers e use filtros e o Context da requisição."
weight = 1
+++

> **Especificação de design:** [`docs/01-router.md`](https://github.com/doido-rs/doido/blob/master/docs/01-router.md)
> e [`docs/02-controller.md`](https://github.com/doido-rs/doido/blob/master/docs/02-controller.md).
> Este guia é o companheiro focado em uso dessas especificações.

A camada de requisição do Doido mapeia de forma limpa para o Rails: um **router**
despacha URLs para **actions de controller**, e cada action recebe um `Context`
tipado e retorna um `Response`. Por baixo é construído sobre `axum::Router`, mas
você trabalha através da macro `routes!` em vez do axum cru.

## Roteamento

As rotas são declaradas com a macro `routes!` em `config/routes.rs`:

```rust
routes! {
    resources!(posts, PostsController);
    resources!(comments, CommentsController, only: [index, show]);
    resources!(admin, AdminController, except: [destroy]);

    get!("/about", PagesController::about);
    post!("/login", SessionsController::create);

    namespace!(api, {
        resources!(users, Api::UsersController);
    });

    scope!("/v2", {
        resources!(articles, V2::ArticlesController);
    });
}
```

- `namespace!` prefixa **tanto** o path **quanto** o caminho do módulo do controller.
- `scope!` prefixa **apenas** o path.
- Verbos suportados: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`.

### As 7 rotas REST

`resources!(posts, PostsController)` gera todas as sete rotas RESTful, cada uma
com um helper de URL em tempo de compilação:

| Helper | Método | Path | Action |
|--------|--------|------|--------|
| `posts_path()` | GET | `/posts` | index |
| `new_post_path()` | GET | `/posts/new` | new |
| `post_path(id)` | GET | `/posts/:id` | show |
| `post_path(id)` | POST | `/posts` | create |
| `edit_post_path(id)` | GET | `/posts/:id/edit` | edit |
| `post_path(id)` | PATCH | `/posts/:id` | update |
| `post_path(id)` | DELETE | `/posts/:id` | destroy |

Use `only:` / `except:` para restringir quais das sete são geradas.

## Controllers

Um controller é uma struct anotada com `#[controller]`; as actions são `async fn`
comuns que recebem um `Context` e retornam um `Response`. A rota despacha para a
action cujo nome de método corresponde à action (convenção sobre configuração).

```rust
#[controller]
struct PostsController;

impl PostsController {
    #[before_action(authenticate)]
    #[before_action(find_post, only = [show, edit, update, destroy])]
    async fn index(ctx: Context) -> Response {
        let posts = Post::all(&ctx.db).await?;
        ctx.render("posts/index", json!({ "posts": posts }))
    }

    #[before_action(authenticate)]
    #[after_action(log_response)]
    async fn create(ctx: Context) -> Response {
        let params = ctx.params::<CreatePostParams>()?;
        match Post::create(&ctx.db, params).await {
            Ok(post) => ctx.redirect_to(post_path(post.id)),
            Err(_)   => ctx.render("posts/new", status = 422),
        }
    }
}
```

## O `Context` da requisição

Tudo o que uma action precisa está no `ctx`:

```rust
ctx.params::<T>()          // params tipados (path + query + body) via serde
ctx.db                     // handle da conexão com o banco
ctx.session                // acesso ao store de sessão
ctx.render(template, data) // renderiza uma view (delega para doido-view)
ctx.redirect_to(path)      // helper de redirect 302
ctx.json(data)             // helper de resposta JSON
ctx.status(code)           // define o status da resposta
```

## Duas formas de filtrar

O Doido oferece dois mecanismos de filtro complementares:

1. **Filtros por macro de atributo (nível de action).** `#[before_action(fn)]` e
   `#[after_action(fn)]` no controller. Restrinja-os com
   `only = [action1, action2]`. Um `before_action` tem a assinatura
   `async fn(ctx: &mut Context) -> Result<(), Response>`; retornar `Err(response)`
   interrompe a cadeia e retorna cedo — o equivalente ao `render`-e-retorna de um
   filtro no Rails.

2. **Camadas de middleware Tower (nível de router).** Aplicadas via a DSL
   `routes!` ou o `.layer()` do axum, elas cobrem preocupações transversais
   (autenticação, rate limiting, request IDs, CORS) em um controller ou namespace
   inteiro. O middleware roda **antes** dos filtros por macro de atributo.

## Testes

A camada de controller foi feita para ser testada sem um servidor HTTP: construa
um `Context` diretamente e chame a action, verificando o `Response` retornado.
Para cobertura de ponta a ponta, monte um bloco `routes!` e dirija-o com o cliente
de teste. Veja a superfície de TDD nas especificações para a matriz de testes
exata.
