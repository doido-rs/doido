+++
title = "Jobs"
description = "Jobs en segundo plano con la macro #[job], colas conectables, workers, reintentos con backoff y una dead-letter queue."
weight = 8
+++

> **Especificación de diseño:** [`docs/09-jobs.md`](https://github.com/doido-rs/doido/blob/master/docs/09-jobs.md).
> Esta guía documenta la API tal como está implementada en `doido-jobs`.

**Análogo en Rails: Active Job.** Los jobs en segundo plano son funciones async normales
anotadas con `#[job]`; se encolan en una cola conectable (en memoria, base de datos o Redis)
y las ejecuta un worker que gestiona la concurrencia, los timeouts por job, los reintentos
con backoff, el leasing confiable y una dead-letter queue.

## Vistazo general

```rust
use doido::jobs::{job, JobQueue, JobPayload, MemoryQueue, Worker};
use std::sync::Arc;
```

## Definir un job

`#[job]` graba la cola, la política de reintentos, el backoff, el timeout y la prioridad en
el job y genera un helper `<name>_enqueue`. El payload del job es su último parámetro tipado
(por defecto `serde_json::Value`), que debe ser `Serialize`.

```rust
use doido::jobs::job;

#[job(queue = "emails", max_retries = 5, backoff = "exponential", backoff_base = 5, timeout = 30, priority = 7)]
async fn send_welcome(user_id: i64) -> doido::Result<()> {
    // …entregar el correo de bienvenida…
    Ok(())
}
```

## Encolar

El `<name>_enqueue(queue, payload)` generado serializa el payload y lo encola, devolviendo
un `JobId`.

```rust
let queue = Arc::new(MemoryQueue::new());
let id = send_welcome_enqueue(queue.as_ref(), 42).await?; // inmediato
```

Para entrega **diferida** o **programada**, construye un `JobPayload` y usa los setters
fluidos (`with_wait`, `with_run_at`, `with_priority`, …):

```rust
use doido::jobs::JobPayload;

let job = JobPayload::new("emails", serde_json::json!({ "user_id": 42 }), 5)
    .with_wait(300)          // se ejecuta en 5 minutos
    .with_priority(9);
queue.enqueue(job).await?;

// …o un momento absoluto:
queue.enqueue_at(JobPayload::new("emails", payload, 5), when).await?;
```

## Elegir un backend

`JobQueue` es el trait conectable; elige un backend con `build_queue(&JobsConfig)` (lee la
sección `jobs` de la config) o construye uno directamente. `MemoryQueue` siempre está
disponible; `DbQueue` (feature `jobs-db`) y `RedisQueue` (feature `jobs-redis`) añaden
durabilidad.

```rust
use doido::jobs::{build_queue, JobsConfig};

let queue = build_queue(&JobsConfig::default()).await?; // Arc<dyn JobQueue>
```

```yaml
# config/production.yml
jobs:
  backend: redis            # memory | db | redis
  queues: [critical, default, emails]
  concurrency: 8
  redis_url: redis://127.0.0.1:6379
```

## Ejecutar el worker

El worker reserva jobs (con lease, de modo que los jobs de un worker caído se recuperen) y
ejecuta tu handler. Usa `WorkerEngine` para control de multi-cola/concurrencia, o `Worker`
para una sola cola; `run_once` vacía una vez (ideal para pruebas), `run` itera hasta el
shutdown. El comando `cargo doido worker` lo ejecuta como proceso.

```rust
use doido::jobs::{Worker, WorkerEngine, EngineConfig};

// Cola única:
Worker::new(queue.clone(), "emails")
    .run_once(|job, _ctx| async move {
        // despacha según job.queue / job.payload y realiza el trabajo
        Ok(())
    })
    .await?;

// Engine multi-cola con concurrencia, ejecutándose hasta Ctrl-C:
let engine = WorkerEngine::new(queue.clone(), EngineConfig::default());
engine.run(handler, tokio::signal::ctrl_c().map(|_| ())).await?;
```

```bash
cargo doido worker            # procesa jobs continuamente
cargo doido worker --once     # vacía la cola y sale
```

## Reintentos y backoff

Los jobs que fallan se reintentan hasta `max_retries` con la `BackoffStrategy` configurada
(`Exponential`, `Linear` o `None`) y `backoff_base` segundos — todo definido en `#[job]`. El
worker hace `nack` de un fallo con el `retry_at` calculado, y `reclaim_expired` reejecuta
los leases perdidos por una caída.

```rust
#[job(max_retries = 5, backoff = "exponential", backoff_base = 10)]
async fn charge_card(order_id: i64) -> doido::Result<()> { Ok(()) }
```

## Dead-letter queue

Los jobs que agotan sus reintentos se mueven a un store de dead-letter, inspeccionable y
reejecutable vía CLI.

```bash
cargo doido jobs:failed            # lista los jobs en dead-letter
cargo doido jobs:retry <job_id>    # reencola uno
cargo doido jobs:discard <job_id>  # descarta uno
```

## Prioridad y programación

`priority` (mayor se ejecuta antes) se define en `#[job]` o por payload con `with_priority`;
`with_wait(secs)` / `with_run_at(datetime)` programan ejecución futura.

## Contexto del job

Un `JobContext` transporta estado compartido (y una conexión a la base de datos con
`jobs-db`) a los handlers; un `WorkerEngine` se puede construir `with_context` para inyectar
estado de la aplicación.

```rust
use doido::jobs::{JobContext, WorkerEngine, EngineConfig};

let engine = WorkerEngine::with_context(queue, EngineConfig::default(), JobContext::new());
```

## Pruebas

`MemoryQueue` más `reserve` / `run_once` hacen los jobs deterministas en las pruebas —
encola, luego vacía y verifica.

```rust
let queue = Arc::new(MemoryQueue::new());
send_welcome_enqueue(queue.as_ref(), 1).await?;
let reserved = queue.reserve(&["emails"], std::time::Duration::from_millis(50)).await?;
assert!(reserved.is_some());
```

## Especificación vs. implementación

> El encolado es una **función libre generada** (`send_welcome_enqueue(queue, payload)`), no
> un método en la struct del job; la forma `Job.perform_later` de la especificación se mapea
> a ese helper más los setters fluidos de `JobPayload`.

## Véase también

- [Mailer](@/docs/reference/mailer.es.md) — `deliver_later` encola en la cola `mailers`.
- [Cache](@/docs/reference/cache.es.md) y [Modelos](@/docs/reference/models.es.md) — backends Redis/DB.
- [Generadores y CLI](@/docs/reference/generators.es.md) — `cargo doido generate job` y `cargo doido worker`.
