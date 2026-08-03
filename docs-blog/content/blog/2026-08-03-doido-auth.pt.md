+++
title = "Apresentando o doido-auth"
description = "Autenticação unificada para o Doido — sessões no estilo Devise, JWT, OAuth2, 2FA, extractors e geradores."
date = 2026-08-03

[taxonomies]
tags = ["release", "auth", "docs"]
+++

O Doido agora inclui o **`doido-auth`**, a resposta do framework ao Devise, OmniAuth
e tokens JWT bearer — tudo em um crate que se integra com a stack de sessão do
controller.

## O que você ganha

- **Trait `AuthUser`** — ligue autenticação a qualquer model SeaORM; sem struct User
  fixa no framework.
- **Estratégias plugáveis** — sessão por cookie (padrão), JWT bearer e backends
  customizados via `AuthStrategy`.
- **Extractors axum** — `CurrentUser`, `MaybeUser`, `RequireAuth` e `AuthToken`
  funcionam em handlers e actions `#[controller]`.
- **Provedores OAuth2** — configure Google, GitHub ou OIDC genérico em `auth.oauth`
  no YAML; rotas de callback vêm com o `auth:install`.
- **2FA opcional** — enrollment e verificação TOTP atrás da feature `auth-2fa`.
- **Geradores** — `auth:install`, `auth:controller` e `auth:scaffold` aparecem em
  `cargo doido generate` quando `doido-auth` é dependência do projeto.

## Um comando para começar

```bash
doido new myapp --database=sqlite --auth
cd myapp
cargo doido db create && cargo doido db migrate
cargo doido server
```

Isso gera model User, rotas de sign-in/sign-up/senha/OAuth, views HTML e um bloco
`auth:` na config — o mesmo fluxo de `rails generate devise:install`.

Em uma app existente, adicione a dependência e rode o gerador de instalação:

```bash
cargo add doido --features auth
cargo doido generate auth:install
```

## Leia o guia completo

A [referência de Auth](/pt/docs/reference/auth/) cobre configuração, boot, extractors,
estratégias customizadas e helpers de teste. A spec de design está em
[`docs/16-auth.md`](https://github.com/doido-rs/doido/blob/master/docs/16-auth.md).
