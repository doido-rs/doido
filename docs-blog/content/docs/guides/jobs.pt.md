+++
title = "Jobs"
description = "Jobs em background com a macro #[job], filas plugáveis, workers, retries com backoff e uma dead-letter queue."
weight = 8
+++

> **Especificação de design:** [`docs/09-jobs.md`](https://github.com/doido-rs/doido/blob/master/docs/09-jobs.md).
> Este guia documenta a API como implementada em `doido-jobs`.

**Análogo no Rails: Active Job.** Jobs em background são funções async comuns anotadas com
`#[job]`; elas são enfileiradas em uma fila plugável (em memória, banco de dados ou Redis) e
executadas por um worker que cuida de concorrência, timeouts por job, retries com backoff,
leasing confiável e uma dead-letter queue.

## Visão geral

```rust
use doido_jobs::{job, JobQueue, JobPayload, MemoryQueue, Worker};
use std::sync::Arc;
```

## Definindo um job

`#[job]` grava a fila, a política de retry, o backoff, o timeout e a prioridade no job e
gera um helper `<name>_enqueue`. O payload do job é seu último parâmetro tipado (padrão
`serde_json::Value`), que precisa ser `Serialize`.

```rust
use doido_jobs::job;

#[job(queue = "emails", max_retries = 5, backoff = "exponential", backoff_base = 5, timeout = 30, priority = 7)]
async fn send_welcome(user_id: i64) -> doido_core::Result<()> {
    // …envia o e-mail de boas-vindas…
    Ok(())
}
```

## Enfileirando

O `<name>_enqueue(queue, payload)` gerado serializa o payload e o enfileira, retornando um
`JobId`.

```rust
let queue = Arc::new(MemoryQueue::new());
let id = send_welcome_enqueue(queue.as_ref(), 42).await?; // imediato
```

Para entrega **adiada** ou **agendada**, construa um `JobPayload` e use os setters fluentes
(`with_wait`, `with_run_at`, `with_priority`, …):

```rust
use doido_jobs::JobPayload;

let job = JobPayload::new("emails", serde_json::json!({ "user_id": 42 }), 5)
    .with_wait(300)          // roda em 5 minutos
    .with_priority(9);
queue.enqueue(job).await?;

// …ou um horário absoluto:
queue.enqueue_at(JobPayload::new("emails", payload, 5), when).await?;
```

## Escolhendo um backend

`JobQueue` é o trait plugável; escolha um backend com `build_queue(&JobsConfig)` (lê a seção
`jobs` da config) ou construa um diretamente. `MemoryQueue` está sempre disponível; `DbQueue`
(feature `jobs-db`) e `RedisQueue` (feature `jobs-redis`) adicionam durabilidade.

```rust
use doido_jobs::{build_queue, JobsConfig};

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

## Rodando o worker

O worker reserva jobs (fazendo lease, de modo que os jobs de um worker que caiu sejam
recuperados) e roda o seu handler. Use `WorkerEngine` para controle de multi-fila/
concorrência, ou `Worker` para uma única fila; `run_once` drena uma vez (ótimo para
testes), `run` roda em loop até o shutdown. O comando `doido worker` o executa como
processo.

```rust
use doido_jobs::{Worker, WorkerEngine, EngineConfig};

// Fila única:
Worker::new(queue.clone(), "emails")
    .run_once(|job, _ctx| async move {
        // despacha por job.queue / job.payload e executa o trabalho
        Ok(())
    })
    .await?;

// Engine multi-fila com concorrência, rodando até o Ctrl-C:
let engine = WorkerEngine::new(queue.clone(), EngineConfig::default());
engine.run(handler, tokio::signal::ctrl_c().map(|_| ())).await?;
```

```bash
doido worker            # processa jobs continuamente
doido worker --once     # drena a fila e sai
```

## Retries & backoff

Jobs que falham são retentados até `max_retries` com a `BackoffStrategy` configurada
(`Exponential`, `Linear` ou `None`) e `backoff_base` segundos — tudo definido no `#[job]`. O
worker dá `nack` numa falha com o `retry_at` calculado, e `reclaim_expired` reexecuta leases
perdidos por uma queda.

```rust
#[job(max_retries = 5, backoff = "exponential", backoff_base = 10)]
async fn charge_card(order_id: i64) -> doido_core::Result<()> { Ok(()) }
```

## Dead-letter queue

Jobs que esgotam seus retries são movidos para um store de dead-letter, inspecionável e
reexecutável via CLI.

```bash
doido jobs:failed            # lista jobs na dead-letter
doido jobs:retry <job_id>    # reenfileira um
doido jobs:discard <job_id>  # descarta um
```

## Prioridade & agendamento

`priority` (maior roda antes) é definido no `#[job]` ou por payload com `with_priority`;
`with_wait(secs)` / `with_run_at(datetime)` agendam execução futura.

## Contexto do job

Um `JobContext` carrega estado compartilhado (e uma conexão de banco com `jobs-db`) para os
handlers; um `WorkerEngine` pode ser construído `with_context` para injetar estado da app.

```rust
use doido_jobs::{JobContext, WorkerEngine, EngineConfig};

let engine = WorkerEngine::with_context(queue, EngineConfig::default(), JobContext::new());
```

## Testes

`MemoryQueue` mais `reserve` / `run_once` tornam os jobs determinísticos em testes —
enfileire, depois drene e verifique.

```rust
let queue = Arc::new(MemoryQueue::new());
send_welcome_enqueue(queue.as_ref(), 1).await?;
let reserved = queue.reserve(&["emails"], std::time::Duration::from_millis(50)).await?;
assert!(reserved.is_some());
```

## Especificação vs. implementação

> O enqueue é uma **função livre gerada** (`send_welcome_enqueue(queue, payload)`), e não um
> método no struct do job; o formato `Job.perform_later` da especificação mapeia para esse
> helper mais os setters fluentes de `JobPayload`.

## Veja também

- [Mailer](@/docs/guides/mailer.pt.md) — `deliver_later` enfileira na fila `mailers`.
- [Cache](@/docs/guides/cache.pt.md) & [Models](@/docs/guides/models.pt.md) — backends Redis/DB.
- [Geradores & CLI](@/docs/guides/generators.pt.md) — `doido generate job` e `doido worker`.
