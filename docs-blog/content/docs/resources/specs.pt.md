+++
title = "Especificações de design"
description = "As specs por crate no repositório — fonte da verdade para intenção de design."
weight = 1
+++

Este manual é curado para **uso no dia a dia**. Quando precisar da intenção completa
de design — decisões de entrevista, contratos de API, notas de reconciliação — leia
as specs no repositório.

## Índice de specs

| Spec | Crate | Tópico |
|------|-------|--------|
| [`00-overview.md`](https://github.com/doido-rs/doido/blob/master/docs/00-overview.md) | all | Filosofia, mapa de crates, TDD |
| [`01-router.md`](https://github.com/doido-rs/doido/blob/master/docs/01-router.md) | `doido-controller` | DSL de rotas |
| [`02-controller.md`](https://github.com/doido-rs/doido/blob/master/docs/02-controller.md) | `doido-controller` | Controllers, params, filters |
| [`03-model.md`](https://github.com/doido-rs/doido/blob/master/docs/03-model.md) | `doido-model` | sea-orm, pool |
| [`04-view.md`](https://github.com/doido-rs/doido/blob/master/docs/04-view.md) | `doido-view` | Templates, layouts |
| [`05-config.md`](https://github.com/doido-rs/doido/blob/master/docs/05-config.md) | `doido-config` | YAML, credenciais |
| [`06-cli.md`](https://github.com/doido-rs/doido/blob/master/docs/06-cli.md) | `doido-generators` | Comandos CLI |
| [`06b-generators.md`](https://github.com/doido-rs/doido/blob/master/docs/06b-generators.md) | `doido-generators` | Geradores |
| [`07-middleware.md`](https://github.com/doido-rs/doido/blob/master/docs/07-middleware.md) | `doido-controller` | Middleware, sessões |
| [`08-mailer.md`](https://github.com/doido-rs/doido/blob/master/docs/08-mailer.md) | `doido-mailer` | E-mail |
| [`09-jobs.md`](https://github.com/doido-rs/doido/blob/master/docs/09-jobs.md) | `doido-jobs` | Jobs, filas |
| [`10-cache.md`](https://github.com/doido-rs/doido/blob/master/docs/10-cache.md) | `doido-cache` | Cache |
| [`11-core.md`](https://github.com/doido-rs/doido/blob/master/docs/11-core.md) | `doido-core` | Erros, inflector |
| [`12-cable.md`](https://github.com/doido-rs/doido/blob/master/docs/12-cable.md) | `doido-cable` | WebSockets |
| [`15-storage.md`](https://github.com/doido-rs/doido/blob/master/docs/15-storage.md) | `doido-storage` | Storage |

## Documento de arquitetura autoritativo

Para o que está **realmente implementado** — status dos crates, reconciliação de
config e boot — veja
[`docs/ARCHITECTURE.md`](https://github.com/doido-rs/doido/blob/master/docs/ARCHITECTURE.md).

Cada guia da **[Referência](/pt/docs/reference/)** linka sua spec no topo da página.
