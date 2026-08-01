+++
title = "Mailer"
description = "Compose email with the #[mailer] macro and Mail builder; deliver now or later through pluggable deliverers."
weight = 9
aliases = ['/docs/guides/mailer/']

+++

> **Design spec:** [`docs/08-mailer.md`](https://github.com/doido-rs/doido/blob/master/docs/08-mailer.md).
> This guide documents the API as implemented in `doido-mailer`.

**Rails analogue: Action Mailer.** A mailer builds a `Mail` and delivers it either
synchronously (`deliver_now`) or in the background (`deliver_later`, which enqueues onto the
`mailers` queue). Transport is pluggable behind the `Deliverer` trait, and templates render
through the same [Tera engine](@/docs/reference/views.md) as views.

## At a glance

```rust
use doido::mailer::{mailer, Mail, Deliverer, SmtpDeliverer, LogDeliverer, TestDeliverer};
```

## Defining a mailer

`#[mailer]` derives a snake_case name used for template resolution
(`UserMailer` → `mailers/user_mailer/<action>`). Actions build and return a `Mail`.

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

## The `Mail` builder

`Mail` is a fluent builder: `from`, `to`, `subject`, `body_html`, `body_text`, plus
attachments. Provide both an HTML and a text body for a proper multipart message.

```rust
let mail = Mail::new()
    .from("noreply@example.com")
    .to("alice@example.com")
    .subject("Your report")
    .body_html("<h1>Report</h1>")
    .body_text("Report");
```

## Deliver now or later

`deliver_now` sends immediately through a `Deliverer`; `deliver_later` serializes the mail
onto the `mailers` queue for a [worker](@/docs/reference/jobs.md) to send.

```rust
// Inline (blocks until sent):
UserMailer::welcome(&user).deliver_now(&deliverer).await?;

// Background (enqueues onto the "mailers" queue, returns a JobId):
UserMailer::welcome(&user).deliver_later(job_queue.as_ref()).await?;
```

## Deliverers

`Deliverer` is the pluggable transport trait. Built-ins: `SmtpDeliverer` (real SMTP),
`SendmailDeliverer` (local `sendmail`), `LogDeliverer` (logs the mail), and `TestDeliverer`
(captures mail in memory). Select per environment via config.

```rust
use std::sync::Arc;
use doido::mailer::{Deliverer, SmtpDeliverer, LogDeliverer};

let deliverer: Arc<dyn Deliverer> = if production {
    Arc::new(SmtpDeliverer::new("smtp://localhost:25"))
} else {
    Arc::new(LogDeliverer) // just log the message in development
};
```

## Attachments

Attach files, or embed inline content (e.g. images referenced by `cid`).

```rust
let mail = Mail::new()
    .to("alice@example.com")
    .subject("Invoice")
    .attach("invoice.pdf", "application/pdf", pdf_bytes)
    .attach_inline("logo.png", "image/png", logo_bytes);
```

## Templates

Templates live under `app/views/mailers/<mailer_name>/<action>` and render through
`doido-view`, so you get layouts, partials, and helpers. Provide `.html` and `.text`
variants for multipart mail.

## Interceptors & previews

Wrap any deliverer with `InterceptingDeliverer` to rewrite or observe every message (e.g.
redirect all mail to a catch-all in staging), and register `MailerPreviews` to render mail
in the browser without sending.

```rust
use doido::mailer::interceptors::InterceptingDeliverer;

let deliverer = InterceptingDeliverer::new(LogDeliverer)
    .intercept(|mail| { /* rewrite recipients, etc. */ })
    .observe(|mail| { /* metrics / logging */ });
```

## Testing

`TestDeliverer` captures delivered mail so tests can assert on recipients, subjects, and
bodies.

```rust
use doido::mailer::TestDeliverer;

let deliverer = TestDeliverer::new();
UserMailer::welcome(&user).deliver_now(&deliverer).await?;

let sent = deliverer.sent().await;
assert_eq!(sent.len(), 1);
assert_eq!(sent[0].subject, "Welcome!");
```

## Spec vs. implementation

> `#[mailer]` is intentionally minimal — it generates the name and template key; you build
> the `Mail` in the action. `deliver_later` enqueues a serialized `Mail` onto the `mailers`
> queue (there is no separate delivery-job struct to define).

## See also

- [Jobs](@/docs/reference/jobs.md) — the `mailers` queue and worker behind `deliver_later`.
- [Views](@/docs/reference/views.md) — the engine that renders email templates.
- [Generators & CLI](@/docs/reference/generators.md) — `doido generate mailer`.
