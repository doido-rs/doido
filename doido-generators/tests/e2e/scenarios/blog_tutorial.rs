//! "Building a blog" tutorial, executed end to end.
//!
//! This scenario is the executable mirror of
//! `docs-blog/content/docs/tutorials/building-a-blog.md`: it runs the same
//! generator command script (`scaffold Post`, `model Comment`,
//! `controller Comments`), applies the same customizations the tutorial asks the
//! reader to make, then exercises the running app over HTTP — public reading,
//! authenticated authoring (post ownership), and reader comments.
//!
//! Keep the embedded Rust/Tera below in sync with the tutorial snippets: if one
//! changes, the other must change too (see `docs/06b-generators.md`).

use crate::common::http;
use crate::common::{db, AppHarness, BaseProfile};
use std::fs;
use std::path::Path;

// --- Tutorial customizations (identical to the doc's code blocks) ----------

const POST_MODEL: &str = r#"#![allow(dead_code)]

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
        e.length("body", &self.body, Some(10), None);
        e
    }
}
"#;

const COMMENT_MODEL: &str = r#"#![allow(dead_code)]

use doido::model::sea_orm;
use doido::model::sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "comments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub post_id: i64,
    pub author_name: String,
    pub body: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
"#;

const POSTS_CONTROLLER: &str = r#"use crate::helpers::PostsHelper;
use crate::models::{comment, post};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use doido::model::serialization::as_json;
use serde::Deserialize;
use serde_json::json;

/// Strong params for creating/updating a post. The author (`user_id`) comes
/// from the session, never the form.
#[derive(Deserialize)]
pub struct PostForm {
    pub title: String,
    pub body: String,
    pub published: Option<String>,
}

pub struct PostsController;

/// Halts and redirects to sign-in unless someone is signed in. Sign-in stores
/// the user's id in the session under "user_id" (see doido-auth).
async fn require_login(ctx: &mut Context) -> Result<(), Response> {
    if ctx.session().get::<i64>("user_id").is_none() {
        return Err(ctx.redirect_to("/users/sign_in"));
    }
    Ok(())
}

#[controller]
impl PostsController {
    /// GET /posts — public: only published posts.
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

    /// GET /posts/{id} — public: the post and its comments.
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

    /// GET /posts/new — authoring is login-guarded.
    #[before_action(require_login)]
    pub async fn new(ctx: Context) -> Response {
        ctx.render("posts/new", json!({}))
    }

    /// POST /posts — creates a post owned by the signed-in author.
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

    /// GET /posts/{id}/edit — login-guarded.
    #[before_action(require_login)]
    pub async fn edit(ctx: Context) -> doido::Result<Response> {
        let id = parse_id(&ctx);
        let post = post::Entity::find_by_id(id).one(ctx.db()).await?;
        Ok(ctx.render("posts/edit", json!({ "post": as_json(&post) })))
    }

    /// PATCH/PUT /posts/{id} — login-guarded.
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

    /// DELETE /posts/{id} — login-guarded.
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
"#;

const COMMENTS_CONTROLLER: &str = r#"use crate::helpers::CommentsHelper;
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
    /// GET /comments — the generator's stub, kept so `CommentsHelper` stays wired.
    pub async fn index(ctx: doido::controller::Context) -> Response {
        ctx.json(json!({ "comments": CommentsHelper::index_count(0) }))
    }

    /// POST /posts/{post_id}/comments — no login required to comment.
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
"#;

const INDEX_VIEW: &str = r#"{% extends "layouts/application.html.tera" %}
{% block content %}
<h1>Blog</h1>
<p>{{ summary }}</p>
{% for post in posts %}
  <article>
    <h2><a href="/posts/{{ post.id }}">{{ post.title }}</a></h2>
  </article>
{% endfor %}
{% endblock %}
"#;

const SHOW_VIEW: &str = r#"{% extends "layouts/application.html.tera" %}
{% block content %}
<article>
  <h1>{{ post.title }}</h1>
  <p>{{ post.body }}</p>
</article>
<section>
  <h2>Comments</h2>
  {% for comment in comments %}
    <p><strong>{{ comment.author_name }}</strong>: {{ comment.body }}</p>
  {% endfor %}
  <form method="post" action="/posts/{{ post.id }}/comments">
    <input type="text" name="author_name" required>
    <textarea name="body" required></textarea>
    <button type="submit">Post comment</button>
  </form>
</section>
{% endblock %}
"#;

fn write(app: &Path, rel: &str, contents: &str) {
    fs::write(app.join(rel), contents).unwrap_or_else(|e| panic!("write {rel}: {e}"));
}

/// Applies a single required substitution, panicking (with the file dumped) if
/// the needle is missing — so a drift in generated output fails loudly.
fn replace_once(haystack: &str, needle: &str, replacement: &str) -> String {
    assert!(
        haystack.contains(needle),
        "expected to find `{needle}` in:\n{haystack}"
    );
    haystack.replacen(needle, replacement, 1)
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn blog_tutorial_scaffold_controller_and_serve() {
    let h = AppHarness::new("blog_tutorial", BaseProfile::WithAuthHtml);

    // 1. Scaffold the Post resource (model + migration + controller + views +
    //    `resources!(posts, PostsController)`), then the Comment model, then the
    //    Comments controller (which injects `get!("/comments", ...)`).
    h.generate(&[
        "generate",
        "scaffold",
        "Post",
        "title:string:not_null",
        "body:text:not_null",
        "published:boolean:not_null",
        "user:references",
    ]);
    h.generate(&[
        "generate",
        "model",
        "Comment",
        "post:references",
        "author_name:string:not_null",
        "body:text:not_null",
    ]);
    h.generate(&["generate", "controller", "Comments"]);

    // 2. Apply the tutorial's customizations.
    write(&h.app, "app/models/post.rs", POST_MODEL);
    write(&h.app, "app/models/comment.rs", COMMENT_MODEL);
    write(
        &h.app,
        "app/controllers/posts_controller.rs",
        POSTS_CONTROLLER,
    );
    write(
        &h.app,
        "app/controllers/comments_controller.rs",
        COMMENTS_CONTROLLER,
    );
    write(&h.app, "app/views/posts/index.html.tera", INDEX_VIEW);
    write(&h.app, "app/views/posts/show.html.tera", SHOW_VIEW);

    // 3. Adjust routes *after* the controllers exist. The scaffold already injected
    //    `resources!(posts, PostsController)` and the controller generator injected
    //    `get!("/comments", ...)`, so every controller a route names is already
    //    present. Point the homepage at the blog (moving Doido's demo to /hello) and
    //    add the RESTful nested comment route.
    let routes_path = h.app.join("config/routes.rs");
    let routes = fs::read_to_string(&routes_path).expect("read config/routes.rs");
    let routes = replace_once(
        &routes,
        "get!(\"/\", HelloController::index);",
        "root!(PostsController::index);\n        get!(\"/hello\", HelloController::index);",
    );
    let routes = replace_once(
        &routes,
        "get!(\"/comments\", CommentsController::index);",
        "get!(\"/comments\", CommentsController::index);\n        post!(\"/posts/{post_id}/comments\", CommentsController::create);",
    );
    fs::write(&routes_path, routes).expect("write config/routes.rs");

    // 4. Build, migrate, boot, and exercise the app end to end.
    h.run_with_db(
        |h| {
            db::assert_table_exists(&h.app, "posts");
            db::assert_table_exists(&h.app, "comments");
            db::assert_column_exists(&h.app, "posts", "user_id");
            db::assert_column_exists(&h.app, "posts", "published");
            db::assert_column_exists(&h.app, "comments", "post_id");
        },
        |app| {
            let base = &app.base_url;

            // The blog homepage boots empty (no published posts yet).
            let empty = http::get_text(&format!("{base}/"));
            assert!(
                empty.contains("Blog"),
                "homepage should render the blog heading"
            );

            // Register + sign in the author, capturing the session cookie.
            let sign_up = http::post_form_with_response(
                &format!("{base}/users/sign_up"),
                &[
                    ("email", "author@example.com"),
                    ("password", "secret123"),
                    ("password_confirmation", "secret123"),
                ],
            );
            assert!(
                (300..400).contains(&sign_up.status),
                "sign up should redirect, got {}",
                sign_up.status
            );
            let cookie = http::session_cookie(&sign_up.set_cookie);
            assert!(
                cookie.contains("_doido_session"),
                "sign up should establish a session"
            );

            // Unauthenticated post creation is blocked by the login guard.
            let anon = http::post_form(
                &format!("{base}/posts"),
                &[("title", "Sneaky"), ("body", "no session here")],
            );
            assert!(
                (300..400).contains(&anon),
                "anonymous create should redirect to sign-in, got {anon}"
            );

            // Authenticated author publishes a post (owned via the session).
            let created = http::post_form_with_cookie(
                &format!("{base}/posts"),
                &[
                    ("title", "My First Post"),
                    ("body", "Hello, world — this is the body."),
                    ("published", "1"),
                ],
                &cookie,
            );
            assert!(
                (300..400).contains(&created.status),
                "authenticated create should redirect, got {}",
                created.status
            );

            // The published post now shows up on the public homepage and its page.
            let index = http::get_text(&format!("{base}/"));
            assert!(
                index.contains("My First Post"),
                "homepage should list the post"
            );

            let show = http::get_text(&format!("{base}/posts/1"));
            assert!(
                show.contains("My First Post"),
                "show should render the post"
            );
            assert!(
                show.contains("Hello, world"),
                "show should render the post body"
            );

            // A reader comments (no login) and the comment appears on the page.
            let commented = http::post_form(
                &format!("{base}/posts/1/comments"),
                &[("author_name", "Reader"), ("body", "Nice post!")],
            );
            assert!(
                (300..400).contains(&commented),
                "comment create should redirect, got {commented}"
            );

            let with_comment = http::get_text(&format!("{base}/posts/1"));
            assert!(
                with_comment.contains("Reader"),
                "show should render the comment author"
            );
            assert!(
                with_comment.contains("Nice post!"),
                "show should render the comment body"
            );
        },
    );
}
