# doido-generators: `new` and `generate` commands — Design

Date: 2026-04-26
Branch: feature/generators-new-generate-commands

## Summary

Add two primary CLI entry points to the doido framework:

- `doido new <app-name> [--database=sqlite|postgres|mysql]` — scaffold a new single-package project from scratch
- `doido generate <generator> [args...]` — apply code templates inside an existing project (already exists, being formalised)

Both commands dispatch through the same `GeneratorRegistry` in `doido-generators`. `new` is a first-class `Generator` implementation named `"new"`, registered in `default_registry()` alongside `scaffold`, `model`, etc.

## Architecture

```
doido new my-app [--database=postgres]
  └─ doido-cli: Commands::New { name, database }
       └─ commands/new.rs
            ├─ prompt for database if --database omitted (sqlite/postgres/mysql)
            ├─ registry.run("new", &[name, "--database=<db>"])
            ├─ write_files(files, root = name/)   ← shared helper
            └─ git init <name>/

doido generate scaffold Post title:String
  └─ doido-cli: Commands::Generate { generator, args }
       └─ commands/generate.rs
            └─ registry.run("scaffold", &["Post", "title:String"])
                 └─ write_files(files, root = ".")
```

`doido-generators` has zero dependency on `doido-cli` (existing constraint preserved).

## `ProjectGenerator` in `doido-generators`

**Location:** `doido-generators/src/generators/new.rs`

Implements `Generator`:
- `name()` → `"new"`
- `description()` → `"Create a new Doido application"`
- `generate(args)` → `Vec<GeneratedFile>`

**`GeneratorArgs` mapping:**
- `args.name` — the application name (e.g. `"my-app"`)
- `args.options["database"]` — `"sqlite"` | `"postgres"` | `"mysql"`

**Files produced** (paths relative to `<app-name>/`):

| Path | Description |
|------|-------------|
| `Cargo.toml` | Single-package manifest; sqlx feature set per database choice |
| `src/main.rs` | Minimal doido boot stub |
| `config/application.toml` | DATABASE_URL + app name |
| `config/routes.rs` | Minimal `routes! {}` stub |
| `app/controllers/.gitkeep` | Placeholder |
| `app/models/.gitkeep` | Placeholder |
| `views/layouts/application.html.tera` | Default HTML layout |
| `db/migrations/.gitkeep` | Placeholder |
| `tests/integration_test.rs` | One passing stub integration test |
| `.gitignore` | Standard Rust + doido ignores |

**Templates:** `doido-generators/src/templates/new/` — embedded via `include_str!`. Tera templates receive context vars `app_name` and `database`.

**Database-specific rendering:**
- `sqlite` → `DATABASE_URL = "sqlite://db/development.db"`, feature `sqlx/sqlite`
- `postgres` → `DATABASE_URL = "postgres://localhost/<app_name>_development"`, feature `sqlx/postgres`
- `mysql` → `DATABASE_URL = "mysql://localhost/<app_name>_development"`, feature `sqlx/mysql`

## `doido-cli` Changes

### New `Commands::New` variant

```rust
/// Create a new Doido application
New {
    /// Application name
    name: String,
    /// Database backend (sqlite, postgres, mysql)
    #[arg(long)]
    database: Option<String>,
},
```

### `commands/new.rs`

1. If `database` is `None` → interactive prompt: `"Which database? [sqlite/postgres/mysql]"`
2. Validate input: reject unknown database values with a clear error
3. Call `registry.run("new", &[&name, &format!("--database={db}")])`
4. Call `write_files(&files, Path::new(&name))` — creates dirs, writes content, prints `  create  <path>`
5. Run `git init <name>/` via `std::process::Command`
6. Print success summary

### Shared `write_files` helper

Extracted to `commands/mod.rs`:

```rust
pub fn write_files(files: &[GeneratedFile], root: &Path) -> anyhow::Result<()>
```

Creates parent directories as needed, writes file content, prints `  create  <path>` per file. Used by both `new` and `generate`.

## Error Handling

- Unknown database value → `anyhow::bail!("Unknown database: {db}. Use sqlite, postgres, or mysql.")`
- File write failure → propagate `io::Error` wrapped in `anyhow`
- `git init` failure → print warning, do not abort (non-fatal)
- Generator not found → existing registry error path

## Testing

### `doido-generators` tests (`tests/new_generator_test.rs`)

- `test_new_generates_all_expected_files` — output contains all 10 expected paths
- `test_new_sqlite_cargo_toml_has_sqlite_feature` — Cargo.toml content check
- `test_new_postgres_sets_correct_database_url` — application.toml content check
- `test_new_mysql_sets_correct_database_url` — application.toml content check
- `test_new_integration_test_file_is_valid_stub` — tests/integration_test.rs compiles (content check)
- `test_new_output_is_deterministic` — same args → same output

### `doido-cli` tests

- `test_new_command_writes_files_to_disk` — temp dir, assert files exist
- `test_new_command_runs_git_init` — assert `.git/` dir created
- `test_new_command_prompts_when_database_omitted` — (manual / integration)

## Module Structure Changes

```
doido-generators/src/
  generators/
    new.rs              ← NEW: ProjectGenerator
  templates/
    new/                ← NEW: Tera templates
      Cargo.toml.tera
      src/main.rs.tera
      config/application.toml.tera
      config/routes.rs.tera
      views/layouts/application.html.tera.tera
      tests/integration_test.rs.tera
      .gitignore.tera
  registry.rs           ← register ProjectGenerator in default_registry()

doido-cli/src/
  commands/
    new.rs              ← NEW: Commands::New handler
    mod.rs              ← add write_files helper + pub mod new
  lib.rs                ← add Commands::New variant + dispatch arm
```

## Known Constraints

- `doido-generators` has zero dependency on `doido-cli` (preserved)
- All generator output is deterministic given the same args (required for TDD)
- Templates embedded in binary via `include_str!` — no runtime template files
- `git init` failure is non-fatal (warn and continue)
