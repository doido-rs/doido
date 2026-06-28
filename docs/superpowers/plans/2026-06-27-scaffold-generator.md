# Scaffold Generator + Context DB — Implementation Plan

Date: 2026-06-27
Design: ../specs/2026-06-27-scaffold-generator-design.md

Staged so each step compiles and is testable on its own.

## Step 1 — doido-model global pool
- [x] `src/pool.rs`: add `OnceLock<DatabaseConnection>` + `init()`, `pool()`,
      `try_pool()`, `set_pool()`.
- [x] Re-export from `src/lib.rs`.
- [x] Unit test: `set_pool` then `pool()` returns it.

## Step 2 — doido-controller Context DB / params / body
- [x] Add `doido-model` dependency to `doido-controller/Cargo.toml`.
- [x] `Context`: `body: Option<Body>`, `path_params: Vec<(String,String)>`.
- [x] `Context::build(req).await` central constructor + `extract_path_params`.
- [x] `db()`, `param()`, `form::<T>()`, `body_json::<T>()`.
- [x] Keep `from_request`/`from_request_parts` (empty params).
- [x] Tests: `param`, `form`, `body_json`.

## Step 3 — doido-controller macro `IntoActionResponse`
- [x] Define trait + impls (`Response`, `Result<Response, E: Display>`).
- [x] Macro: build via `Context::build(req).await`, wrap body in
      `IntoActionResponse::into_action_response(...)`.
- [x] Test: action returning `Result<Response>` (Ok + Err → 500).

## Step 4 — Field accessors
- [x] `name()`, public `is_required()`, `html_input_type()`,
      `params_struct_field()`, `active_model_set()`.
- [x] Unit tests.

## Step 5 — ModelGenerator maintains app/models/mod.rs
- [x] Emit/patch `app/models/mod.rs` registering `pub mod {singular};` +
      `pub use {singular}::Entity as {Model};` via markers.
- [x] new-app template: `app/models/mod.rs` with markers; `src/main.rs` mounts it.

## Step 6 — Scaffold generator + templates
- [x] `templates/scaffold/controller_html.rs.template`,
      `controller_api.rs.template`, `views/*.html.tera`.
- [x] `scaffold.rs`: parse `--api`; run model gen; render controller with
      field fragments; emit views (HTML); register controller in
      `app/controllers/mod.rs`; inject `resources!` into `config/routes.rs`.
- [x] Replace the clobbering route marker with real read-modify-write injection.

## Step 7 — Tests + build
- [x] Scaffold content tests (HTML + `--api`), injection tests.
- [x] `cargo test` for model, controller, generators; `cargo build --workspace`.
