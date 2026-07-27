# 15 — Storage (`doido-storage`)

Attached-file storage for Doido — the **ActiveStorage** analogue. It stores file
bytes through a pluggable **service** (local disk by default; in-memory, S3,
Cloudflare R2 and Azure Blob behind features) and keeps metadata (blobs,
polymorphic attachments, variant records) in the database. A [`Storage`] facade
ties a service, a sea-orm connection and an HMAC signer together and offers
Rails-like operations; axum routes serve blobs and accept direct uploads.

## Crate map

| Module | Responsibility |
|--------|----------------|
| `service` | The `Service` trait (`upload`/`download`/`delete`/`exists`/`size`/`url`/`presigned_put`) + `Arc<dyn Service>` impl |
| `disk` | `DiskService` — local filesystem, sharded `root/aa/bb/key` (**default**) |
| `memory` | `MemoryService` — in-process `HashMap` (dev/test) |
| `s3` | `S3Service` — AWS S3 **and** Cloudflare R2 (feature `storage-s3`) |
| `azure` | `AzureBlobService` (feature `storage-azure`) |
| `gcs` | `GcsService` — Google Cloud Storage (feature `storage-gcs`) |
| `registry` | custom-adapter registry: `register_adapter` + `type: <kind>` selection |
| `config` | `storage:` section of `config/<env>.yml` → `Arc<dyn Service>` |
| `blob` | `Blob` metadata + `storage_blobs` row ops (raw SQL) |
| `attachments` | polymorphic `has_one`/`has_many` helpers over `storage_attachments` |
| `signing` | HMAC-SHA256 signed ids / signed URLs (`Signer`) |
| `content_type` | dependency-free content-type detection (Marcel analogue) |
| `checksum` | base64 MD5 + byte size |
| `schema` | `ensure_tables` (SQLite convenience; canonical path is the generator migration) |
| `serving` | axum handlers: redirect, proxy, disk PUT, direct uploads |
| `analyzer` | image dimensions (feature `storage-image`) |
| `jobs` | `purge_later` / `analyze_later` (feature `storage-jobs`, uses `doido-jobs`) |
| `client` | `Storage` facade (also the axum state) |

## Services (config)

Configured in the `storage` section of `config/<env>.yml`:

```yaml
storage:
  service: local                 # active service this environment
  services:
    local:  { type: disk, root: storage }
    test:   { type: memory }
    amazon: { type: s3, bucket: my-bucket, region: us-east-1 }
    cloudflare:
      type: r2
      bucket: my-bucket
      endpoint: "https://<accountid>.r2.cloudflarestorage.com"
    azure:  { type: azure, container: my-container, account: my-account }
    google: { type: gcs, bucket: my-bucket }   # auth via ADC
```

Credentials come from the config or the standard environment variables (AWS keys,
`AZURE_STORAGE_ACCESS_KEY`, `GOOGLE_APPLICATION_CREDENTIALS` for GCS ADC) and must
not be committed. `disk` and `memory` are always available; `s3`/`r2` need
`storage-s3`, `azure` needs `storage-azure`, `gcs` needs `storage-gcs`. Selecting a
backend whose feature is off yields a clear error.

## Custom services (external integrations)

Any external file service is a first-class backend: implement the `Service` trait,
register a factory under a `type` string at boot, then select it from config.

```rust
use doido_storage::{register_adapter, Service, ServiceConfig};
use std::sync::Arc;

register_adapter("dropbox", |name: &str, cfg: &ServiceConfig| {
    let _token = cfg.option_str("token");        // arbitrary YAML keys via cfg.options
    Ok(Arc::new(DropboxService::connect(name, cfg)?) as Arc<dyn Service>)
});
```

```yaml
storage:
  service: files
  services:
    files: { type: dropbox, token: "...", root: "/app" }
```

Unrecognized `type` values deserialize to `ServiceBackend::Custom(kind)` and resolve
through the registry (a clear error names the missing adapter if none is registered).
Scaffold a skeleton with `doido generate storage:adapter <Name>`, which writes
`app/storage/<name>_service.rs` (a `Service` impl + `register()` to call at boot).

## API

```rust
use doido_storage::{Storage, Disposition};

let storage = Storage::from_config(conn).await?; // service + signer from config/env
storage.ensure_tables().await?;                  // dev/test only

// Upload + attach (has_many append; use one()/detach() for has_one).
let blob = storage
    .attach_upload("User", "1", "avatar", "me.png", bytes)
    .await?;

let avatar = storage.one("User", "1", "avatar").await?;   // Option<Blob>
let photos = storage.many("Post", "9", "photos").await?;  // Vec<Blob>

let url = storage.url_for(&blob, Disposition::Inline).await?; // native or proxy URL
storage.purge_for_record("User", "1").await?;                // dependent purge
```

## Serving

`doido_storage::serving::routes(storage)` returns an axum `Router` mounted under
the prefix (default `/doido/storage`):

* `GET  {prefix}/blobs/redirect/{signed_id}/{filename}` — 302 to the service's
  native URL (S3/Azure presigned) or to the proxy route (disk/memory).
* `GET  {prefix}/blobs/proxy/{signed_id}/{filename}` — stream through the app.
* `PUT  {prefix}/disk/{token}` — receive a disk/memory direct upload.
* `POST {prefix}/direct_uploads` — create a blob, return a signed upload URL.

Signed ids/URLs are HMAC-SHA256 over `DOIDO_SECRET_KEY_BASE`.

## Generators

- `doido generate storage:install` emits a timestamped migration creating
  `storage_blobs`, `storage_attachments`, `storage_variant_records` (via the
  `doido_model::migration` builders), registers it in `db/migration/src/lib.rs`, and
  appends a `storage:` section to `config/development.yml` / `config/test.yml`.
- `doido generate storage:adapter <Name>` scaffolds a custom `Service` adapter at
  `app/storage/<name>_service.rs` (impl skeleton + `register()`).

## TDD surface

- `disk_test` / `memory_test` — the `Service` contract (round-trip, delete
  idempotency, key safety).
- `blob_test` — checksum/content-type/size on upload; purge removes object + row.
- `attachments_test` — has_one replace, has_many append, dependent purge.
- `config_test` — YAML parse + build; gcs/custom parse; cloud backend without its feature errors.
- `registry_test` — custom adapter selected via `type` + reads `cfg.options`; unregistered kind errors.
- `signing_test` — sign/verify, wrong key/purpose/tamper/expiry.
- `serving_test` — proxy 200 + bytes, redirect 307 → proxy, direct upload round-trip.
- `schema_test` — `ensure_tables` idempotent.
- `content_type` / `checksum` / `analyzer` — inline unit tests.

## Deferred (backlog)

Image variants (`variant(resize_to_limit:)`, named variants, `representation`),
PDF/video previews, video/audio analyzers, the Mirror service, native Azure SAS
URLs, a `#[has_one_attached]` proc-macro, and `compose`.
