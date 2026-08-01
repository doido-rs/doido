+++
title = "Especificaciones de diseño"
description = "Las specs por crate en el repositorio — fuente de verdad para intención de diseño."
weight = 1
+++

Este manual está curado para **uso diario**. Cuando necesites la intención completa de
diseño — decisiones de entrevista, contratos de API, notas de reconciliación — lee las
specs en el repositorio.

## Índice de specs

| Spec | Crate | Tema |
|------|-------|------|
| [`00-overview.md`](https://github.com/doido-rs/doido/blob/master/docs/00-overview.md) | all | Filosofía, mapa de crates, TDD |
| [`01-router.md`](https://github.com/doido-rs/doido/blob/master/docs/01-router.md) | `doido-controller` | DSL de rutas |
| [`02-controller.md`](https://github.com/doido-rs/doido/blob/master/docs/02-controller.md) | `doido-controller` | Controladores, params |
| [`03-model.md`](https://github.com/doido-rs/doido/blob/master/docs/03-model.md) | `doido-model` | sea-orm, pool |
| [`04-view.md`](https://github.com/doido-rs/doido/blob/master/docs/04-view.md) | `doido-view` | Plantillas, layouts |
| [`05-config.md`](https://github.com/doido-rs/doido/blob/master/docs/05-config.md) | `doido-config` | YAML, credenciales |
| [`06-cli.md`](https://github.com/doido-rs/doido/blob/master/docs/06-cli.md) | `doido-generators` | Comandos CLI |
| [`06b-generators.md`](https://github.com/doido-rs/doido/blob/master/docs/06b-generators.md) | `doido-generators` | Generadores |
| [`07-middleware.md`](https://github.com/doido-rs/doido/blob/master/docs/07-middleware.md) | `doido-controller` | Middleware, sesiones |
| [`08-mailer.md`](https://github.com/doido-rs/doido/blob/master/docs/08-mailer.md) | `doido-mailer` | Correo |
| [`09-jobs.md`](https://github.com/doido-rs/doido/blob/master/docs/09-jobs.md) | `doido-jobs` | Jobs, colas |
| [`10-cache.md`](https://github.com/doido-rs/doido/blob/master/docs/10-cache.md) | `doido-cache` | Caché |
| [`11-core.md`](https://github.com/doido-rs/doido/blob/master/docs/11-core.md) | `doido-core` | Errores, inflector |
| [`12-cable.md`](https://github.com/doido-rs/doido/blob/master/docs/12-cable.md) | `doido-cable` | WebSockets |
| [`15-storage.md`](https://github.com/doido-rs/doido/blob/master/docs/15-storage.md) | `doido-storage` | Storage |

## Documento de arquitectura autoritativo

Para lo que está **realmente implementado** — estado de crates, reconciliación de
config y boot — consulta
[`docs/ARCHITECTURE.md`](https://github.com/doido-rs/doido/blob/master/docs/ARCHITECTURE.md).

Cada guía de **[Referencia](/es/docs/reference/)** enlaza su spec al inicio de la página.
