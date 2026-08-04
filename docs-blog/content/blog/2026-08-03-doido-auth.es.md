+++
title = "Presentamos doido-auth"
description = "Autenticación unificada para Doido — sesiones al estilo Devise, JWT, OAuth2, 2FA, extractors y generadores."
date = 2026-08-03

[taxonomies]
tags = ["release", "auth", "docs"]
+++

Doido ahora incluye **`doido-auth`**, la respuesta del framework a Devise, OmniAuth
y tokens JWT bearer — todo en un crate que se integra con la pila de sesiones del
controlador.

## Qué incluye

- **Trait `AuthUser`** — vincula la autenticación a cualquier modelo SeaORM; sin struct
  User fija en el framework.
- **Estrategias enchufables** — sesión por cookie (predeterminado), JWT bearer y backends
  personalizados vía `AuthStrategy`.
- **Extractors axum** — `CurrentUser`, `MaybeUser`, `RequireAuth` y `AuthToken`
  funcionan en handlers y acciones `#[controller]`.
- **Proveedores OAuth** — configura cualquier proveedor de identidad en `auth.oauth`
  mediante la trait `OAuthProvider`; las entradas OAuth 2.0 se cargan del YAML en el boot.
- **2FA opcional** — enrollment y verificación TOTP detrás de la feature `auth-2fa`.
- **Generadores** — `auth:install`, `auth:controller` y `auth:scaffold` aparecen en
  `cargo doido generate` cuando `doido-auth` es dependencia del proyecto.

## Un comando para empezar

```bash
doido new myapp --database=sqlite --auth
cd myapp
cargo doido db create && cargo doido db migrate
cargo doido server
```

Eso genera modelo User, rutas de sign-in/sign-up/contraseña/OAuth, vistas HTML y un
bloque `auth:` en la config — el mismo flujo que `rails generate devise:install`.

En una app existente, añade la dependencia y ejecuta el generador de instalación:

```bash
cargo add doido --features auth
cargo doido generate auth:install
```

## Lee la guía completa

La [referencia de Auth](/es/docs/reference/auth/) cubre configuración, boot, extractors,
estrategias personalizadas y helpers de prueba. La spec de diseño está en
[`docs/16-auth.md`](https://github.com/doido-rs/doido/blob/master/docs/16-auth.md).
