# Doido migration DSL — eliminate raw SQL in `db/migration`

Phased plan to extend the **Doido / `doido-model` migration command surface** so every file under `backend/db/migration/` can express schema and data changes without hand-written SQL, then refactor Fivia’s migrations to use it exclusively.

**Doido version today:** `=0.0.31` (see `backend/Cargo.toml` workspace deps).

**Related code today:**

| Location | Role |
|---|---|
| `doido_model::migration` | Rails-style helpers: `create_table`, `alter_table`, `add_column`, `add_index`, `add_foreign_key`, … (SeaQuery + `ConnectionTrait::execute`) |
| `backend/db/migration/schema.rs` | Vendored **ColType** helpers (`create_table(m, …)`, `add_column`, `add_reference`, …) — mounted via `backend/src/lib.rs` as `crate::schema` |
| `doido db migrate …` | SeaORM migrator in-process (`doido_model::commands::db::run_migrator`); migrations live in `db/migration/` |

---

## Goal

1. **Framework:** Ship missing migration helpers in **`doido-model`** (and document them on `doido db` / migration generator output) so common PostgreSQL DDL/DML does not require `execute_unprepared` / string `Statement`s.
2. **Fivia:** Refactor all **`backend/db/migration/*.rs`** (and `schema.rs` if kept) so **no migration calls**:
   - `ConnectionTrait::execute_unprepared`
   - `execute_raw` / `query_one_raw` / `query_all_raw` with literal SQL strings
   - `Statement::from_string` for DDL/DML authored by hand

**Allowed:** `SchemaManager` methods, `doido_model::migration::*`, SeaQuery builders (`Table::`, `Index::`, `ColumnDef::`, `Expr::`, …) passed to `manager.create_table` / `alter_table` / `create_index`, and **data** changes built with SeaQuery `Update` / `Delete` / `Insert` or SeaORM `Entity`/`Query` (no string SQL).

---

## Current state (inventory)

As of this plan:

| Metric | Value |
|---|---|
| Migration modules | ~67 under `backend/db/migration/` |
| Files still using raw SQL | **32** (see list below) |
| Heaviest offenders | `m20261001_120000_payment_centric_billing.rs` (~48 calls), `m20260824_000000_create_exam_tests.rs` (~30), `m20260820_000000_create_test_templates.rs` (~15), `m20260602_000000_add_company_id_to_specialties.rs` (~15) |

**Files with `execute_unprepared` / similar (rewrite targets):**

`m20250927_151055_enable_crypto_option`, `m20260601_000000_add_frequency_to_medical_record_medications`, `m20260602_000000_add_company_id_to_specialties`, `m20260607_000000_medical_record_signing`, `m20260613_000000_create_signing_certificates`, `m20260613_120000_create_release_reports`, `m20260620_120000_add_target_roles_to_release_reports`, `m20260620_140000_create_billing_tables`, `m20260621_000000_create_support_chat`, `m20260622_000000_add_storage_columns_to_documents`, `m20260819_000000_remove_useless_models`, `m20260820_000000_create_test_templates`, `m20260820_000002_create_test_applications`, `m20260823_000000_add_exam_fields_to_medical_records`, `m20260824_000000_create_exam_tests`, `m20260830_000000_drop_exam_tests_template_unique`, `m20260831_000000_create_audit_version_tables`, `m20260901_000000_drop_versions_item_id_fk`, `m20260902_000000_recreate_release_reports`, `m20260910_000000_drop_companies_company_type`, `m20260918_000000_add_sort_order_to_billing_plans`, `m20260919_000000_add_apply_age_range_to_test_templates`, `m20260925_000000_asaas_billing`, `m20260928_000000_move_user_pii_to_patients_doctors`, `m20260930_120000_create_billing_payment_integration_logs`, `m20260930_200000_move_identity_document_to_users`, `m20261001_120000_payment_centric_billing`, `m20261003_120000_doctors_contact_phone_required`, `m20261004_120000_add_calendar_feed_token_to_company_users`, `m20261005_120000_create_appointment_recurrence_series`, `m20261008_120000_create_doctor_availability_rules`, `m20261008_120000_move_tours_to_frontend_options`, plus **`schema.rs`** (enum helpers use raw introspection/DROP TYPE).

**Style split in Fivia:**

- ~35 migrations already use **`crate::schema::*` + ColType** (good direction, but duplicated vs Doido).
- Several use **`SchemaManager` + sea_orm_migration::schema** directly (e.g. billing tables) — keep pattern, drop only the raw index tails.
- Newer files sometimes skip helpers entirely and embed full `CREATE TABLE …` strings.

**Doido DSL gaps vs Fivia needs** (raw SQL categories observed):

| Category | Example in Fivia | Present in `doido_model::migration`? |
|---|---|---|
| PostgreSQL extension | `CREATE EXTENSION pgcrypto` | No |
| Partial / filtered index | `UNIQUE INDEX … WHERE col IS NOT NULL` | No (only simple `add_index`) |
| Named index (non-`idx_<table>_…`) | `company_users_calendar_feed_token_key` | No |
| Index with `DESC` / multi-col + predicate | billing partial indexes | No |
| `CHECK` constraints | `doctor_availability_rules` | No |
| `TIME` columns | availability rules | No (`TableBuilder` lacks `time`, `small_integer`, `timestamptz`) |
| `ALTER COLUMN SET/DROP NOT NULL`, `DROP DEFAULT` | PII / identity moves | No |
| Data backfill | `UPDATE … FROM …` | No |
| `DELETE WHERE` | specialties NULL company cleanup | No |
| `DROP TABLE … CASCADE` | billing refactor | Partial (`drop_table` uses `IF EXISTS`, not CASCADE) |
| Drop constraint / FK by name | versions FK, unique on exam_tests | Partial (FK naming differs from Fivia `fk-{from}-{col}-to-{to}`) |
| Dynamic per-table DDL | audit `{table}_versions` | No helper |
| JSONB column updates | tour → `frontend_options` | No |

Internal note: **`doido_model::migration::create_table` uses `big_integer` auto-increment `id`**, while Fivia ColType uses **`SERIAL` / integer PK** (`PkAuto`). Any consolidation must either add **`id: serial`** mode to the Doido builder or standardize on bigint PKs for **new** tables only (existing tables stay int serial).

---

## Production safety

Fivia tracks **`production`**. Rewriting an already-applied migration changes checksum semantics only if someone runs old code against a DB that partially applied — SeaORM records migration **name**, not file hash.

Rules for this effort:

1. **Do not change the observable effect** of `up`/`down` for migrations already applied in production (same tables, columns, indexes, constraints, and row transformations).
2. Prefer **mechanical translation** (raw SQL → equivalent SeaQuery) over “cleanup” of legacy choices.
3. Verify with **`cargo doido db migrate fresh`** on an empty Postgres DB and **`make test`** (backend tests migrate once per suite).
4. Optional hardening: snapshot `pg_dump --schema-only` before/after refactor on a fresh migrate and diff (CI job, not blocking at first).

---

## Phase 1 — Extend `doido-model` migration API (upstream)

**Target crate:** `doido-model` (publish **`0.0.32`** or next patch; bump Fivia workspace pins after release).

Add modules under `src/migration/` and re-export from `doido_model::migration` (keep existing functions stable).

### 1.1 PostgreSQL extensions

```rust
pub async fn enable_extension(manager: &SchemaManager<'_>, name: &str) -> Result<(), DbErr>;
// Postgres: CREATE EXTENSION IF NOT EXISTS "name"
// Other backends: no-op + tracing::debug
```

Unit test: renders correct SQL on Postgres builder; SQLite test is no-op.

### 1.2 Index helpers (generalize `index.rs`)

```rust
pub struct IndexOpts {
    pub unique: bool,
    pub name: Option<String>,           // default: idx_<table>_<cols>
    pub where_expr: Option<Expr>,       // partial index
    pub nulls_not_distinct: bool,       // PG15+ unique nulls (optional)
}

pub async fn create_index(
    manager: &SchemaManager<'_>,
    table: &str,
    columns: &[IndexColumn],            // name + optional Sort::Desc
    opts: IndexOpts,
) -> Result<(), DbErr>;

pub async fn drop_index(
    manager: &SchemaManager<'_>,
    name: &str,
    table: Option<&str>,                // backend-specific DROP INDEX
) -> Result<(), DbErr>;
```

Covers: `m20261004_*` partial unique, billing partial indexes, custom index names in exam/billing migrations.

### 1.3 Table-level checks and table builder types

Extend `TableBuilder`:

- `small_integer`, `big_integer` (non-PK), `time`, `timestamptz` / `timestamp_with_time_zone`
- `json_binary` (JSONB)
- `check(name, expr)` → table constraint
- `references_named(name, to_table, opts: ReferenceOpts)` with `on_delete`, `on_update`, **`foreign_key_name`**
- Optional: **`without_implicit_id()`** + `serial_id()` for Loco/Fivia-compatible PKs

Extend `AlterTableBuilder`:

- `set_not_null(col)`, `drop_not_null(col)`
- `set_default(col, expr)`, `drop_default(col)`
- `add_check(name, expr)` (Postgres)

### 1.4 Foreign keys (align with Fivia naming)

Today Doido uses `fk_{table}_{column}`; Fivia vendored schema uses `fk-{fromtbl}-{ref}-to-{totbl}`.

Add:

```rust
pub struct ForeignKeyOpts {
    pub name: Option<String>,
    pub on_delete: ForeignKeyAction,
    pub on_update: ForeignKeyAction,
}

pub async fn add_foreign_key_with_opts(..., opts: ForeignKeyOpts) -> Result<(), DbErr>;
pub async fn remove_foreign_key_by_name(manager, table, constraint_name) -> Result<(), DbErr>;
```

Default name strategy: **parameterized** so Fivia can pass Loco-style names without raw SQL.

### 1.5 Data migration helpers (`migration::data`)

New submodule — still **no string SQL**; build SeaQuery statements:

```rust
pub async fn update_from(
    manager: &SchemaManager<'_>,
    target_table: &str,
    set: &[(&str, Expr)],              // e.g. identity_document = users.identity_document
    from_table: &str,
    join_on: Expr,
) -> Result<(), DbErr>;

pub async fn update_where(
    manager: &SchemaManager<'_>,
    table: &str,
    set: &[(&str, Expr)],
    filter: Expr,
) -> Result<(), DbErr>;

pub async fn delete_where(
    manager: &SchemaManager<'_>,
    table: &str,
    filter: Expr,
) -> Result<(), DbErr>;
```

Implement using `sea_query::UpdateStatement` / `DeleteStatement` with Postgres backend, `FROM` clause where supported.

Covers: `m20260928_*`, `m20260910_*`, `m20260918_*`, `m20261003_*`, JSONB tour migration.

### 1.6 Drop table with cascade

```rust
pub async fn drop_table_cascade(manager: &SchemaManager<'_>, name: &str) -> Result<(), DbErr>;
```

### 1.7 Audit / version tables helper (optional but high leverage)

```rust
pub async fn create_audit_version_table(
    manager: &SchemaManager<'_>,
    source_table: &str,
    version_table: &str,   // default: "{source_table}_versions"
) -> Result<(), DbErr>;
```

Encodes the pattern in `m20260831_*` without `format!(… CREATE TABLE …)`.

### 1.8 Enum helpers (move off raw SQL)

Port logic from Fivia `schema.rs`:

- `create_enum_type`, `add_enum_values`, `drop_enum_type` using SeaQuery where possible; isolate any unavoidable Postgres catalog query inside **`doido_model::migration::enum_type`** only (not in app migrations).

### 1.9 Tests & docs (Doido)

| File | Action |
|---|---|
| `doido-model/tests/migration_dsl_test.rs` | Extend for new types, partial indexes, alter null/default |
| New `doido-model/tests/migration_postgres_integration.rs` | Run against `TestDb` Postgres feature if available |
| `doido-model/src/migration/mod.rs` | Module docs + examples for each helper |
| `doido-generators` (if migration stub exists) | Generated migration imports `doido_model::migration::*`, comment ban on raw SQL |

### 1.10 CLI surface (`doido db`)

No new subcommands required; update **help text** and developer docs:

- `doido db migrate generate` template uses DSL helpers.
- Mention **`doido db migrate --without-entities`** unchanged.

**Verify (Doido repo):** `cargo test -p doido-model`, publish crate, tag release.

---

## Phase 2 — Fivia: adopt Doido migration API

### 2.1 Dependency bump

- Bump workspace `doido*` to the release that includes Phase 1.
- `backend/Cargo.toml`: ensure `doido-model` feature `postgres` (+ `cli` if tests need it).

### 2.2 Re-export strategy (pick one; recommended: **B**)

**A — Thin re-export (minimal churn):**  
Keep `backend/db/migration/schema.rs` but implement each function as a wrapper around `doido_model::migration` + ColType compatibility shim.

**B — Consolidate (recommended):**  
Deprecate vendored `schema.rs`; migrate ColType call sites to `doido_model::migration` + `TableBuilder` / `add_column` closures. Remove `#[path = "../db/migration/schema.rs"]` from `backend/src/lib.rs` when unused.

**C — Dual import during transition:**  
New migrations only use `doido_model::migration`; old files converted in Phase 3.

Update **`backend/AGENTS.md`**: “Migrations must use `doido_model::migration` / SeaQuery; no raw SQL.”

### 2.3 ColType / naming parity checklist

When converting, preserve:

- Table names (pluralization in ColType `normalize_table` vs Doido literal names — **match existing table names exactly**).
- FK names (`fk-specialties-company_id-to-companies` style).
- Index names (`idx_*`, partial predicates, unique constraints).
- Column types: `TIMESTAMPTZ`, `SMALLINT` enums as integers, `UUID` defaults `gen_random_uuid()`.

---

## Phase 3 — Rewrite Fivia migrations (batched)

Work in batches; after each batch: `cargo doido db migrate fresh` + `cargo test -- --test-threads 1`.

### Batch A — Extensions, indexes, simple DDL (low risk)

| File | Raw pattern → helper |
|---|---|
| `m20250927_151055_enable_crypto_option` | `enable_extension(m, "pgcrypto")` |
| `m20261004_*` | `add_column` + `create_index` with `where_expr` |
| `m20261005_*` | `drop_index` + `add_index` (recurrence) |
| `m20260930_120000_create_billing_payment_integration_logs` | indexes via `create_index` |
| `m20260830_*`, `m20260901_*` | `drop_index` / `remove_foreign_key_by_name` |
| `m20260620_140000_create_billing_tables` | replace 2 partial indexes only (tables already SeaQuery) |

### Batch B — `create_table` strings → builders

| File | Notes |
|---|---|
| `m20261008_120000_create_doctor_availability_rules` | `create_table` + `.check(...)` + FK opts + `time` / `small_integer` |
| `m20260621_*`, `m20260902_*`, `m20260613_120000_*` | support chat, release reports |
| `m20260820_000000_create_test_templates` | multi-table test schema |
| `m20260820_000002_create_test_applications` | |
| `m20260925_000000_asaas_billing` | |

### Batch C — Column alters & backfills

| File | Notes |
|---|---|
| `m20260601_*`, `m20260607_*`, `m20260613_000000_*`, `m20260620_120000_*` | `alter_table` set/drop null/default |
| `m20260622_*`, `m20260819_*`, `m20260823_*` | add/drop columns |
| `m20260919_*`, `m20261008_120000_move_tours_to_frontend_options` | JSONB updates via `update_where` / `Expr::cust` sparingly |
| `m20261003_*` | NOT NULL enforcement after backfill |

### Batch D — Data migrations

| File | Notes |
|---|---|
| `m20260602_000000_add_company_id_to_specialties` | `update_from`, `delete_where`, then `set_not_null`, indexes |
| `m20260910_000000_drop_companies_company_type` | `update_where` |
| `m20260928_*`, `m20260930_200000_*` | multi-statement PII moves |

### Batch E — Large structural migrations

| File | Notes |
|---|---|
| `m20260824_000000_create_exam_tests` | Split: SeaQuery table/index/FK + data backfill helpers |
| `m20261001_120000_payment_centric_billing` | Largest file; do sub-PRs (add columns → backfill → drop old tables) |
| `m20260831_000000_create_audit_version_tables` | `create_audit_version_table` loop |

### Batch F — `schema.rs`

- Move enum/check helpers into Doido Phase 1.8.
- Delete or reduce `backend/db/migration/schema.rs` to re-exports only.

---

## Phase 4 — Enforcement

### 4.1 CI guard (Fivia)

Add script (e.g. `scripts/check-migrations-no-raw-sql.sh`) invoked from `make test` or CI:

```bash
rg 'execute_unprepared|execute_raw|query_one_raw|query_all_raw|Statement::from_string' \
  backend/db/migration --glob '*.rs' \
  && { echo 'Raw SQL in migrations forbidden'; exit 1; } || true
```

Allowlist: none in `m*.rs` after Phase 3; optionally allow **`schema.rs`** until deleted.

### 4.2 Optional Clippy / custom lint (later)

If migrations move to a tiny `#![deny(...)]` crate, use `clippy::disallowed_methods` on `ConnectionTrait::execute_unprepared` in that crate only.

---

## Phase 5 — Documentation

| Doc | Update |
|---|---|
| `backend/AGENTS.md` | Migration section: DSL only, link to `doido_model::migration` docs |
| `docs/AGENTS.md` | Optional superpowers spec `docs/superpowers/specs/2026-10-09-migration-dsl-design.md` if brainstorming output exists |
| Doido changelog | Breaking: none if only additive APIs |

---

## Verification matrix

| Check | Command |
|---|---|
| Format / compile | `cd backend && cargo fmt && cargo check` |
| Fresh schema | `cargo doido db migrate fresh` (Postgres up via `docker-compose`) |
| Full suite | `make test` from repo root |
| No raw SQL | `scripts/check-migrations-no-raw-sql.sh` |
| Entity sync | `cargo doido db migrate` (default re-exports entities) |

---

## Suggested PR sequence

1. **doido-model:** Phase 1 API + tests (external repo / crate release).
2. **fivia:** Bump Doido + Batch A migrations + CI script (warn-only first).
3. **fivia:** Batches B–C.
4. **fivia:** Batches D–E + remove `schema.rs` + CI enforce (fail on raw SQL).

---

## Out of scope

- Rewriting **`already-squashed`** history for greenfield installs only (`migrate fresh`) — not replacing Flyway-style external migration tools.
- SQLite compatibility for Fivia production (Postgres-only helpers are acceptable if gated on `DatabaseBackend::Postgres`).
- Auto-generating migrations from entity diff (SeaORM `generate` stays as-is).

---

## Success criteria

- [ ] Zero matches for forbidden raw SQL patterns in `backend/db/migration/m*.rs`.
- [ ] All helpers used by Fivia documented in `doido_model::migration` with unit tests.
- [ ] `make test` green on CI.
- [ ] New migrations generated/scaffolded use DSL helpers by default.
