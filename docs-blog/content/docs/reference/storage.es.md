+++
title = "Storage"
description = "Almacenamiento de archivos adjuntos: blobs, attachments polimórficos, services conectables y URLs firmadas."
weight = 12
+++

> **Especificación de diseño:** [`docs/15-storage.md`](https://github.com/doido-rs/doido/blob/master/docs/15-storage.md).
> Esta guía documenta la API tal como está implementada en `doido-storage`.

**Análogo en Rails: Active Storage.** Almacena los bytes de los archivos en un service
conectable (disco, memoria, S3/R2, Azure, GCS), rastrea metadatos (blobs, checksums, content
types) en la base de datos, adjunta archivos de forma polimórfica a cualquier registro y
sírvelos mediante URLs firmadas. La fachada `Storage` reúne un service, una conexión a la
base de datos y un signer.

## Vistazo general

```rust
use doido::storage::{Storage, DiskService, MemoryService, Signer, Blob, Disposition};
```

## Services

`Service` es el trait conectable de backend. `DiskService` (sistema de archivos local) es el
por defecto; `MemoryService` es para pruebas; `S3Service` (feature `storage-s3`, también
Cloudflare R2), `AzureBlobService` (`storage-azure`) y `GcsService` (`storage-gcs`) cubren la
nube. Elige el service activo en la config `storage`; registra los personalizados con
`register_adapter`.

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

Adapter personalizado:

```rust
use doido::storage::register_adapter;

register_adapter("dropbox", my_dropbox_factory);
```

## La fachada Storage

Construye un `Storage` a partir de una conexión, un service y un `Signer`, o desde la config
con `from_config`. `ensure_tables` crea las tablas de metadatos `storage_blobs` /
`storage_attachments` si faltan.

```rust
use doido::storage::{Storage, MemoryService, Signer};
use std::sync::Arc;

let storage = Storage::new(conn, Arc::new(MemoryService::new("test")), Signer::from_env());
storage.ensure_tables().await?;

// …o directamente desde la sección de config storage:
let storage = Storage::from_config(conn).await?;
```

## Subida y descarga

`create_and_upload` detecta el content type, calcula un checksum MD5, almacena los bytes y
registra un `Blob`. `download` los lee de vuelta; `purge` elimina el objeto, sus filas de
attachment y los metadatos del blob.

```rust
let blob = storage.create_and_upload("hello.txt", b"hi".to_vec(), None).await?;
let bytes = storage.download(&blob.key).await?;   // b"hi"
storage.purge(&blob.key).await?;
```

Un `Blob` registra la `key`, el `filename`, el `content_type`, el `byte_size`, el `checksum`
y el `service_name`.

## Attachments

Adjunta blobs a cualquier registro de forma polimórfica por `(record_type, record_id,
name)` — el análogo de `has_one_attached` / `has_many_attached`.

```rust
// Subida y attach en una llamada:
let blob = storage.attach_upload("User", "1", "avatar", "me.png", png_bytes).await?;

// has_one: el blob adjunto, si lo hay
let avatar = storage.one("User", "1", "avatar").await?;      // Option<Blob>
// has_many: todos los blobs adjuntos
let photos = storage.many("Post", "42", "photos").await?;    // Vec<Blob>

storage.attach("User", "1", "avatar", &existing_key).await?; // adjunta un blob existente
storage.purge_for_record("User", "1").await?;                // desadjunta + purga todo
```

## IDs y URLs firmadas

El `Signer` genera IDs firmados a prueba de manipulación y opcionalmente con expiración
(HMAC-SHA256). `url_for` devuelve una URL prefirmada nativa (S3/Azure/GCS) o una ruta firmada
de la aplicación (disk/memory), con un `Disposition` inline o attachment.

```rust
use doido::storage::Disposition;

let signed = storage.signed_id(&blob.key);           // id opaco y verificable
let key = storage.verify_signed_id(&signed)?;        // vuelve a la key (error si se manipula)

let url = storage.url_for(&avatar, Disposition::Inline).await?;      // ver en el navegador
let dl  = storage.url_for(&doc, Disposition::Attachment).await?;     // forzar descarga
```

## Serving

`doido-storage` provee rutas axum para servir blobs — un endpoint de redirect (302 a la URL
del service), un endpoint de proxy (hace streaming de los bytes a través de la app) y
endpoints de direct-upload (`create` un blob + devuelve una URL PUT prefirmada) — para que
los navegadores puedan subir directamente al service.

## Jobs en segundo plano

Con la feature `storage-jobs`, `purge_later` encola la limpieza en
[doido-jobs](@/docs/reference/jobs.es.md) en vez de bloquear la petición.

```rust
use doido::storage::jobs::purge_later;

purge_later(job_queue.as_ref(), &blob.key).await?;
```

## Pruebas

`MemoryService` mantiene los bytes en proceso, así que los round-trips de
subida/attach/descarga son rápidos y aislados.

```rust
use doido::storage::{DiskService, Service};

let service = DiskService::new("local", std::env::temp_dir());
service.upload("k", b"bytes".to_vec(), Some("text/plain")).await?;
assert_eq!(service.download("k").await?, b"bytes");
```

## Especificación vs. implementación

> Las **variantes** de imagen, los **previews** de PDF/vídeo, los **analizadores** de medios
> y el service Mirror están **aplazados** — la tabla `storage_variant_records` está esbozada
> pero la maquinaria de variantes no está construida. Los blobs, attachments polimórficos,
> la firma, el direct upload y el serving están todos implementados.

## Véase también

- [Modelos](@/docs/reference/models.es.md) — los attachments referencian tus registros; `ensure_tables`.
- [Jobs](@/docs/reference/jobs.es.md) — `purge_later` y limpieza en segundo plano.
- [Generadores y CLI](@/docs/reference/generators.es.md) — `doido generate storage:install` / `storage:adapter`.
