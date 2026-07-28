+++
title = "Storage"
description = "Attached-file storage: blobs, polymorphic attachments, pluggable services, and signed URLs."
weight = 12
+++

> **Design spec:** [`docs/15-storage.md`](https://github.com/doido-rs/doido/blob/master/docs/15-storage.md).
> This guide documents the API as implemented in `doido-storage`.

**Rails analogue: Active Storage.** Store file bytes in a pluggable service (disk, memory,
S3/R2, Azure, GCS), track metadata (blobs, checksums, content types) in the database, attach
files polymorphically to any record, and serve them through signed URLs. The `Storage`
facade bundles a service, a database connection, and a signer.

## At a glance

```rust
use doido_storage::{Storage, DiskService, MemoryService, Signer, Blob, Disposition};
```

## Services

`Service` is the pluggable backend trait. `DiskService` (local filesystem) is the default;
`MemoryService` is for tests; `S3Service` (feature `storage-s3`, also Cloudflare R2),
`AzureBlobService` (`storage-azure`), and `GcsService` (`storage-gcs`) cover the cloud. Pick
the active service in the `storage` config; register custom ones with `register_adapter`.

```yaml
# config/production.yml
storage:
  service: amazon
  services:
    local:  { type: disk, root: storage }
    test:   { type: memory }
    amazon: { type: s3, bucket: my-bucket, region: us-east-1 }
    r2:     { type: r2, bucket: my-bucket, endpoint: "https://<id>.r2.cloudflarestorage.com" }
    azure:  { type: azure, container: my-container, account: my-account }
    google: { type: gcs, bucket: my-bucket }
```

Custom adapter:

```rust
use doido_storage::register_adapter;

register_adapter("dropbox", my_dropbox_factory);
```

## The Storage facade

Build a `Storage` from a connection, a service, and a `Signer`, or from config with
`from_config`. `ensure_tables` creates the `storage_blobs` / `storage_attachments` metadata
tables if missing.

```rust
use doido_storage::{Storage, MemoryService, Signer};
use std::sync::Arc;

let storage = Storage::new(conn, Arc::new(MemoryService::new("test")), Signer::from_env());
storage.ensure_tables().await?;

// …or straight from the storage config section:
let storage = Storage::from_config(conn).await?;
```

## Uploading & downloading

`create_and_upload` detects the content type, computes an MD5 checksum, stores the bytes,
and records a `Blob`. `download` reads them back; `purge` deletes the object, its attachment
rows, and the blob metadata.

```rust
let blob = storage.create_and_upload("hello.txt", b"hi".to_vec(), None).await?;
let bytes = storage.download(&blob.key).await?;   // b"hi"
storage.purge(&blob.key).await?;
```

A `Blob` records the `key`, `filename`, `content_type`, `byte_size`, `checksum`, and
`service_name`.

## Attachments

Attach blobs to any record polymorphically by `(record_type, record_id, name)` — the
`has_one_attached` / `has_many_attached` analogue.

```rust
// Upload and attach in one call:
let blob = storage.attach_upload("User", "1", "avatar", "me.png", png_bytes).await?;

// has_one: the attached blob, if any
let avatar = storage.one("User", "1", "avatar").await?;      // Option<Blob>
// has_many: all attached blobs
let photos = storage.many("Post", "42", "photos").await?;    // Vec<Blob>

storage.attach("User", "1", "avatar", &existing_key).await?; // attach an existing blob
storage.purge_for_record("User", "1").await?;                // detach + purge all
```

## Signed IDs & URLs

The `Signer` mints tamper-proof, optionally-expiring signed IDs (HMAC-SHA256). `url_for`
returns a native presigned URL (S3/Azure/GCS) or a signed application route (disk/memory),
with an inline or attachment `Disposition`.

```rust
use doido_storage::Disposition;

let signed = storage.signed_id(&blob.key);           // opaque, verifiable id
let key = storage.verify_signed_id(&signed)?;        // back to the key (errors if tampered)

let url = storage.url_for(&avatar, Disposition::Inline).await?;      // view in browser
let dl  = storage.url_for(&doc, Disposition::Attachment).await?;     // force download
```

## Serving

`doido-storage` provides axum routes for serving blobs — a redirect endpoint (302 to the
service URL), a proxy endpoint (streams bytes through the app), and direct-upload endpoints
(`create` a blob + return a presigned PUT URL) — so browsers can upload straight to the
service.

## Background jobs

With feature `storage-jobs`, `purge_later` enqueues cleanup onto [doido-jobs](@/docs/guides/jobs.md)
instead of blocking the request.

```rust
use doido_storage::jobs::purge_later;

purge_later(job_queue.as_ref(), &blob.key).await?;
```

## Testing

`MemoryService` keeps bytes in-process, so upload/attach/download round-trips are fast and
isolated.

```rust
use doido_storage::{DiskService, Service};

let service = DiskService::new("local", std::env::temp_dir());
service.upload("k", b"bytes".to_vec(), Some("text/plain")).await?;
assert_eq!(service.download("k").await?, b"bytes");
```

## Spec vs. implementation

> Image **variants**, PDF/video **previews**, media **analyzers**, and the Mirror service
> are **deferred** — the `storage_variant_records` table is scaffolded but the variant
> machinery is not built. Core blobs, polymorphic attachments, signing, direct upload, and
> serving are all implemented.

## See also

- [Models](@/docs/guides/models.md) — attachments reference your records; `ensure_tables`.
- [Jobs](@/docs/guides/jobs.md) — `purge_later` and background cleanup.
- [Generators & CLI](@/docs/guides/generators.md) — `doido generate storage:install` / `storage:adapter`.
