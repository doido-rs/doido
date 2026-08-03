# doido-generators — Spec

Rails analogue: **rails generate** (`rails generate model`, `rails generate scaffold`, etc.)

## Decisions (resolved in interview)

- **Separate crate** from `doido-cli` — independently usable, testable, and extensible
- **All Rails generator targets ship in v1**
- **Extensible registry** — apps and plugins register custom generators
- **Route auto-injection** — appends to `config/routes.rs` when relevant
- **Optional crate generators** — generators owned by optional workspace crates
  (e.g. `doido-auth`) register via that crate's `generators::register()` and appear
  in `doido generate` **only when** the current project's `Cargo.toml` lists the
  crate as a dependency. They are never part of `default_registry()`.

## Responsibility

`doido-generators` owns all code generation logic. `doido-cli` is just a thin dispatcher.

## Module Structure

```
doido-generators/
  src/
    lib.rs
    registry.rs         ← GeneratorRegistry + Generator trait
    args.rs             ← GeneratorArgs, FieldDef, FileAction types
    route_injector.rs   ← parses config/routes.rs and appends route entries
    generators/
      model.rs
      controller.rs
      migration.rs
      scaffold.rs
      resource.rs       ← scaffold without views
      mailer.rs
      job.rs
      channel.rs
    templates/          ← embedded Tera templates for generated file content
      model.rs.tera
      controller.rs.tera
      migration.rs.tera
      views/
        index.html.tera
        show.html.tera
        new.html.tera
        edit.html.tera
      mailer.rs.tera
      job.rs.tera
      channel.rs.tera
```

## `Generator` Trait (extensible)

```rust
pub trait Generator: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn generate(&self, args: &GeneratorArgs) -> Result<Vec<GeneratedFile>>;
}

pub struct GeneratorArgs {
    pub name:    String,                       // e.g. "Post"
    pub fields:  Vec<FieldDef>,                // e.g. [("title", "String")]
    pub actions: Vec<String>,                  // for controller generator
    pub options: HashMap<String, String>,      // --option=value flags
}

pub struct FieldDef {
    pub name:      String,
    pub field_type: String,   // "String" | "i64" | "bool" | "DateTime" | etc.
    pub nullable:  bool,
}

pub struct GeneratedFile {
    pub path:    PathBuf,
    pub content: String,
    pub action:  FileAction,  // Create | Skip | Overwrite
}
```

## `GeneratorRegistry`

```rust
// Built-in generators registered automatically
// Apps add custom generators at boot:
doido_generators::registry().register(Box::new(MyGenerator));

// List all
doido_generators::registry().list();  // → Vec<(&str, &str)>  (name, description)

// Dispatch (built-in + project-local + optional crate generators when installed)
doido_generators::dispatch("scaffold", args)?;
```

### Optional crate generators (`doido-auth`, …)

Some generators live in optional first-party crates rather than in
`doido-generators`. The CLI builds the effective registry per project:

1. Start with `default_registry()` (built-in generators only).
2. Parse the project's `Cargo.toml` for optional crate deps (e.g. `doido-auth`, or
   `doido` with feature `auth`).
3. When present, call `<crate>::generators::register(&mut reg)` to merge crate-owned
   generators into the registry for this invocation.
4. When absent, those generators are **not listed** and **not dispatchable**.

```rust
// doido-generators/src/commands/generate.rs (target behaviour)
fn registry_for_project() -> GeneratorRegistry {
    let mut reg = default_registry();
    if project_has_doido_auth("Cargo.toml") {
        doido_auth::generators::register(&mut reg);
    }
    reg
}
```

Auth generators (`auth:install`, `auth:controller`, `auth:scaffold`) are specified
in [16-auth.md](16-auth.md). See that doc for bootstrap via `doido new --auth` or
`cargo add doido-auth`.

## Built-in Generators (v1)

| Generator | Files Created | Route Injected |
|-----------|--------------|----------------|
| `model` | `models/<name>.rs`, migration | No |
| `controller` | `controllers/<name>_controller.rs`, view stubs | Yes |
| `migration` | `db/migrations/<timestamp>_<name>.rs` | No |
| `scaffold` | model + migration + controller + all views | Yes — `resources!(...)` |
| `resource` | model + migration + controller (no views) | Yes — `resources!(...)` |
| `mailer` | `mailers/<name>_mailer.rs`, view templates | No |
| `job` | `jobs/<name>_job.rs` | No |
| `channel` | `channels/<name>_channel.rs` | No (prints hint to add `cable!(...)` manually) |
| `storage:install` | storage tables migration + `storage:` config | No |
| `storage:adapter` | `app/storage/<name>_service.rs` | No |

Auth generators (`auth:install`, `auth:controller`, `auth:scaffold`) are **not**
built-in — they live in `doido-auth` and appear only when that crate is a project
dependency. See [16-auth.md](16-auth.md).

## Field Specs (`model`, `scaffold`, `resource`)

After the name, pass any number of `name:type[:modifier...]` specs. The `model`
generator turns each spec into both a migration column and a SeaORM model field;
the implicit auto-incrementing `id` primary key is always added for you.

```sh
doido generate model Post \
  title:string:not_null body:text author:references \
  slug:string:unique views:integer:index
```

```rust
// db/migration/src/m..._create_posts_table.rs
create_table(manager, "posts", |t| {
    t.string("title").not_null();
    t.text("body");
    t.references("author");          // adds `author_id` (NOT NULL)
    t.string("slug").unique_key();
    t.integer("views");
})
.await?;
add_index(manager, "posts", &["views"]).await?;
Ok(())

// app/models/post.rs
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,            // NOT NULL → bare type
    pub body: Option<String>,     // nullable → Option<T>
    pub author_id: i64,
    pub slug: Option<String>,
    pub views: Option<i32>,
}
```

- **Type** (default `string`): `string`, `text`, `integer`/`int`, `bigint`,
  `float`, `double`, `decimal`, `boolean`/`bool`, `timestamp`/`datetime`,
  `date`, `json`/`jsonb`, `uuid`, `binary`, `references`/`belongs_to`.
- **Modifiers**: `not_null` (column is required; otherwise the model field is
  `Option<T>`), `unique`, `index`. `references` columns get an `_id` suffix and
  are always NOT NULL.
- Unknown types or modifiers are a hard error so typos surface immediately.

## Scaffold (`scaffold`)

`scaffold` runs the `model` generator and adds a full RESTful controller, views,
and route wiring — a complete CRUD resource from one command:

```sh
doido generate scaffold Post title:string:not_null body:text author:references
doido generate scaffold Post title:string --api      # JSON API, no views
```

Produces:
- `app/models/post.rs` + migration (via the `model` generator), registered in
  `app/models/mod.rs`.
- `app/controllers/posts_controller.rs` — a `#[controller]` with all 7 actions
  (`index, show, new, create, edit, update, destroy`) performing real sea-orm
  persistence through `Context::db()`, plus a `PostForm` strong-params struct
  derived from the field specs. Registered in `app/controllers/mod.rs`.
- HTML mode: `app/views/posts/{index,show,new,edit,_form}.html.tera`, with table
  columns and form inputs derived from the fields. `--api` skips views and the
  actions return `ctx.json(...)`.
- `resources!(posts, PostsController);` injected into `config/routes.rs`
  (existing routes preserved).

Controller actions return `doido::Result<Response>`, so they use `?` for
fallible work (DB calls, body parsing); the `#[controller]` macro maps an `Err`
to a `500`. Request data is read via `ctx.param("id")`, `ctx.form::<T>()`, and
`ctx.body_json::<T>()`. The global DB pool is installed at server boot
(`doido_model::pool::init`).

## Auth generators (owned by `doido-auth`, not built-in)

Auth generators are implemented in `doido-auth/src/generators/` and registered via
`doido_auth::generators::register()`. They are visible to `doido generate` **if and
only if** the project's `Cargo.toml` lists `doido-auth` (directly or via `doido`
with feature `auth`). Full spec: [16-auth.md](16-auth.md).

Bootstrap for new apps: `doido new --auth` adds the dependency and runs
`auth:install`. For existing apps: `cargo add doido-auth`, then
`doido generate auth:install`.

## Route Auto-Injection into `config/routes.rs`

```rust
// Before
routes! {
    get!("/", HomeController::index);
}

// After `doido generate scaffold Post title:String`
routes! {
    get!("/", HomeController::index);
    resources!(posts, PostsController);   // ← injected
}
```

Injection rules:
- Finds the `routes! { ... }` block via text parsing
- Appends before the closing `}`
- Skips injection if the controller is already present (prints warning)
- Creates `config/routes.rs` with minimal scaffold if it does not exist

## Conflict Resolution (interactive)

When a file already exists, prompts:
```
conflict  src/controllers/posts_controller.rs
Overwrite? [Y]es / [N]o / [A]ll / [Q]uit
```

With `--force` flag, overwrites all without prompting.  
With `--dry-run` flag, prints files without writing anything.

## Known Requirements

- All generator output is **deterministic** given the same args (required for TDD)
- Templates embedded in the binary via `include_str!` — no runtime template files needed
- Field type mapping: `String→Text`, `i64→BigInteger`, `bool→Boolean`, `DateTime→DateTime`
- `doido-generators` has zero dependency on `doido-cli`

## TDD Surface

- Test each generator produces expected file content for given args
- Test `scaffold` creates all expected files
- Test `resource` creates all expected files except views
- Test route injection appends correct entry to `config/routes.rs`
- Test route injection skips when controller already registered
- Test route injection creates file when `config/routes.rs` missing
- Test `--dry-run` returns files without writing to disk
- Test `--force` overwrites without prompting
- Test custom generator registered and dispatched via registry
- Test field type mapping for all supported types
- Test `project_has_doido_auth` — auth generators listed only when `doido-auth` is in Cargo.toml
- Integration test: generate scaffold → `cargo check` compiles without errors
