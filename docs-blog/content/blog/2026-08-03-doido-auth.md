+++
title = "Introducing doido-auth"
description = "Unified authentication for Doido — Devise-style sessions, JWT, OAuth2, 2FA, extractors, and generators."
date = 2026-08-03

[taxonomies]
tags = ["release", "auth", "docs"]
+++

Doido now ships **`doido-auth`**, the framework's answer to Devise, OmniAuth, and
JWT bearer tokens — all in one crate that composes with the existing controller
session stack.

## What you get

- **`AuthUser` trait** — bind authentication to any SeaORM model; no hard-coded User
  struct in the framework.
- **Pluggable strategies** — cookie session (default), JWT bearer, and custom
  backends via `AuthStrategy`.
- **Axum extractors** — `CurrentUser`, `MaybeUser`, `RequireAuth`, and `AuthToken`
  work in handlers and `#[controller]` actions.
- **OAuth providers** — configure any identity provider under `auth.oauth` via the
  `OAuthProvider` trait; OAuth 2.0 entries load from YAML at boot.
- **Optional 2FA** — TOTP enrollment and verification behind the `auth-2fa` feature.
- **Generators** — `auth:install`, `auth:controller`, and `auth:scaffold` appear in
  `cargo doido generate` when `doido-auth` is a project dependency.

## One command to start

```bash
doido new myapp --database=sqlite --auth
cd myapp
cargo doido db create && cargo doido db migrate
cargo doido server
```

That scaffolds a User model, sign-in/sign-up/password/OAuth routes, HTML views, and
an `auth:` config block — the same workflow as `rails generate devise:install`.

For an existing app, add the dependency and run the install generator:

```bash
cargo add doido --features auth
cargo doido generate auth:install
```

## Read the full guide

The [Auth reference](/docs/reference/auth/) covers configuration, boot sequence,
extractors, custom strategies, and testing helpers. The design spec lives in
[`docs/16-auth.md`](https://github.com/doido-rs/doido/blob/master/docs/16-auth.md).
