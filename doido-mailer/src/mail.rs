use crate::deliverer::Deliverer;
use doido_core::Result;
use serde::{Deserialize, Serialize};

/// A file attached to a [`Mail`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
    /// `true` for inline (e.g. embedded image), `false` for a download.
    pub inline: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Mail {
    pub from: Option<String>,
    pub to: String,
    pub subject: String,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

impl Mail {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to = to.into();
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn body_html(mut self, html: impl Into<String>) -> Self {
        self.body_html = Some(html.into());
        self
    }

    pub fn body_text(mut self, text: impl Into<String>) -> Self {
        self.body_text = Some(text.into());
        self
    }

    /// Attach a file as a download.
    pub fn attach(
        mut self,
        filename: impl Into<String>,
        content_type: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        self.attachments.push(Attachment {
            filename: filename.into(),
            content_type: content_type.into(),
            data,
            inline: false,
        });
        self
    }

    /// Attach a file inline (e.g. an embedded image).
    pub fn attach_inline(
        mut self,
        filename: impl Into<String>,
        content_type: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        self.attachments.push(Attachment {
            filename: filename.into(),
            content_type: content_type.into(),
            data,
            inline: true,
        });
        self
    }

    pub async fn deliver_now(&self, deliverer: &dyn Deliverer) -> Result<()> {
        deliverer.deliver(self).await
    }

    /// Enqueue this mail onto the `mailers` queue for background delivery (Rails
    /// `deliver_later`). A worker with a mailer job handler delivers it later.
    pub async fn deliver_later(
        &self,
        queue: &dyn doido_jobs::JobQueue,
    ) -> Result<doido_jobs::JobId> {
        let payload = serde_json::to_value(self)?;
        let job = doido_jobs::JobPayload::new("mailers", payload, 3);
        queue.enqueue(job).await
    }
}
