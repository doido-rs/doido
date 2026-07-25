//! MIME message assembly. A mail with both text and HTML bodies becomes a
//! `multipart/alternative` message; otherwise a single-part message.

use crate::mail::Mail;
use base64::{engine::general_purpose::STANDARD, Engine as _};

const BOUNDARY: &str = "doido_mime_boundary_9f2a";
const MIXED_BOUNDARY: &str = "doido_mixed_boundary_7c3b";

/// Assemble the full RFC 5322 / MIME message for `mail`. With attachments the
/// message is `multipart/mixed` (body part followed by each base64 attachment).
pub fn to_mime(mail: &Mail) -> String {
    if mail.attachments.is_empty() {
        return body_message(mail);
    }

    let from = mail.from.as_deref().unwrap_or("no-reply@localhost");
    let mut out = format!(
        "From: {from}\r\nTo: {}\r\nSubject: {}\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=\"{MIXED_BOUNDARY}\"\r\n\r\n",
        mail.to, mail.subject
    );

    // Body part: prefer text, else html.
    let (ct, body) = match (&mail.body_text, &mail.body_html) {
        (Some(text), _) => ("text/plain", text.as_str()),
        (None, Some(html)) => ("text/html", html.as_str()),
        (None, None) => ("text/plain", ""),
    };
    out.push_str(&format!(
        "--{MIXED_BOUNDARY}\r\nContent-Type: {ct}; charset=utf-8\r\n\r\n{body}\r\n"
    ));

    for att in &mail.attachments {
        let disposition = if att.inline { "inline" } else { "attachment" };
        let encoded = STANDARD.encode(&att.data);
        out.push_str(&format!(
            "--{MIXED_BOUNDARY}\r\nContent-Type: {}; name=\"{}\"\r\n\
             Content-Transfer-Encoding: base64\r\n\
             Content-Disposition: {disposition}; filename=\"{}\"\r\n\r\n{encoded}\r\n",
            att.content_type, att.filename, att.filename
        ));
    }
    out.push_str(&format!("--{MIXED_BOUNDARY}--\r\n"));
    out
}

/// The body-only message (no attachments): multipart/alternative when both text
/// and HTML are present, otherwise a single part.
fn body_message(mail: &Mail) -> String {
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
