//! An SMTP deliverer (Action Mailer `:smtp`), a small self-contained async SMTP
//! client (`EHLO`/`MAIL FROM`/`RCPT TO`/`DATA`/`QUIT`). Plain TCP by default,
//! with opt-in `STARTTLS` (`SmtpDeliverer::new(addr).starttls()`) that upgrades
//! the connection to TLS via rustls when the server advertises the capability.

use crate::deliverer::Deliverer;
use crate::mail::Mail;
use doido_core::Result;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};
use tokio::net::TcpStream;

/// Delivers mail by talking SMTP to a server at `host:port`.
pub struct SmtpDeliverer {
    addr: String,
    ehlo_name: String,
    starttls: bool,
}

impl SmtpDeliverer {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            ehlo_name: "doido".to_string(),
            starttls: false,
        }
    }

    /// Upgrade to TLS via `STARTTLS` before authenticating/sending, when the
    /// server advertises it in its EHLO capabilities.
    pub fn starttls(mut self) -> Self {
        self.starttls = true;
        self
    }

    /// The host portion of `addr` (before `:port`), used as the TLS server name.
    fn host(&self) -> &str {
        self.addr.split(':').next().unwrap_or(&self.addr)
    }
}

/// Build the RFC 5322 / MIME message for `mail` (delegates to [`crate::mime`]).
pub fn build_message(mail: &Mail) -> String {
    crate::mime::to_mime(mail)
}

/// EHLO-advertised capabilities we care about.
struct Caps {
    starttls: bool,
}

#[async_trait::async_trait]
impl Deliverer for SmtpDeliverer {
    async fn deliver(&self, mail: &Mail) -> Result<()> {
        let tcp = TcpStream::connect(&self.addr).await.map_err(|e| {
            doido_core::anyhow::anyhow!("smtp connect to {} failed: {e}", self.addr)
        })?;
        let mut stream = BufReader::new(MaybeTls::Plain(tcp));

        expect(&mut stream, "220").await?;
        let caps = ehlo(&mut stream, &self.ehlo_name).await?;

        if self.starttls && caps.starttls {
            send(&mut stream, "STARTTLS").await?;
            expect(&mut stream, "220").await?;
            let upgraded = tls_upgrade(stream.into_inner(), self.host()).await?;
            stream = BufReader::new(upgraded);
            // Re-issue EHLO over the encrypted channel (capabilities may change).
            ehlo(&mut stream, &self.ehlo_name).await?;
        }

        let from = mail.from.as_deref().unwrap_or("no-reply@localhost");
        send(&mut stream, &format!("MAIL FROM:<{from}>")).await?;
        expect(&mut stream, "250").await?;
        // One RCPT TO per envelope recipient (to + cc + bcc).
        for rcpt in mail.recipients() {
            send(&mut stream, &format!("RCPT TO:<{rcpt}>")).await?;
            expect(&mut stream, "250").await?;
        }

        send(&mut stream, "DATA").await?;
        expect(&mut stream, "354").await?;
        stream.write_all(build_message(mail).as_bytes()).await?;
        stream.write_all(b"\r\n.\r\n").await?;
        stream.flush().await?;
        expect(&mut stream, "250").await?;

        send(&mut stream, "QUIT").await?;
        Ok(())
    }
}

/// Send `EHLO` and parse the multi-line 250 reply for capabilities.
async fn ehlo<S>(stream: &mut BufReader<S>, name: &str) -> Result<Caps>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    send(stream, &format!("EHLO {name}")).await?;
    let mut starttls = false;
    loop {
        let mut line = String::new();
        if stream.read_line(&mut line).await? == 0 {
            return Err(doido_core::anyhow::anyhow!("smtp connection closed early"));
        }
        let line = line.trim_end();
        if line.len() < 3 || &line[..3] != "250" {
            return Err(doido_core::anyhow::anyhow!(
                "smtp EHLO expected 250, got: {line}"
            ));
        }
        if line.to_ascii_uppercase().contains("STARTTLS") {
            starttls = true;
        }
        // "250 " (or a bare "250") ends the reply; "250-" is a continuation.
        if line.len() == 3 || line.as_bytes()[3] == b' ' {
            return Ok(Caps { starttls });
        }
    }
}

/// Wrap a plain connection in TLS (no-op if it is already a TLS stream).
async fn tls_upgrade(stream: MaybeTls, host: &str) -> Result<MaybeTls> {
    let tcp = match stream {
        MaybeTls::Plain(tcp) => tcp,
        tls => return Ok(tls),
    };
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| doido_core::anyhow::anyhow!("rustls config failed: {e}"))?
    .with_root_certificates(roots)
    .with_no_client_auth();
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|e| doido_core::anyhow::anyhow!("invalid TLS server name '{host}': {e}"))?;
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let tls = connector
        .connect(server_name, tcp)
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("STARTTLS handshake failed: {e}"))?;
    Ok(MaybeTls::Tls(Box::new(tls)))
}

async fn send<W: AsyncWriteExt + Unpin>(w: &mut W, cmd: &str) -> Result<()> {
    w.write_all(format!("{cmd}\r\n").as_bytes()).await?;
    w.flush().await?;
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

/// A connection that is either plain TCP or TLS-wrapped, so the same SMTP
/// conversation code works before and after a `STARTTLS` upgrade.
enum MaybeTls {
    Plain(TcpStream),
    Tls(Box<tokio_rustls::client::TlsStream<TcpStream>>),
}

impl AsyncRead for MaybeTls {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            MaybeTls::Plain(s) => Pin::new(s).poll_read(cx, buf),
            MaybeTls::Tls(s) => Pin::new(s.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for MaybeTls {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            MaybeTls::Plain(s) => Pin::new(s).poll_write(cx, buf),
            MaybeTls::Tls(s) => Pin::new(s.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            MaybeTls::Plain(s) => Pin::new(s).poll_flush(cx),
            MaybeTls::Tls(s) => Pin::new(s.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            MaybeTls::Plain(s) => Pin::new(s).poll_shutdown(cx),
            MaybeTls::Tls(s) => Pin::new(s.as_mut()).poll_shutdown(cx),
        }
    }
}
