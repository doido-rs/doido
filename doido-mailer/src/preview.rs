//! Mailer previews (Rails `ActionMailer::Preview`): register example mails so a
//! dev endpoint can list and render them without sending.

use crate::mail::Mail;
use std::collections::BTreeMap;

/// A preview builder: returns an example [`Mail`].
pub type PreviewFn = fn() -> Mail;

/// A registry of named mail previews.
#[derive(Default)]
pub struct MailerPreviews {
    previews: BTreeMap<String, PreviewFn>,
}

impl MailerPreviews {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a preview under `name` (e.g. `"user_mailer/welcome"`).
    pub fn register(&mut self, name: &str, preview: PreviewFn) -> &mut Self {
        self.previews.insert(name.to_string(), preview);
        self
    }

    /// All registered preview names, sorted.
    pub fn names(&self) -> Vec<&str> {
        self.previews.keys().map(String::as_str).collect()
    }

    /// Render a preview to its MIME message, or `None` if unregistered.
    pub fn render(&self, name: &str) -> Option<String> {
        self.previews
            .get(name)
            .map(|build| crate::mime::to_mime(&build()))
    }
}
