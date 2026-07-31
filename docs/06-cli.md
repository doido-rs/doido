# doido-cli — Spec

Rails analogue: **rails runtime commands** (`rails server`, `rails console`, `rails db:*`)

> **Status (2026-07-28): mostly done.** Runtime commands are merged into
> **`doido-generators`**. Implemented: `server`, `console` (evcxr), `routes`,
> `db create` + SeaORM migrate/rollback/status passthrough, `worker`, `generate`, `new`.
> **Open:** `db seed`/`db reset`/`db prepare`/`db schema` exist in `doido-model` but are
> **not wired** as subcommands; `jobs:failed/retry/discard` and `credentials:edit` are
> log-only stubs and `credentials:show` is absent; `server` does not parse `--port`/`--env`.
> See [ARCHITECTURE.md](ARCHITECTURE.md).

## Decisions (resolved in interview)

- **Runtime commands only** — generators live in the separate `doido-generators` crate
- **SeaORM CLI** — `doido db migrate|generate …` delegates to `doido_model::sea_orm_cli` (feature `cli` on `doido-model`); never import `sea_orm_cli` directly
- `doido-cli` depends on `doido-generators` to dispatch `doido generate` commands, but does not own generator logic

## Responsibility

`doido-cli` owns the binary entry point and all **runtime** subcommands:

```
doido server                  ← start axum server
doido server --port 4000
doido server --env production

doido console                 ← interactive REPL with app context loaded

doido routes                  ← print all registered routes as a table

doido db migrate              ← run pending migrations (via doido_model::sea_orm_migration / sea_orm_cli)
doido db rollback
doido db rollback --step 3
doido db status
doido db seed
doido db reset                ← drop + migrate + seed

doido jobs:failed             ← list dead letter jobs
doido jobs:retry <job_id>
doido jobs:retry --all
doido jobs:discard <job_id>

doido worker                  ← start background job worker process
doido worker --queue critical

doido credentials:edit        ← decrypt, open $EDITOR, re-encrypt
doido credentials:show        ← print decrypted credentials (dev only)

doido generate <name> [args]  ← delegates to doido-generators
doido generate --list         ← list all registered generators
```

## Module Structure

```
doido-cli/
  src/
    lib.rs
    main.rs
    commands/
      server.rs
      console.rs
      routes.rs
      db/
        mod.rs
        migrate.rs
        rollback.rs
        seed.rs
        reset.rs
        status.rs
      jobs.rs
      credentials.rs
      generate.rs       ← thin shim: parses args, delegates to doido-generators
      worker.rs
```

## Known Requirements

- Binary: `doido` (entry point in `doido-cli`)
- CLI argument parsing via `clap`
- `doido generate` subcommand delegates entirely to `doido_generators::dispatch(args)`
- All runtime commands are independently testable modules
- `doido routes` prints the route table the `routes!` macro registers (in
  `doido-controller`) as the app builds its router; `doido server` prints the
  same table on startup before listening

## TDD Surface

- Test `doido routes` prints correct route table
- Test `doido db migrate` invokes sea-orm runner
- Test `doido db rollback --step N` rolls back N steps
- Test `doido generate` delegates to generator registry and passes args through
- Test unknown subcommand prints help and exits with non-zero code
