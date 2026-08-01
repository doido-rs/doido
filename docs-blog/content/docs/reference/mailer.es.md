+++
title = "Mailer"
description = "Compón correos con la macro #[mailer] y el builder Mail; entrega ahora o después mediante deliverers conectables."
weight = 9
+++

> **Especificación de diseño:** [`docs/08-mailer.md`](https://github.com/doido-rs/doido/blob/master/docs/08-mailer.md).
> Esta guía documenta la API tal como está implementada en `doido-mailer`.

**Análogo en Rails: Action Mailer.** Un mailer construye un `Mail` y lo entrega de forma
síncrona (`deliver_now`) o en segundo plano (`deliver_later`, que encola en la cola
`mailers`). El transporte es conectable detrás del trait `Deliverer`, y las plantillas
renderizan mediante el mismo [engine Tera](@/docs/reference/views.es.md) que las vistas.

## Vistazo general

```rust
use doido::mailer::{mailer, Mail, Deliverer, SmtpDeliverer, LogDeliverer, TestDeliverer};
```

## Definir un mailer

`#[mailer]` deriva un nombre snake_case usado para la resolución de plantillas
(`UserMailer` → `mailers/user_mailer/<action>`). Las actions construyen y devuelven un
`Mail`.

```rust
use doido::mailer::{mailer, Mail};

#[mailer]
struct UserMailer;

impl UserMailer {
    fn welcome(user: &User) -> Mail {
        let html = doido::view::render(
            "mailers/user_mailer/welcome",
            &serde_json::json!({ "name": user.name }),
        ).unwrap_or_default();

        Mail::new()
            .from("noreply@example.com")
            .to(&user.email)
            .subject("Welcome!")
            .body_html(html)
    }
}
```

## El builder `Mail`

`Mail` es un builder fluido: `from`, `to`, `subject`, `body_html`, `body_text`, además de
adjuntos. Provee un cuerpo HTML y uno de texto para un mensaje multipart adecuado.

```rust
let mail = Mail::new()
    .from("noreply@example.com")
    .to("alice@example.com")
    .subject("Your report")
    .body_html("<h1>Report</h1>")
    .body_text("Report");
```

## Entregar ahora o después

`deliver_now` envía de inmediato mediante un `Deliverer`; `deliver_later` serializa el
correo en la cola `mailers` para que un [worker](@/docs/reference/jobs.es.md) lo envíe.

```rust
// Inline (bloquea hasta enviar):
UserMailer::welcome(&user).deliver_now(&deliverer).await?;

// En segundo plano (encola en la cola "mailers", devuelve un JobId):
UserMailer::welcome(&user).deliver_later(job_queue.as_ref()).await?;
```

## Deliverers

`Deliverer` es el trait conectable de transporte. Integrados: `SmtpDeliverer` (SMTP real),
`SendmailDeliverer` (`sendmail` local), `LogDeliverer` (registra el correo) y
`TestDeliverer` (captura el correo en memoria). Selecciónalo por entorno vía config.

```rust
use std::sync::Arc;
use doido::mailer::{Deliverer, SmtpDeliverer, LogDeliverer};

let deliverer: Arc<dyn Deliverer> = if production {
    Arc::new(SmtpDeliverer::new("smtp://localhost:25"))
} else {
    Arc::new(LogDeliverer) // solo registra el mensaje en desarrollo
};
```

## Adjuntos

Adjunta archivos, o incrusta contenido inline (p. ej. imágenes referenciadas por `cid`).

```rust
let mail = Mail::new()
    .to("alice@example.com")
    .subject("Invoice")
    .attach("invoice.pdf", "application/pdf", pdf_bytes)
    .attach_inline("logo.png", "image/png", logo_bytes);
```

## Plantillas

Las plantillas viven en `app/views/mailers/<mailer_name>/<action>` y renderizan mediante
`doido-view`, así que obtienes layouts, partials y helpers. Provee variantes `.html` y
`.text` para correo multipart.

## Interceptors y previews

Envuelve cualquier deliverer con `InterceptingDeliverer` para reescribir u observar cada
mensaje (p. ej. redirigir todo el correo a un catch-all en staging), y registra
`MailerPreviews` para renderizar el correo en el navegador sin enviarlo.

```rust
use doido::mailer::interceptors::InterceptingDeliverer;

let deliverer = InterceptingDeliverer::new(LogDeliverer)
    .intercept(|mail| { /* reescribe destinatarios, etc. */ })
    .observe(|mail| { /* métricas / logging */ });
```

## Pruebas

`TestDeliverer` captura el correo entregado para que las pruebas puedan verificar
destinatarios, asuntos y cuerpos.

```rust
use doido::mailer::TestDeliverer;

let deliverer = TestDeliverer::new();
UserMailer::welcome(&user).deliver_now(&deliverer).await?;

let sent = deliverer.sent().await;
assert_eq!(sent.len(), 1);
assert_eq!(sent[0].subject, "Welcome!");
```

## Especificación vs. implementación

> `#[mailer]` es deliberadamente mínimo — genera el nombre y la clave de plantilla; tú
> construyes el `Mail` en la action. `deliver_later` encola un `Mail` serializado en la cola
> `mailers` (no hay una struct de job de entrega separada que definir).

## Véase también

- [Jobs](@/docs/reference/jobs.es.md) — la cola `mailers` y el worker detrás de `deliver_later`.
- [Vistas](@/docs/reference/views.es.md) — el engine que renderiza las plantillas de correo.
- [Generadores y CLI](@/docs/reference/generators.es.md) — `cargo doido generate mailer`.
