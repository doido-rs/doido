# Otimização de latência — doido-jobs Web UI (`/doido/jobs`)

Documento de referência para diagnóstico e melhorias de performance do painel de filas em **produção** (carregamento observado ~7 s). Não prescreve alterações no QuickNFe além do que já está configurado; foco em entender o fluxo e priorizar alavancas.

## Contexto no QuickNFe

- O painel **não faz parte do frontend** (React/Vite). É servido pelo **servidor Doido** no mesmo processo da API.
- Dependência: `doido-jobs` **0.0.32-test3** com feature `webui` (`backend/Cargo.toml`).
- Configuração: `jobs.webui` em `backend/config/production.yml` — `enabled: true`, `mount_path: /doido/jobs`, credenciais via `JOBS_WEBUI_USERNAME` / `JOBS_WEBUI_PASSWORD` (HTTP Basic Auth).
- Filas configuradas: `default`, `catalog`, `mailers`.
- Montagem: crate `doido` chama `doido_jobs::webui::try_merge_routes` ao subir `cargo doido server`.

## O que acontece em cada carregamento

1. **Uma requisição GET** (`/doido/jobs` ou com query `tab`, `queue`, `page_token`, etc.).
2. **SSR**: template Tera embutido (`dashboard.html.tera`) + CSS inline — HTML completo, sem SPA pesada.
3. Handler `index` (`doido-jobs` → `webui/dashboard.rs`):
   - **Sempre** executa `queue_counts` para **cada fila** listada em `jobs.queues`.
   - Aba padrão `tab=queues`: só contagens + render.
   - Abas `in_queue`, `executing`, `retry`, `dead`: **mesmas contagens** + `list_jobs` (até **50** jobs por página).

Conclusão: latência percebida ≈ **TTFB do backend + PostgreSQL**, não download de assets.

## Gargalo principal: contagens SQL

Implementação em `DbQueue::queue_counts` (`doido-jobs` → `db.rs`):

- Para **cada fila**, **6 queries `COUNT(*)` em série**:
  - pending ready, scheduled, running, retry wait, failed ready, dead.
- Com 3 filas → **18 round-trips SQL por page view**, em **todas as abas** (inclusive quando o usuário só quer ver jobs na aba Dead).

Índice criado pelo migrate do crate: `(queue, status, run_at)`. Ajuda, mas não elimina o custo de muitas queries sequenciais nem o volume de linhas.

## Gargalo secundário: abas de jobs

- `list_jobs` lê a coluna **`data` (TEXT)** inteira por linha (JSON do job).
- O dashboard inclui **payload pretty-printed** no HTML (`serde_json::to_string_pretty` em `job_row`).
- Jobs da fila **`catalog`** (sync ADN / NFS-e) tendem a ser **numerosos** e com payloads grandes — ver `docs/nfse/catalog-sync.md` e volume típico refletido em `scripts/doido_jobs_prune_non_rj.sql`.

Paginação “Previous” pode executar **counts adicionais** (`list_jobs_before`).

## O que provavelmente não explica ~7 s

- Template HTML/CSS (pequeno, embutido).
- Basic Auth.
- Frontend QuickNFe.

## Hipóteses para ~7 s em produção

| Cenário | Mecanismo |
|--------|-----------|
| Aba **Queues** (default) | Tabela `doido_jobs` grande + **18 counts sequenciais** (~300–400 ms cada → ~6–7 s). |
| Outras abas | 18 counts + até 50 leituras de `data` + HTML grande + CPU de pretty-print. |
| Infra | Latência app ↔ PostgreSQL (região/pool); contenção I/O com **worker** processando `catalog`. |

## Diagnóstico recomendado (sem mudar código)

1. **Logs**: em produção, `logger.format: json_response` expõe `latency_ms` — filtrar `GET /doido/jobs` e query `tab`.
2. **Browser**: Network → **Waiting (TTFB)** na primeira resposta; confirma se o gargalo é servidor/DB.
3. **PostgreSQL**:
   ```sql
   SELECT queue, status, COUNT(*) FROM doido_jobs GROUP BY 1, 2 ORDER BY 3 DESC;
   SELECT pg_size_pretty(pg_total_relation_size('doido_jobs'));
   ```
4. **`EXPLAIN (ANALYZE, BUFFERS)`** em um count equivalente (ex.: `queue = 'catalog'`, `status = 'pending'`, `run_at <= now()`).

## Otimizações por camada

### Operação / dados (QuickNFe prod — maior alavanca imediata)

- Medir e reduzir volume em `doido_jobs` (retention, limpeza de `dead`, jobs obsoletos de sync).
- Manter autovacuum / `VACUUM ANALYZE` saudável em tabela quente.
- Colocar app e PostgreSQL na **mesma região**; revisar tamanho do pool sob carga worker + webui.

### Upstream (`doido-jobs` / Doido — ganho estrutural)

QuickNFe só se beneficia ao **atualizar a dependência**; hoje não há knobs em YAML além de mount e auth.

| Melhoria | Benefício |
|----------|-----------|
| Agregar contagens em **1–2 SQL** (`GROUP BY queue, status`, buckets de `run_at`) | 18 queries → 1–2 |
| **Paralelizar** counts por fila ou por bucket | Menor wall-clock |
| Chamar `queue_counts` **somente na aba Queues** | Abas de jobs deixam de pagar 18 counts |
| Listagem **sem payload** (detalhe sob demanda) | Menos I/O em `data`, HTML e CPU |
| **Cache TTL** (5–15 s) de contagens no processo | Painel operacional tolera stale leve |
| Evitar counts extras na paginação “Previous” | Menos queries por navegação |

### Infra / produto

- Restringir acesso ao painel (rede interna/VPN) — não reduz TTFB, reduz risco.
- Alternativa operacional: CLI `cargo doido jobs list` / SQL direto quando o painel estiver lento por volume de dados.

## Escopo de código

| Onde | Papel |
|------|--------|
| QuickNFe | Habilita `jobs.webui` e filas; enqueue em `doido_jobs` (API/worker). |
| `doido-jobs` | Dashboard, queries, template, paginação. |
| PostgreSQL | Tamanho e saúde de `doido_jobs` dominam counts. |

## Prompt útil para agente / PR upstream

Use o texto abaixo ao pedir implementação no crate **doido-jobs** ou ao abrir issue no Doido:

---

**Objetivo:** reduzir latência do Jobs Web UI (`GET /doido/jobs`) em produção com tabela `doido_jobs` grande (centenas de milhares+ de linhas, fila `catalog`).

**Problema atual:**

- `dashboard::index` chama `queue_counts` em **todo** request, para cada fila em `jobs.queues`.
- `DbQueue::queue_counts` executa **6 `COUNT` sequenciais por fila** (18 queries com 3 filas).
- Abas de jobs ainda pagam esses counts e em seguida `list_jobs` (50 linhas) lendo coluna `data` completa; HTML inclui payload pretty-printed.

**Critérios de aceite sugeridos:**

1. Aba `queues`: no máximo **1–2 queries** agregadas para todas as filas configuradas (ou counts paralelos documentados).
2. Abas `in_queue` / `executing` / `retry` / `dead`: **não** executar `queue_counts` unless necessário para UI mínima.
3. Lista de jobs: colunas leves por padrão; payload opcional (query param ou segunda rota).
4. Testes de integração SQLite/Postgres com tabela sinteticamente grande ou mocks que provem número de queries.
5. Opcional: cache in-process de counts com TTL configurável (`jobs.webui.stats_ttl_secs`).

**Referência QuickNFe:** filas `default`, `catalog`, `mailers`; sync catalog documentado em `docs/nfse/catalog-sync.md`.

---

## Referências no repositório

- `backend/config/production.yml` — `jobs.webui`
- `backend/Cargo.toml` — `doido-jobs` + feature `webui`
- `docs/nfse/catalog-sync.md` — volume e fila `catalog`
- `scripts/doido_jobs_prune_non_rj.sql` — exemplo de limpeza em fila inchada
- Upstream (crate local em build): `doido-jobs-0.0.32-test3` — `src/webui/dashboard.rs`, `src/db.rs` (`queue_counts`, `list_jobs`)
