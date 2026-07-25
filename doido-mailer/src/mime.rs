//! MIME message assembly. A mail with both text and HTML bodies becomes a
//! `multipart/alternative` message; otherwise a single-part message.

use crate::mail::Mail;

const BOUNDARY: &str = "doido_mime_boundary_9f2a";

/// Assemble the full RFC 5322 / MIME message for `mail`.
pub fn to_mime(mail: &Mail) -> String {
    let from = mail.from.as_deref().unwrap_or("no-reply@localhost");
    let headers = format!(
        "From: {from}\r\nTo: {}\r\nSubject: {}\r\nMIME-Version: 1.0\r\n",
        mail.to, mail.subject
    );

    match (&mail.body_text, &mail.body_html) {
        (Some(text), Some(html)) => format!(
            "{headers}Content-Type: multipart/alternative; boundary=\"{BOUNDARY}\"\r\n\r\n\
             --{BOUNDARY}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{text}\r\n\
             --{BOUNDARY}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{html}\r\n\
             --{BOUNDARY}--\r\n"
        ),
        (Some(text), None) => {
            format!("{headers}Content-Type: text/plain; charset=utf-8\r\n\r\n{text}")
        }
        (None, Some(html)) => {
            format!("{headers}Content-Type: text/html; charset=utf-8\r\n\r\n{html}")
        }
        (None, None) => format!("{headers}Content-Type: text/plain; charset=utf-8\r\n\r\n"),
    }
}
