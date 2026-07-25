//! Sendmail deliverer (Action Mailer `:sendmail`): pipe the MIME message to the
//! local `sendmail` binary's stdin.

use crate::deliverer::Deliverer;
use crate::mail::Mail;
use doido_core::Result;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Delivers by invoking a `sendmail`-compatible binary with `-t`.
pub struct SendmailDeliverer {
    binary: String,
}

impl SendmailDeliverer {
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
        }
    }
}

impl Default for SendmailDeliverer {
    fn default() -> Self {
        Self::new("/usr/sbin/sendmail")
    }
}

#[async_trait::async_trait]
impl Deliverer for SendmailDeliverer {
    async fn deliver(&self, mail: &Mail) -> Result<()> {
        let mut child = Command::new(&self.binary)
            .arg("-t")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| {
                doido_core::anyhow::anyhow!("sendmail ({}) spawn failed: {e}", self.binary)
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(crate::mime::to_mime(mail).as_bytes())
                .await?;
            // Drop stdin so the child sees EOF.
            drop(stdin);
        }

        let status = child.wait().await?;
        if !status.success() {
            return Err(doido_core::anyhow::anyhow!("sendmail exited with {status}"));
        }
        Ok(())
    }
}
