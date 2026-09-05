# 15 — Storage (`doido-storage`)

Attached-file storage for Doido — the **ActiveStorage** analogue. It stores file
bytes through a pluggable **service** (local disk by default; in-memory, S3,
Cloudflare R2, Azure Blob, and GCS behind features) and keeps metadata (blobs,
polymorphic attachments, variant records) in the database. A [`Storage`] facade
ties a service, a sea-orm connection and an HMAC signer together and offers
Rails-like operations; axum routes serve blobs and accept direct uploads.

> **Status:** Core done (blobs, attachments, signing, serving, direct upload,
> generators). **Deferred:** variants, previews, Mirror, native Azure SAS,
> `#[has_one_attached]`, `compose`.

## Decisions (resolved)

- **Pluggable `Service` backends** — disk default; cloud behind Cargo features.
- **Named drivers in one YAML section** — per-env file selects the active `driver`
  (Rails `config/storage.yml` + `config.active_storage.service`).
- **Metadata in DB, bytes in service** — raw SQL for blobs/attachments; canonical
  schema via `storage:install` migration.
- **HMAC signed ids/URLs** — `DOIDO_SECRET_KEY_BASE` (insecure dev fallback with warning).
- **Serving routes auto-mounted at boot** — after `init_storage()` when initialisation succeeds.

## Crate map

| Module | Responsibility |
|--------|----------------|
| `service` | The `Service` trait + `UrlOptions` |
| `providers` | Built-in backends (`disk`, `memory`, `s3`, `azure`, `gcs`) |
| `registry` | Custom adapters: `register_adapter` + `type: <kind>` |
| `config` | `storage:` section of `config/<env>.yml` → `Arc<dyn Service>` |
| `client` | `Storage` facade |
| `global` | Process-global singleton (`init_storage`, `storage()`) |
| `testing` | Test helpers (`memory_storage`, `install`) |
| `blob` / `attachments` | Metadata + polymorphic attach helpers |
| `signing` | HMAC-SHA256 signed ids / URLs |
| `serving` | axum redirect, proxy, disk PUT, direct uploads |
| `jobs` | `purge_later` / `analyze_later` (feature `storage-jobs`) |

## `Service` trait (pluggable)

```rust
#[async_trait::async_trait]
pub trait Service: Send + Sync {
    fn name(&self) -> &str;
    fn public(&self) -> bool { false }
    async fn upload(&self, key: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<()>;
    async fn download(&self, key: &str) -> Result<Vec<u8>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn size(&self, key: &str) -> Result<u64>;
    async fn url(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> { ... }
    async fn presigned_put(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> { ... }
}
```

Custom adapters implement `Service`, register a factory with `register_adapter("kind", ...)`,
then select `type: kind` in YAML. Scaffold with `doido generate storage:adapter <Name>`.

## Built-in backends

| Backend | YAML `type` | Cargo feature | Notes |
|---------|-------------|---------------|-------|
| `DiskService` | `disk` | default | `root` (default `storage`), optional `public` |
| `MemoryService` | `memory` | default | In-process; tests |
| `S3Service` | `s3` | `storage-s3` | AWS S3; credentials from config or `AWS_*` env |
| `S3Service` (R2) | `r2` | `storage-s3` | Cloudflare R2; requires `endpoint` |
| `AzureBlobService` | `azure` | `storage-azure` | `container`, `account`; key from config or `AZURE_STORAGE_ACCESS_KEY` |
| `GcsService` | `gcs`, `google` | `storage-gcs` | `bucket`; ADC via `GOOGLE_APPLICATION_CREDENTIALS` |

Enable cloud features on the `doido` dependency, e.g. `doido = { features = ["storage-s3"] }`.
Selecting a backend whose feature is off yields a clear error at boot.

## Configuration (`config/<env>.yml`)

Top-level keys under `storage:`:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `driver` | `String` | first driver / `local` | Active named driver (alias: `service`) |
| `drivers` | map | — | Named driver configs (alias: `services`) |
| `prefix` | `String` | `/doido/storage` | Route prefix for serving |
| `expires_in` | `u64` | `300` | Signed-URL TTL in seconds |

Each entry in `drivers` is a [`ServiceConfig`]:

| Field | Used by | Description |
|-------|---------|-------------|
| `type` | all | `disk`, `memory`, `s3`, `r2`, `azure`, `gcs`, or custom kind |
| `public` | disk, cloud | Public readability (URL generation) |
| `root` | disk | Filesystem root (default `storage`) |
| `bucket` | s3, r2, gcs | Bucket name |
| `region` | s3, r2 | Region (`auto` for R2) |
| `endpoint` | s3, r2 | Custom endpoint (required for R2) |
| `access_key_id`, `secret_access_key` | s3, r2 | Else `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` |
| `container`, `account` | azure | Container and storage account |
| `access_key` | azure | Else `AZURE_STORAGE_ACCESS_KEY` |
| *(any other key)* | custom | Available via `cfg.option_str("key")` |

### Example: development

```yaml
storage:
  driver: local
  drivers:
    local: { type: disk, root: storage }
    test:  { type: memory }
```

### Example: test

```yaml
storage:
  driver: test
  drivers:
    local: { type: disk, root: storage }
    test:  { type: memory }
```

### Example: production (S3)

```yaml
storage:
  driver: amazon
  prefix: /files
  expires_in: 3600
  drivers:
    local:  { type: disk, root: storage }
    amazon: { type: s3, bucket: my-bucket, region: us-east-1 }
```

Credentials must not be committed inline in production — use environment variables
or encrypted credentials (see [05-config.md](05-config.md)).

### Secrets & environment variables

| Variable | Purpose |
|----------|---------|
| `DOIDO_SECRET_KEY_BASE` | HMAC signing for blob ids and serving URLs (required in production) |
| `STORAGE__DRIVER` | Override active driver |
| `STORAGE__PREFIX` | Override serving route prefix |
| `STORAGE__EXPIRES_IN` | Override signed-URL TTL (seconds) |
| `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` | S3/R2 credentials |
| `AZURE_STORAGE_ACCESS_KEY` | Azure Blob credentials |
| `GOOGLE_APPLICATION_CREDENTIALS` | GCS application-default credentials |

`doido_storage::config::load()` reads `config/<env>.yml` and applies `STORAGE__*`
overrides. Missing file / missing section defaults to a disk driver named `local`
rooted at `storage/`.

Legacy YAML keys `service` / `services` still deserialize (aliases for `driver` / `drivers`).

## Boot & access

Boot order (see [ARCHITECTURE.md](ARCHITECTURE.md)): after the DB pool is ready,
`doido_storage::init_storage()` builds the facade from config and installs the
global singleton. Controllers read it via `ctx.storage()`. Serving routes are
merged automatically at server boot when storage initialised successfully
(`doido_storage::serving::merge_routes`).

```rust
// In a controller:
let blob = ctx.storage().attach_upload("User", "1", "avatar", "me.png", bytes).await?;
```

Programmatic boot:

```rust
use doido_storage::{init_from_config, init_with, StorageConfig};

init_from_config(my_config).await?;           // from explicit config
init_with(test_storage).await?;               // install a built facade
```

Custom adapters must register **before** `init_storage()`:

```rust
// src/main.rs — before Doido::new().run()
#[path = "../app/storage/mod.rs"]
mod storage;

#[tokio::main]
async fn main() {
    storage::register_all();   // from `doido generate storage:adapter`
    doido::Doido::new().router(routes::router()).run().await;
}
```

If `init_storage()` fails, the server still starts (warn log) but `ctx.storage()` panics
on first use — fix config or enable the required Cargo feature.

## Storage facade API

```rust
use doido_storage::{Storage, Disposition, StorageConfig, Signer};

let storage = Storage::from_config(conn).await?;
// or:
let storage = cfg.into_storage(conn, Signer::from_env()).await?;

storage.ensure_tables().await?;  // dev/test convenience only

let blob = storage
    .attach_upload("User", "1", "avatar", "me.png", bytes)
    .await?;

let avatar = storage.one("User", "1", "avatar").await?;
let url = storage.url_for(&blob, Disposition::Inline).await?;
storage.purge_for_record("User", "1").await?;
```

## Serving

Default prefix `/doido/storage` (override with `storage.prefix` or `STORAGE__PREFIX`):

* `GET  {prefix}/blobs/redirect/{signed_id}/{filename}` — redirect to native or proxy URL
* `GET  {prefix}/blobs/proxy/{signed_id}/{filename}` — stream through the app
* `PUT  {prefix}/disk/{token}` — disk/memory direct upload
* `POST {prefix}/direct_uploads` — create blob + return upload URL

Routes are mounted automatically at boot; no manual `routes.rs` merge required for
standard apps.

## Generators

- **`doido generate storage:install`** — migration for `storage_blobs`,
  `storage_attachments`, `storage_variant_records`; appends `storage:` to
  `config/development.yml` and `config/test.yml` (skips if already present;
  idempotent). `doido new` bundles the migration tables by default.
- **`doido generate storage:adapter <Name>`** — scaffolds `app/storage/<name>_service.rs`
  plus `app/storage/mod.rs` with `register_all()` for boot wiring.

## Background jobs (feature `storage-jobs`)

`purge_later` / `analyze_later` enqueue work via `doido-jobs`. The analyze job is
currently a stub.

## Testing

```rust
use doido_storage::testing::{install, memory_storage};

let storage = memory_storage(conn);
install(storage.clone()).unwrap();

// Controllers:
let avatar = ctx.storage().one("User", "1", "avatar").await?;
```

`MemoryService` keeps bytes in-process. `set_storage` / `init_with` install a
global facade for integration tests.

## Known requirements

- Cloud backends require matching Cargo features on `doido` / `doido-storage`.
- Blob metadata SQL is SQLite-oriented today; use the generator migration as the
  canonical schema path.
- Only the active `driver` is built at runtime; blobs record `service_name` but
  operations use the global active service.
- `DOIDO_SECRET_KEY_BASE` must be set in production for tamper-proof URLs.

## TDD surface

- `disk_test` / `memory_test` — `Service` contract
- `blob_test` — checksum/content-type/size; purge
- `attachments_test` — has_one replace, has_many append, dependent purge
- `config_test` — YAML parse, env overrides, `into_storage`, feature errors
- `registry_test` — custom adapter via `type` + `cfg.options`
- `signing_test` — sign/verify, tamper/expiry
- `serving_test` — proxy, redirect, direct upload
- `global_init_test` — idempotent init
- `schema_test` — `ensure_tables` idempotent

## Deferred (backlog)

Image variants, PDF/video previews, video/audio analyzers, Mirror service, native
Azure SAS URLs, `#[has_one_attached]` proc-macro, and `compose` (multi-service routing).
