//! An SMTP deliverer (Action Mailer `:smtp`), a small self-contained async SMTP
//! client (`EHLO`/`MAIL FROM`/`RCPT TO`/`DATA`/`QUIT`) over plain TCP.

use crate::deliverer::Deliverer;
use crate::mail::Mail;
use doido_core::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Delivers mail by talking SMTP to a server at `host:port`.
pub struct SmtpDeliverer {
    addr: String,
    ehlo_name: String,
}

impl SmtpDeliverer {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            ehlo_name: "doido".to_string(),
        }
    }
}

/// Build the RFC 5322 / MIME message for `mail` (delegates to [`crate::mime`]).
pub fn build_message(mail: &Mail) -> String {
    crate::mime::to_mime(mail)
}

#[async_trait::async_trait]
impl Deliverer for SmtpDeliverer {
    async fn deliver(&self, mail: &Mail) -> Result<()> {
        let stream = TcpStream::connect(&self.addr).await.map_err(|e| {
            doido_core::anyhow::anyhow!("smtp connect to {} failed: {e}", self.addr)
        })?;
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        expect(&mut reader, "220").await?;
        send(&mut write_half, &format!("EHLO {}", self.ehlo_name)).await?;
        expect(&mut reader, "250").await?;

        let from = mail.from.as_deref().unwrap_or("no-reply@localhost");
        send(&mut write_half, &format!("MAIL FROM:<{from}>")).await?;
        expect(&mut reader, "250").await?;
        send(&mut write_half, &format!("RCPT TO:<{}>", mail.to)).await?;
        expect(&mut reader, "250").await?;

        send(&mut write_half, "DATA").await?;
        expect(&mut reader, "354").await?;
        write_half.write_all(build_message(mail).as_bytes()).await?;
        write_half.write_all(b"\r\n.\r\n").await?;
        expect(&mut reader, "250").await?;

        send(&mut write_half, "QUIT").await?;
        Ok(())
    }
}

async fn send<W: AsyncWriteExt + Unpin>(w: &mut W, cmd: &str) -> Result<()> {
    w.write_all(format!("{cmd}\r\n").as_bytes()).await?;
    Ok(())
}

/// Read a (possibly multi-line) SMTP reply and require the given status code.
async fn expect<R: AsyncBufReadExt + Unpin>(reader: &mut R, code: &str) -> Result<()> {
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            return Err(doido_core::anyhow::anyhow!("smtp connection closed early"));
        }
        let line = line.trim_end();
        if line.len() < 3 || &line[..3] != code {
            return Err(doido_core::anyhow::anyhow!(
                "smtp expected {code}, got: {line}"
            ));
        }
        // "250 " ends the reply; "250-" is a continuation line.
        if line.len() == 3 || line.as_bytes()[3] == b' ' {
            return Ok(());
        }
    }
}
