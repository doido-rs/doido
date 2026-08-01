+++
title = "Core"
description = "Inflexões, o modelo de erros, logging, ambiente e utilitários compartilhados sobre os quais todos os outros crates são construídos."
weight = 1
+++

> **Especificação de design:** [`docs/11-core.md`](https://github.com/doido-rs/doido/blob/master/docs/11-core.md).
> Este guia documenta a API como implementada em `doido-core`.

**Análogo no Rails: Active Support.** `doido-core` é a fundação da qual todos os outros
crates do Doido dependem — e ele não depende de nenhum deles. Fornece o inflector de
strings, o modelo de erros da aplicação, a configuração de logging, a detecção de
ambiente e um punhado de extensões ergonômicas (helpers de string/tempo, notificações,
um relógio de teste). Também reexporta os crates do ecossistema que o framework inteiro
compartilha, para que os crates downstream dependam apenas de `doido-core`.

## Visão geral

```rust
use doido::Result;
use doido::core::{Inflector, Environment, init_logger, load_inflections};

// Reexportado para crates downstream (dependa de doido-core, não destes diretamente):
use doido::core::{anyhow, async_trait, serde, thiserror, tracing};
```

## Inflexão de strings

`Inflector` é uma fachada estática que oferece todas as transformações no estilo Rails.
Ela alimenta os geradores (nomes de tabela, nomes de classe, helpers de rota) e está
disponível em qualquer lugar da sua app.

```rust
use doido::core::Inflector;

Inflector::pluralize("post");        // "posts"
Inflector::singularize("comments");  // "comment"
Inflector::camelize("blog_post");    // "BlogPost"
Inflector::camelize_lower("blog_post"); // "blogPost"
Inflector::underscore("BlogPost");   // "blog_post"
Inflector::dasherize("blog_post");   // "blog-post"
Inflector::humanize("author_id");    // "Author"
Inflector::tableize("BlogPost");     // "blog_posts"
Inflector::classify("blog_posts");   // "BlogPost"
Inflector::foreign_key("Author");    // "author_id"
Inflector::constantize("blog_post"); // "BlogPost"
```

## Regras de inflexão customizadas

Os padrões cobrem o inglês comum. Sobrescreva ou estenda-os para plurais irregulares,
substantivos incontáveis e siglas — via arquivo YAML ou programaticamente. É o mesmo
mecanismo que os geradores respeitam, então `config/inflection.yaml` mantém os nomes
gerados consistentes com a linguagem do seu domínio.

Carregue as regras de `config/inflection.yaml` no boot (retorna `Ok(false)` quando o
arquivo está ausente, então é seguro chamar incondicionalmente):

```rust
// Carrega config/inflection.yaml se existir; Ok(true) quando aplicado, Ok(false) quando ausente.
doido::core::load_inflections("config/inflection.yaml")?;
```

```yaml
# config/inflection.yaml
irregular:
  - [person, people]
  - [mouse, mice]
uncountable:
  - equipment
  - information
acronym:
  - API
  - HTTP
```

Ou configure as regras em código com `init_inflections`, usando o builder `Inflections`
(`irregular`, `uncountable`, `acronym`, `plural`, `singular`):

```rust
doido::core::init_inflections(|i| {
    i.irregular("person", "people");
    i.uncountable("equipment");
    i.acronym("API");
});
```

## O modelo de erros

O código de aplicação usa um único alias `Result<T>` construído sobre `anyhow`; cada
crate define seus próprios erros tipados com `thiserror`. `anyhow`, `bail` e o trait
`Context` (`AnyhowContext`) são reexportados, então `?` e o contexto de erro funcionam
sem configuração.

```rust
use doido::Result;
use doido::core::anyhow::Context;

fn load_settings() -> Result<Settings> {
    let raw = std::fs::read_to_string("config/settings.toml")
        .context("settings file is missing")?;      // adiciona contexto a qualquer erro
    let settings = toml::from_str(&raw)?;            // ? converte em anyhow::Error
    Ok(settings)
}
```

Para um crate de biblioteca, defina um erro tipado com o `thiserror` reexportado:

```rust
use doido::core::thiserror;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("key not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

## Logging

Uma única chamada instala o subscriber global do `tracing`. Níveis, formato de saída,
diretivas de filtro e redirecionamento para arquivo vêm da seção `logger` do
`config/<env>.yml` via `LoggerConfig`; `init_logger()` usa `RUST_LOG` (ou padrões
sensatos) e é idempotente.

```rust
// Forma mais simples — respeita RUST_LOG, com fallback para padrões sensatos.
doido::core::init_logger();

// Ou controle via config (config/<env>.yml → [logger]):
use doido::core::LoggerConfig;
let cfg = LoggerConfig { level: "debug".into(), ..Default::default() };
doido::core::logger::init_with_config(&cfg);
```

```yaml
# config/development.yml
logger:
  level: debug          # trace | debug | info | warn | error
  format: verbose       # compact (padrão) | verbose | json_response
  sql: true             # loga cada statement SQL
  # file: log/development.log   # redireciona a saída (ANSI desativado)
  # directives: "info,my_app=debug,sqlx=warn"  # override completo do EnvFilter
```

`LogFormat` seleciona o renderizador: `Compact` (uma linha, padrão), `Verbose`
(multilinha com todos os campos e a localização no código) e `JsonResponse` (um objeto
JSON por resposta HTTP — logs de acesso e métricas de latência).

## Ambiente

`Environment` distingue `Development`, `Test` e `Production`, resolvido a partir do
ambiente do processo.

```rust
use doido::core::Environment;

match Environment::get_env() {
    Environment::Development => { /* erros verbosos, hot reload */ }
    Environment::Test => { /* backends em memória */ }
    Environment::Production => { /* estrito, cacheado */ }
}

let name = Environment::get_env().as_str(); // "development" | "test" | "production"
```

## Helpers de string e tempo

`core_ext` adiciona traits de extensão ergonômicos (`Blank` para checagens de vazio,
`StringExt` para transformações comuns), e `time_ext` oferece helpers de tempo relativo
no estilo Rails.

```rust
use doido::core::core_ext::Blank;
use doido::core::time_ext::{days_ago, beginning_of_day};

"".is_blank();                 // true
"  ".is_blank();               // true (só espaços)
let since = days_ago(7);       // DateTime<Utc> de sete dias atrás
let start = beginning_of_day(chrono::Utc::now());
```

## Instrumentação e notificações

`trace` fornece helpers finos e consistentes de eventos estruturados usados por todo o
framework (requisições, jobs, queries, e-mail), e `notifications` oferece um pub/sub leve
para instrumentação em processo.

```rust
use doido::core::notifications::Notifications;

let notifications = Notifications::new();
notifications.subscribe("post.created", |payload| {
    tracing::info!(%payload, "a post was created");
});
notifications.instrument("post.created", "{\"id\":1}");
```

## Relógio de teste

`TestClock` congela e avança o tempo em testes, tornando determinística a lógica
dependente de tempo.

```rust
use doido::core::test_time::TestClock;

let clock = TestClock::new(chrono::Utc::now());
clock.travel(chrono::Duration::hours(2)); // avança no tempo
let later = clock.now();
```

## Veja também

- [Configuração](@/docs/reference/configuration.pt.md) — de onde vêm `logger` e outras configurações.
- [Models](@/docs/reference/models.pt.md) — o logging de `sql` flui para os logs de query do sea-orm.
- [Geradores & CLI](@/docs/reference/generators.pt.md) — consumidores do inflector.
