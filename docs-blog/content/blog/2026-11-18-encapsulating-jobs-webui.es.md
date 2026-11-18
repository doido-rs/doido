+++
title = "Configuración del WebUI de Jobs en una sola línea en Doido"
description = "Encapsulando configuración, resolución síncrona de colas, rutas de montaje y middleware de autenticación para paneles de tareas en segundo plano."
date = 2026-11-18

[taxonomies]
tags = ["release", "jobs", "webui", "docs"]
+++

Las colas en segundo plano de Doido (`doido-jobs`) ofrecen un panel HTTP integrado para inspeccionar conteos de colas, ejecuciones activas, reintentos programados y dead letters.

Anteriormente, conectar este panel al router de su aplicación requería código repetitivo: cargar la configuración YAML, resolver el driver de cola activo (verificando la disponibilidad del pool de base de datos o recurriendo a colas en memoria), normalizar rutas de montaje, construir `JobsWebState` y adjuntar middleware de autenticación.

Con la incorporación de `build_jobs_webui_router`, toda esta configuración queda encapsulada en una sola llamada limpia.

## El problema: más de 30 líneas de configuración repetitiva

Previamente, una aplicación requería código similar a este para construir y proteger el router del panel:

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

## La solución: `build_jobs_webui_router`

`doido-jobs` ahora proporciona `build_jobs_webui_router` (y `build_unprotected_jobs_webui_router`), gestionando todo el ciclo de vida:

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

Si la ruta del panel ya está definida en `config/<env>.yml` bajo `jobs.webui.mount_path`, puede pasar `None` para usar esa ruta directamente:

```rust
let router = doido_jobs::webui::build_jobs_webui_router(None, auth_layer);
```

Y para entornos no protegidos o prototipos locales, `build_unprotected_jobs_webui_router(None)` no requiere ninguna anotación genérica de tipos.

## Resolución síncrona de colas: `default_sync_queue`

Internamente, la resolución de cola también se exporta como `doido_jobs::config::default_sync_queue(&jobs_config)`. Examina `jobs.type` en su configuración:
- Para `memory`, instancia `MemoryQueue` con el tamaño de lote configurado.
- Para `db` (con la feature `jobs-db` activa), verifica si hay una conexión lista mediante `doido_model::pool::try_pool()` y la envuelve en `DbQueue`; si no hay pool instalado, utiliza de forma segura `MemoryQueue`.
- Para otros drivers, proporciona respaldo seguro a memoria.

## Más información

Consulte la guía completa de tareas en segundo plano en el manual [Referencia de Jobs](/docs/reference/jobs/).
