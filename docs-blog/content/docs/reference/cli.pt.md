+++
title = "CLI & geradores"
description = "Todos os comandos de runtime e geradores de código do binário doido."
weight = 3
+++

O CLI global `doido` cria novas aplicações. Dentro de um projeto gerado,
`cargo doido` (um alias do Cargo configurado pelo `doido new`) é o ponto de
entrada para comandos de runtime e geração de código.

## Comandos de runtime

| Comando | Descrição |
|---------|-----------|
| `doido new <name>` | Cria uma nova aplicação (`--database=sqlite\|postgres\|mysql`) |
| `cargo doido server` | Inicia o servidor web |
| `cargo doido routes` | Imprime a tabela de rotas |
| `cargo doido console` | Inicia um console interativo |
| `cargo doido db <cmd>` | Cria bancos, roda migrations do SeaORM, gera entidades |
| `cargo doido jobs <cmd>` | Inspeciona e gerencia jobs em background |
| `cargo doido worker` | Roda o worker de jobs (`--once` para drenar e sair) |
| `cargo doido credentials <cmd>` | Gerencia credenciais criptografadas com AES-256-GCM |
| `cargo doido generate <gen>` | Roda um gerador de código (veja abaixo) |

## Geradores

Rode `cargo doido generate` sem argumentos para listar todos os geradores registrados:

| Gerador | Gera |
|---------|------|
| `controller` | Um controller com actions |
| `helper` | Um helper de controller em `app/helpers/` |
| `model` | Um model + migration do SeaORM |
| `migration` | Uma migration isolada |
| `scaffold` | Model, controller, views, rotas — o CRUD completo |
| `job` | Um job em background |
| `mailer` | Um mailer com templates |
| `channel` | Um canal WebSocket |
| `templates` | Templates de view para um controller existente |
| `generator` | Um novo gerador customizado (o registro é extensível) |

Os geradores injetam rotas automaticamente em `config/routes.rs` e respeitam as
regras de pluralização declaradas em `config/inflection.yaml`.

## Crates do workspace

| Crate | Análogo no Rails | Responsabilidade |
|-------|------------------|------------------|
| `doido` | binário `rails` | Ponto de entrada, runtime da app |
| `doido-core` | Active Support | Traits, erros, inflector, logger, utilitários |
| `doido-controller` | Action Dispatch + Controller + Rack | DSL de rotas, requisições, params, helpers de controller, middleware Tower, sessões |
| `doido-model` | Active Record | Re-exports do sea-orm, pool de conexões, helpers de teste |
| `doido-view` | Action View | Templates Tera, layouts, partials |
| `doido-config` | `config/` do Rails | Config TOML/YAML em camadas, credenciais, overrides por env |
| `doido-generators` | CLI + geradores do `rails` | Comandos de runtime e geradores de código |
| `doido-mailer` | Action Mailer | Composição e envio de e-mails |
| `doido-jobs` | Active Job | Jobs em background com backends plugáveis e retries |
| `doido-cache` | Active Support Cache | Cache plugável (memory / redis / memcache) |
| `doido-cable` | Action Cable | Canais WebSocket e pub/sub |
| `doido-storage` | Active Storage | Armazenamento de arquivos (disk / S3 / R2 / Azure) |

Para a intenção completa de design de cada crate, veja as
[especificações](https://github.com/doido-rs/doido/tree/master/docs).
