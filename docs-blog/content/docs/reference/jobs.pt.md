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
use doido::jobs::{job, JobQueue, JobPayload, MemoryQueue, Worker};
use std::sync::Arc;
```

## Definindo um job

`#[job]` grava a fila, a política de retry, o backoff, o timeout e a prioridade no job e
gera um helper `<name>_enqueue`. O payload do job é seu último parâmetro tipado (padrão
`serde_json::Value`), que precisa ser `Serialize`.

```rust
use doido::jobs::job;

#[job(queue = "emails", max_retries = 5, backoff = "exponential", backoff_base = 5, timeout = 30, priority = 7)]
async fn send_welcome(user_id: i64) -> doido::Result<()> {
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
use doido::jobs::JobPayload;

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

## Cron periódico

Com `cargo doido worker` (modo contínuo), o worker lê `jobs.cron` em `config/<env>.yml` na
inicialização, valida expressões cron e nomes de `#[job]`, e enfileira um job a cada tick
(horários **UTC**, seis campos: `sec min hour dom month dow`). Vários workers usam leader
lock (Redis ou advisory lock Postgres/MySQL) para não duplicar enqueue. Cron fica desligado
em `test` por padrão; `worker --once` não inicia o agendador.

```yaml
jobs:
  cron:
    tasks:
      - job_name: sync_catalog_daily
        schedule: "0 0 3 * * *"
        payload: {}
```

## Rodando o worker

O worker reserva jobs (fazendo lease, de modo que os jobs de um worker que caiu sejam
recuperados) e roda o seu handler. Use `WorkerEngine` para controle de multi-fila/
concorrência, ou `Worker` para uma única fila; `run_once` drena uma vez (ótimo para
testes), `run` roda em loop até o shutdown. O comando `cargo doido worker` o executa como
processo.

```rust
use doido::jobs::{Worker, WorkerEngine, EngineConfig};

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
cargo doido worker            # processa jobs continuamente
cargo doido worker --once     # drena a fila e sai
```

## Executar jobs pela CLI

Liste handlers linkados no binário da app, rode um no processo (Rails `perform_now`) ou
enfileire um job para o worker (`perform_later`).

| Rails | Doido |
|-------|-------|
| `perform_later` | `cargo doido jobs run <nome> --later` |
| `perform_now` | `cargo doido jobs run <nome>` |
| Processador | `cargo doido worker` / `cargo doido worker --once` |

```bash
cargo doido jobs list
cargo doido jobs run send_welcome --payload '42'
cargo doido jobs run send_welcome --payload '{"user_id":42}'
cargo doido jobs run send_welcome --later
```

`--payload` é JSON (padrão `{}`). Fila, timeout e max retries vêm dos atributos
`#[job(...)]`, salvo `--queue`, `--timeout` ou `--max-retries`. Execução in-process não
grava em `doido_jobs`; use `--later` e `worker --once` para processar de forma assíncrona.

## Retries & backoff

`attempts` sobe a cada `reserve`; `max_retries` limita o número total de **execuções**
(incluindo a primeira). Jobs que falham com tentativas restantes ficam em status **`Failed`**
na mesma fila, com `run_at` definido pela `BackoffStrategy` (`Exponential`, `Linear` ou
`None`) e `backoff_base` do `#[job]`. Quando `run_at` vence, o worker reserva de novo. O
engine chama `nack` com o `retry_at` calculado; `reclaim_expired` recupera leases expirados
após queda do processo.

```rust
#[job(max_retries = 5, backoff = "exponential", backoff_base = 10)]
async fn charge_card(order_id: i64) -> doido::Result<()> { Ok(()) }
```

## Dead-letter queue

Jobs que esgotam `max_retries` vão para status **`Dead`** no store de dead-letter,
inspecionável e reexecutável via CLI. `jobs retry` reenfileira cada dead job com id novo,
`tentativas` zeradas e **`job_name` preservado** (mesma política de retry/timeout).

```bash
cargo doido jobs failed    # lista jobs na dead-letter (tracing)
cargo doido jobs retry     # reenfileira todos os dead jobs
cargo doido jobs discard   # remove todos os dead jobs
```

## Web UI

Com `jobs.webui.enabled: true` em `config/<env>.yml` e `cargo doido server`, o framework
monta um dashboard com abas (padrão `/doido/jobs`) no mesmo backend de fila do worker:
contagens, fila, em execução, retry e dead letters. A aba **Dead** permite
retry/discard (por job e em lote). Em produção, defina `jobs.webui.username` e
`password` (HTTP Basic Auth) ou o dashboard não será montado. Feature `webui` em
`doido-jobs` (ativa no crate `doido`).

## Prioridade & agendamento

`priority` (maior roda antes) é definido no `#[job]` ou por payload com `with_priority`;
`with_wait(secs)` / `with_run_at(datetime)` agendam execução futura.

## Contexto do job

Um `JobContext` carrega estado compartilhado (e uma conexão de banco com `jobs-db`) para os
handlers; um `WorkerEngine` pode ser construído `with_context` para injetar estado da app.

```rust
use doido::jobs::{JobContext, WorkerEngine, EngineConfig};

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

- [Mailer](@/docs/reference/mailer.pt.md) — `deliver_later` enfileira na fila `mailers`.
- [Cache](@/docs/reference/cache.pt.md) & [Models](@/docs/reference/models.pt.md) — backends Redis/DB.
- [Geradores & CLI](@/docs/reference/generators.pt.md) — `cargo doido generate job` e `cargo doido worker`.
