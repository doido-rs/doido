+++
title = "Storage"
description = "Armazenamento de arquivos anexados: blobs, attachments polimórficos, services plugáveis e URLs assinadas."
weight = 12
+++

> **Especificação de design:** [`docs/15-storage.md`](https://github.com/doido-rs/doido/blob/master/docs/15-storage.md).
> Este guia documenta a API como implementada em `doido-storage`.

**Análogo no Rails: Active Storage.** Armazene os bytes de arquivos em um service plugável
(disco, memória, S3/R2, Azure, GCS), rastreie metadados (blobs, checksums, content types) no
banco, anexe arquivos de forma polimórfica a qualquer registro e sirva-os por URLs
assinadas. A fachada `Storage` reúne um service, uma conexão de banco e um signer.

## Visão geral

```rust
use doido_storage::{Storage, DiskService, MemoryService, Signer, Blob, Disposition};
```

## Services

`Service` é o trait plugável de backend. `DiskService` (sistema de arquivos local) é o
padrão; `MemoryService` é para testes; `S3Service` (feature `storage-s3`, também Cloudflare
R2), `AzureBlobService` (`storage-azure`) e `GcsService` (`storage-gcs`) cobrem a nuvem.
Escolha o service ativo na config `storage`; registre os customizados com `register_adapter`.

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

Adapter customizado:

```rust
use doido_storage::register_adapter;

register_adapter("dropbox", my_dropbox_factory);
```

## A fachada Storage

Construa um `Storage` a partir de uma conexão, um service e um `Signer`, ou a partir da
config com `from_config`. `ensure_tables` cria as tabelas de metadados `storage_blobs` /
`storage_attachments` se ausentes.

```rust
use doido_storage::{Storage, MemoryService, Signer};
use std::sync::Arc;

let storage = Storage::new(conn, Arc::new(MemoryService::new("test")), Signer::from_env());
storage.ensure_tables().await?;

// …ou direto da seção de config storage:
let storage = Storage::from_config(conn).await?;
```

## Upload & download

`create_and_upload` detecta o content type, calcula um checksum MD5, armazena os bytes e
registra um `Blob`. `download` os lê de volta; `purge` deleta o objeto, suas linhas de
attachment e os metadados do blob.

```rust
let blob = storage.create_and_upload("hello.txt", b"hi".to_vec(), None).await?;
let bytes = storage.download(&blob.key).await?;   // b"hi"
storage.purge(&blob.key).await?;
```

Um `Blob` registra a `key`, o `filename`, o `content_type`, o `byte_size`, o `checksum` e o
`service_name`.

## Attachments

Anexe blobs a qualquer registro de forma polimórfica por `(record_type, record_id, name)` —
o análogo a `has_one_attached` / `has_many_attached`.

```rust
// Upload e attach em uma chamada:
let blob = storage.attach_upload("User", "1", "avatar", "me.png", png_bytes).await?;

// has_one: o blob anexado, se houver
let avatar = storage.one("User", "1", "avatar").await?;      // Option<Blob>
// has_many: todos os blobs anexados
let photos = storage.many("Post", "42", "photos").await?;    // Vec<Blob>

storage.attach("User", "1", "avatar", &existing_key).await?; // anexa um blob existente
storage.purge_for_record("User", "1").await?;                // desanexa + purga tudo
```

## IDs & URLs assinadas

O `Signer` gera IDs assinados à prova de adulteração e opcionalmente expiráveis
(HMAC-SHA256). `url_for` retorna uma URL presigned nativa (S3/Azure/GCS) ou uma rota
assinada da aplicação (disk/memory), com um `Disposition` inline ou attachment.

```rust
use doido_storage::Disposition;

let signed = storage.signed_id(&blob.key);           // id opaco e verificável
let key = storage.verify_signed_id(&signed)?;        // volta à key (erro se adulterado)

let url = storage.url_for(&avatar, Disposition::Inline).await?;      // visualiza no navegador
let dl  = storage.url_for(&doc, Disposition::Attachment).await?;     // força download
```

## Serving

`doido-storage` fornece rotas axum para servir blobs — um endpoint de redirect (302 para a
URL do service), um endpoint de proxy (faz streaming dos bytes pela aplicação) e endpoints
de direct-upload (`create` um blob + retorna uma URL PUT presigned) — para que os
navegadores possam fazer upload direto para o service.

## Jobs em background

Com a feature `storage-jobs`, `purge_later` enfileira a limpeza em
[doido-jobs](@/docs/reference/jobs.pt.md) em vez de bloquear a requisição.

```rust
use doido_storage::jobs::purge_later;

purge_later(job_queue.as_ref(), &blob.key).await?;
```

## Testes

`MemoryService` mantém os bytes em processo, então os round-trips de upload/attach/download
são rápidos e isolados.

```rust
use doido_storage::{DiskService, Service};

let service = DiskService::new("local", std::env::temp_dir());
service.upload("k", b"bytes".to_vec(), Some("text/plain")).await?;
assert_eq!(service.download("k").await?, b"bytes");
```

## Especificação vs. implementação

> As **variantes** de imagem, os **previews** de PDF/vídeo, os **analisadores** de mídia e o
> service Mirror estão **adiados** — a tabela `storage_variant_records` está esboçada, mas a
> maquinaria de variantes não foi construída. Blobs, attachments polimórficos, assinatura,
> direct upload e serving estão todos implementados.

## Veja também

- [Models](@/docs/reference/models.pt.md) — os attachments referenciam seus registros; `ensure_tables`.
- [Jobs](@/docs/reference/jobs.pt.md) — `purge_later` e limpeza em background.
- [Geradores & CLI](@/docs/reference/generators.pt.md) — `doido generate storage:install` / `storage:adapter`.
