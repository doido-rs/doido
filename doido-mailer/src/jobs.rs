//! The built-in background delivery job behind [`Mail::deliver_later`].
//!
//! Defining it as a `#[job]` means it self-registers with the worker's job
//! registry (link-time, via `inventory`), so an enqueued mail is actually
//! delivered by the worker rather than silently acked. The handler uses the
//! process-global deliverer configured from `[mailer]` (see [`crate::global`]).
//!
//! [`Mail::deliver_later`]: crate::Mail::deliver_later

use crate::mail::Mail;

/// Deliver a mail enqueued by `Mail::deliver_later` (queue `mailers`).
#[doido_jobs::job(queue = "mailers", max_retries = 3)]
async fn deliver_mail(mail: Mail) -> doido_core::Result<()> {
    crate::global::deliverer().deliver(&mail).await
}
