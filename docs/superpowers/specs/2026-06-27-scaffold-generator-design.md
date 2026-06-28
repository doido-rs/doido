# Scaffold Generator + Context DB Access — Design

Date: 2026-06-27
Status: implemented

## Summary

Turn the placeholder `scaffold` generator into a real Rails-style resource
generator that emits, from one command, a working model + migration + a 7-action
RESTful controller + views, wired into routes and module trees. Because the
controller actions must perform real persistence, this also adds the
prerequisite framework plumbing: a database handle reachable from `Context`,
path-param and request-body access, and `Result`-returning actions.

```sh
doido generate scaffold Post title:string body:text published:boolean author:references
doido generate scaffold Post title:string ... --api   # JSON instead of HTML views
```

## Motivation / current gaps

The existing `scaffold.rs` only stitches `ControllerGenerator` (an `index`-only
stub) + `ModelGenerator` + a route *marker* comment. It is non-functional:

1. `resources!(...)` expands to all 7 actions (`index, show, new, create, edit,
   update, destroy`) — a scaffolded controller with only `index` will not compile.
2. No view templates exist.
3. The "route injection" writes a `config/routes.rs` containing only a marker
   comment; `write_files` overwrites, so it **erases** the real routes file.
4. The controller is never registered in `app/controllers/mod.rs`, and the
   generated app never mounts an `app/models/mod.rs` at all.
5. `Context` has no DB handle, no path params, no body parsing; `render` is a
   stub. Real CRUD is impossible without these.

## Decisions (confirmed)

- **HTML + JSON**, selected by a `--api` flag (default HTML with views).
- **Real persistence** via sea-orm — add DB access to `Context` first.
- **Global connection pool** (`OnceLock<DatabaseConnection>` in `doido-model`)
  rather than axum `State`, because the controller macro builds `Context` purely
  from the request and threading state would require macro + router changes.
- **Actions return `Result<Response>`** (as well as plain `Response`), enabled by
  a small `IntoActionResponse` trait the macro wraps the body in — backward
  compatible, no return-type parsing.

## Architecture

### 1. doido-model — global pool (`src/pool.rs`)
```rust
static POOL: OnceLock<DatabaseConnection> = OnceLock::new();
pub async fn init() -> Result<&'static DatabaseConnection, DbErr>; // connect() + store
pub fn pool() -> &'static DatabaseConnection;                      // panics if uninit
pub fn try_pool() -> Option<&'static DatabaseConnection>;
pub fn set_pool(conn) -> Result<(), DatabaseConnection>;           // for tests
```
Initialized at server boot (`commands/server.rs`) before the router starts.

### 2. doido-controller — `Context` (depends on doido-model; acyclic)
- `Context::build(req).await` — central constructor: splits the request, extracts
  raw path params (via `RawPathParams`), stores `parts`, `body: Option<Body>`,
  and `path_params: Vec<(String,String)>`. The macro calls this.
- `db(&self) -> &'static DatabaseConnection` → `doido_model::pool::pool()`.
- `param(&self, name) -> Option<&str>` — matched path segment (e.g. `id`).
- `form::<T>(&mut self).await` / `body_json::<T>(&mut self).await` — consume the
  body and deserialize (urlencoded / JSON).
- `from_request` / `from_request_parts` retained (empty path params) for tests.

### 3. doido-controller macros — `IntoActionResponse`
```rust
pub trait IntoActionResponse { fn into_action_response(self) -> Response; }
impl IntoActionResponse for Response { ... identity }
impl<E: Display> IntoActionResponse for Result<Response, E> { Ok→r, Err→500 }
```
Macro handler becomes:
```rust
let mut ctx = Context::build(req).await;
<before>
let __response = IntoActionResponse::into_action_response({ #body });
<after>
__response
```

### 4. doido-generators — `Field` accessors (for views + active models)
- `name() -> &str` (column name), `html_input_type()`, `is_required()` pub,
  `set_assignment()` (e.g. `title: Set(form.title)`), `struct_field()` (for the
  params struct).

### 5. doido-generators — scaffold templates (`templates/scaffold/`)
- `controller_html.rs.template`, `controller_api.rs.template`
- `views/{index,show,new,edit,_form}.html.tera`

Generator (`scaffold.rs`) responsibilities:
- Run `ModelGenerator` (model + migration + migration lib.rs).
- Render the chosen controller into `app/controllers/{plural}_controller.rs`,
  filling field-driven fragments (params struct, `Set(...)` assignments, table
  columns, form inputs).
- HTML mode: render the 5 views into `app/views/{plural}/`.
- Register the controller in `app/controllers/mod.rs` (read-modify-write).
- Maintain `app/models/mod.rs` (also done by `ModelGenerator` now).
- Inject `resources!({plural}, {Controller});` into `config/routes.rs`
  (read-modify-write, preserving existing routes).

### 6. new-app template wiring
- `src/main.rs` mounts a `models` module.
- `app/models/mod.rs` created with generator markers (like the migration lib.rs).

## Action semantics (HTML variant)

| Action  | Body |
|---------|------|
| index   | `Entity::find().all(ctx.db())` → `render("{plural}/index", …)` |
| show    | `Entity::find_by_id(id).one(...)` → `render("{plural}/show", …)` |
| new     | `render("{plural}/new", …)` |
| create  | `ctx.form::<Params>()` → `ActiveModel{ … Set }` → `insert` → redirect `/{plural}` |
| edit    | `find_by_id` → `render("{plural}/edit", …)` |
| update  | `form` + `find_by_id` → update → redirect |
| destroy | `Entity::delete_by_id(id).exec(...)` → redirect |

API variant mirrors this returning `ctx.json(...)` with appropriate status codes.

## Testing

- doido-model: pool set/get unit test.
- doido-controller: `param`, `form`/`body_json`, `IntoActionResponse` (both arms).
- doido-generators: `Field` accessor units; scaffold content tests (7 actions,
  views present/absent per `--api`, mod.rs + routes injection, field-driven
  fragments). Following existing convention, generated apps are asserted by
  content, not compiled in-suite.

## Out of scope

- Wiring `doido-view` real rendering into `Context::render` (still returns the
  stub body) — separate effort; scaffold emits the templates that will be used
  once it lands.
- Connection pooling tuning / multiple named connections.
