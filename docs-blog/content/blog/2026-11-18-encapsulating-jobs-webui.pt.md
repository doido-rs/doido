+++
title = "Configuração do WebUI de Jobs em uma linha no Doido"
description = "Encapsulando configuração, resolução síncrona de filas, caminhos de montagem e middleware de autenticação para dashboards de jobs em segundo plano."
date = 2026-11-18

[taxonomies]
tags = ["release", "jobs", "webui", "docs"]
+++

As filas em segundo plano do Doido (`doido-jobs`) oferecem um dashboard HTTP integrado para inspecionar contagens de filas, execuções ativas, agendamentos de retry e dead letters.

Anteriormente, plugar esse dashboard no router da sua aplicação exigia código repetitivo: carregar as configurações YAML, resolver o driver de fila ativo (verificando a disponibilidade do pool de banco de dados ou recorrendo a filas em memória), normalizar URLs de montagem, construir `JobsWebState` e anexar middleware de autenticação.

Com o lançamento de `build_jobs_webui_router`, toda essa configuração foi encapsulada em uma única chamada limpa.

## O problema: mais de 30 linhas de código repetitivo

Anteriormente, uma aplicação precisava de um código como este apenas para instanciar e proteger o router do dashboard:

```rust
pub fn build_jobs_webui_router() -> axum::Router {
    let jobs_config = doido_jobs::config::load();
    let queue: std::sync::Arc<dyn doido_jobs::JobQueue> = match jobs_config.backend {
        doido_jobs::config::Backend::Memory => std::sync::Arc::new(
            doido_jobs::MemoryQueue::new().with_enqueue_batch_size(jobs_config.enqueue_batch_size),
        ),
        doido_jobs::config::Backend::Db => {
            if let Some(conn) = doido_model::pool::try_pool().cloned() {
                doido_jobs::config::build_db_queue(conn)
            } else {
                std::sync::Arc::new(
                    doido_jobs::MemoryQueue::new()
                        .with_enqueue_batch_size(jobs_config.enqueue_batch_size),
                )
            }
        }
        #[allow(unreachable_patterns)]
        _ => std::sync::Arc::new(
            doido_jobs::MemoryQueue::new().with_enqueue_batch_size(jobs_config.enqueue_batch_size),
        ),
    };

    let mount_path =
        doido_jobs::webui::normalize_mount(crate::settings::get().jobs.webui.mount_path());
    let state =
        doido_jobs::webui::JobsWebState::new(queue, jobs_config).with_mount_path(&mount_path);

    let mut router = doido_jobs::webui::routes_at(&mount_path, state);
    if !crate::settings::dev_or_test_runtime() {
        router = router.layer(from_fn(crate::auth::jobs_webui_auth_layer));
    }
    router
}
```

## A solução: `build_jobs_webui_router`

O `doido-jobs` agora oferece `build_jobs_webui_router` (e `build_unprotected_jobs_webui_router`), gerenciando todo esse ciclo:

```rust
use axum::middleware::from_fn;

pub fn build_jobs_webui_router() -> axum::Router {
    let auth_layer = (!crate::settings::dev_or_test_runtime())
        .then(|| from_fn(crate::auth::jobs_webui_auth_layer));

    doido_jobs::webui::build_jobs_webui_router(
        Some(crate::settings::get().jobs.webui.mount_path()),
        auth_layer,
    )
}
```

Se o endpoint do seu dashboard já estiver definido no `config/<env>.yml` sob `jobs.webui.mount_path`, você pode passar `None` para usar esse caminho diretamente:

```rust
let router = doido_jobs::webui::build_jobs_webui_router(None, auth_layer);
```

E para ambientes sem autenticação ou protótipos locais, `build_unprotected_jobs_webui_router(None)` não requer nenhuma anotação de tipo genérico.

## Resolução síncrona de fila: `default_sync_queue`

Por baixo dos panos, a resolução de fila também está exposta como `doido_jobs::config::default_sync_queue(&jobs_config)`. Ela avalia a configuração `jobs.type`:
- Para `memory`, instancia `MemoryQueue` com o tamanho de lote configurado.
- Para `db` (com a feature `jobs-db` ativa), verifica se há uma conexão ativa via `doido_model::pool::try_pool()` e a encapsula em `DbQueue`; caso nenhum pool esteja inicializado, recorre com segurança a `MemoryQueue`.
- Para outros drivers, fornece fallback seguro para memória.

## Saiba mais

Consulte o guia completo no manual [Referência de Jobs](/docs/reference/jobs/).
