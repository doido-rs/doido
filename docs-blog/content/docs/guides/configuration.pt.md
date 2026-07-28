+++
title = "Configuração"
description = "Config YAML por ambiente, acesso tipado e overrides por variável de ambiente."
weight = 2
+++

> **Especificação de design:** [`docs/05-config.md`](https://github.com/doido-rs/doido/blob/master/docs/05-config.md).
> Este guia documenta o caminho **implementado** (YAML por ambiente). Veja a nota no final
> para o que a especificação adia.

**Análogo no Rails: `config/`.** O Doido lê um único arquivo YAML por ambiente,
`config/<env>.yml`, e o expõe como configurações fortemente tipadas. Variáveis de
ambiente podem sobrescrever qualquer valor, então segredos e ajustes por deploy ficam
fora do arquivo. O tipo de config vive em `doido-controller`.

## Visão geral

```rust
use doido_controller::{Config, YamlConfig, ServerConfig};
use doido_controller::env_override::{apply_env_overrides, from_process_env};
```

## YAML por ambiente

Cada ambiente tem seu próprio arquivo em `config/`, selecionado por
`Environment::get_env()` (`development`, `test`, `production`). As seções de topo
reconhecidas são `server`, `logger` e `middleware`; outros subsistemas (cache, jobs,
storage, …) leem suas próprias seções do mesmo arquivo.

```yaml
# config/development.yml
server:
  bind: 0.0.0.0
  port: 3000
logger:
  level: debug
  format: verbose
  sql: true
middleware:
  cors:
    enabled: true
    allowed_origins: ["*"]
    allowed_methods: ["GET", "POST"]
```

## Acesso tipado

`YamlConfig` desserializa o arquivo em structs tipados. Carregue o arquivo do ambiente
atual com `load()` (com fallback para os padrões quando ausente, via a função livre
`config::load()`), um ambiente específico com `load_env()`, ou faça o parse de uma string
diretamente com `from_yaml()`. Acesse as seções pelo trait `Config`: `server()`,
`logger()`, `middleware()`.

```rust
use doido_controller::{Config, YamlConfig};

// Carrega config/<env-atual>.yml (ex.: config/development.yml).
let config = YamlConfig::load()?;

let addr = format!("{}:{}", config.server().bind, config.server().port); // "0.0.0.0:3000"
let level = &config.logger().level;                                      // "debug"
let cors_on = config.middleware().cors.enabled;                          // true

// Ou nunca falhe — fallback para os padrões quando o arquivo está ausente/inválido:
let config = doido_controller::config::load(); // Box<dyn Config>
```

`ServerConfig` tem padrão `0.0.0.0:3000`; `LoggerConfig` tem padrão `info` (veja
[Core](@/docs/guides/core.pt.md)); `MiddlewareConfig`/`CorsConfig` ficam desativados a
menos que habilitados.

## Overrides por variável de ambiente

Qualquer configuração pode ser sobrescrita por uma variável de ambiente chamada
`SECTION__KEY` (duplo underscore): `SERVER__PORT=4000` define `server.port`. Os valores
são convertidos para bool ou número quando fazem parse, senão são mantidos como string —
e uma nova seção é criada se ainda não existir.

```bash
# Override no boot — ideal para segredos e valores por deploy:
SERVER__PORT=4000 LOGGER__LEVEL=warn DATABASE__URL=postgres://... doido server
```

Os overrides são aplicados ao valor de config já parseado, antes da desserialização
tipada:

```rust
use doido_controller::env_override::{apply_env_overrides, from_process_env};

let mut value: serde_json::Value = serde_json::to_value(&raw_config)?;

// Puxa as variáveis SECTION__KEY direto do ambiente do processo:
from_process_env(&mut value);

// …ou aplique um conjunto explícito (útil em testes):
apply_env_overrides(&mut value, &[
    ("SERVER__PORT".into(), "4000".into()),
    ("LOGGER__LEVEL".into(), "debug".into()),
]);
```

## Configuração dos subsistemas

As seções `server`/`logger`/`middleware` são tipadas por `YamlConfig`, mas cada
subsistema plugável lê sua própria seção do mesmo `config/<env>.yml`. Veja o guia
correspondente para as chaves exatas:

```yaml
cache:   { type: memory }                 # → guia de Cache
jobs:    { backend: memory, queues: [default] }  # → guia de Jobs
storage: { service: local }               # → guia de Storage
database: { url: sqlite://db/development.db }     # → guia de Models
```

## Especificação vs. implementação

> A especificação descreve **TOML em camadas** (`config/doido.toml` + overrides por
> ambiente) e **credenciais criptografadas com AES-256-GCM** (`config/credentials.toml.enc`
> + `master.key`). Isso está **adiado**. O caminho implementado e testado é **YAML por
> ambiente** mais overrides `SECTION__KEY`, documentado acima.

## Veja também

- [Core](@/docs/guides/core.pt.md) — a seção `logger` e o `Environment`.
- [Middleware & sessões](@/docs/guides/middleware.pt.md) — a seção `middleware.cors`.
- [Models](@/docs/guides/models.pt.md) — a seção `database` e o pool de conexões.
