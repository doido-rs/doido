+++
title = "Mailer"
description = "Componha e-mails com a macro #[mailer] e o builder Mail; entregue agora ou depois por meio de deliverers plugáveis."
weight = 9
+++

> **Especificação de design:** [`docs/08-mailer.md`](https://github.com/doido-rs/doido/blob/master/docs/08-mailer.md).
> Este guia documenta a API como implementada em `doido-mailer`.

**Análogo no Rails: Action Mailer.** Um mailer constrói um `Mail` e o entrega de forma
síncrona (`deliver_now`) ou em background (`deliver_later`, que enfileira na fila
`mailers`). O transporte é plugável por trás do trait `Deliverer`, e os templates renderizam
pela mesma [engine Tera](@/docs/reference/views.pt.md) das views.

## Visão geral

```rust
use doido_mailer::{mailer, Mail, Deliverer, SmtpDeliverer, LogDeliverer, TestDeliverer};
```

## Definindo um mailer

`#[mailer]` deriva um nome snake_case usado para resolução de template
(`UserMailer` → `mailers/user_mailer/<action>`). As actions constroem e retornam um `Mail`.

```rust
use doido_mailer::{mailer, Mail};

#[mailer]
struct UserMailer;

impl UserMailer {
    fn welcome(user: &User) -> Mail {
        let html = doido_view::render(
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

## O builder `Mail`

`Mail` é um builder fluente: `from`, `to`, `subject`, `body_html`, `body_text`, além de
anexos. Forneça um corpo HTML e um de texto para uma mensagem multipart adequada.

```rust
let mail = Mail::new()
    .from("noreply@example.com")
    .to("alice@example.com")
    .subject("Your report")
    .body_html("<h1>Report</h1>")
    .body_text("Report");
```

## Entregar agora ou depois

`deliver_now` envia imediatamente por um `Deliverer`; `deliver_later` serializa o e-mail na
fila `mailers` para um [worker](@/docs/reference/jobs.pt.md) enviar.

```rust
// Inline (bloqueia até enviar):
UserMailer::welcome(&user).deliver_now(&deliverer).await?;

// Background (enfileira na fila "mailers", retorna um JobId):
UserMailer::welcome(&user).deliver_later(job_queue.as_ref()).await?;
```

## Deliverers

`Deliverer` é o trait plugável de transporte. Embutidos: `SmtpDeliverer` (SMTP real),
`SendmailDeliverer` (`sendmail` local), `LogDeliverer` (loga o e-mail) e `TestDeliverer`
(captura o e-mail em memória). Selecione por ambiente via config.

```rust
use std::sync::Arc;
use doido_mailer::{Deliverer, SmtpDeliverer, LogDeliverer};

let deliverer: Arc<dyn Deliverer> = if production {
    Arc::new(SmtpDeliverer::new("smtp://localhost:25"))
} else {
    Arc::new(LogDeliverer) // apenas loga a mensagem em desenvolvimento
};
```

## Anexos

Anexe arquivos, ou embuta conteúdo inline (ex.: imagens referenciadas por `cid`).

```rust
let mail = Mail::new()
    .to("alice@example.com")
    .subject("Invoice")
    .attach("invoice.pdf", "application/pdf", pdf_bytes)
    .attach_inline("logo.png", "image/png", logo_bytes);
```

## Templates

Os templates ficam em `app/views/mailers/<mailer_name>/<action>` e renderizam pelo
`doido-view`, então você tem layouts, partials e helpers. Forneça variantes `.html` e
`.text` para e-mail multipart.

## Interceptors & previews

Envolva qualquer deliverer com `InterceptingDeliverer` para reescrever ou observar cada
mensagem (ex.: redirecionar todo e-mail para um catch-all em staging), e registre
`MailerPreviews` para renderizar o e-mail no navegador sem enviar.

```rust
use doido_mailer::interceptors::InterceptingDeliverer;

let deliverer = InterceptingDeliverer::new(LogDeliverer)
    .intercept(|mail| { /* reescreve destinatários, etc. */ })
    .observe(|mail| { /* métricas / logging */ });
```

## Testes

`TestDeliverer` captura o e-mail entregue para que os testes possam verificar
destinatários, assuntos e corpos.

```rust
use doido_mailer::TestDeliverer;

let deliverer = TestDeliverer::new();
UserMailer::welcome(&user).deliver_now(&deliverer).await?;

let sent = deliverer.sent().await;
assert_eq!(sent.len(), 1);
assert_eq!(sent[0].subject, "Welcome!");
```

## Especificação vs. implementação

> `#[mailer]` é propositalmente mínimo — gera o nome e a chave de template; você constrói o
> `Mail` na action. `deliver_later` enfileira um `Mail` serializado na fila `mailers` (não
> há um struct de job de entrega separado para definir).

## Veja também

- [Jobs](@/docs/reference/jobs.pt.md) — a fila `mailers` e o worker por trás do `deliver_later`.
- [Views](@/docs/reference/views.pt.md) — a engine que renderiza os templates de e-mail.
- [Geradores & CLI](@/docs/reference/generators.pt.md) — `doido generate mailer`.
