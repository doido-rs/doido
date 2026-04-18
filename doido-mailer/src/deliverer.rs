use crate::mail::Mail;
use doido_core::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait::async_trait]
pub trait Deliverer: Send + Sync {
    async fn deliver(&self, mail: &Mail) -> Result<()>;
}

pub struct LogDeliverer;

#[async_trait::async_trait]
impl Deliverer for LogDeliverer {
    async fn deliver(&self, mail: &Mail) -> Result<()> {
        tracing::info!(
            to = %mail.to,
            subject = %mail.subject,
            "delivering mail"
        );
        Ok(())
    }
}

#[derive(Clone)]
pub struct TestDeliverer {
    captured: Arc<Mutex<Vec<Mail>>>,
}

impl TestDeliverer {
    pub fn new() -> Self {
        Self {
            captured: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn sent(&self) -> Vec<Mail> {
        self.captured.lock().await.clone()
    }
}

impl Default for TestDeliverer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Deliverer for TestDeliverer {
    async fn deliver(&self, mail: &Mail) -> Result<()> {
        self.captured.lock().await.push(mail.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Deliverer, LogDeliverer, TestDeliverer};
    use crate::mail::Mail;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_log_deliverer_succeeds() {
        let d = LogDeliverer;
        let mail = Mail::new().to("a@b.com").subject("Hi").body_text("Hello");
        d.deliver(&mail).await.unwrap();
    }

    #[tokio::test]
    async fn test_test_deliverer_captures_mail() {
        let d = TestDeliverer::new();
        let mail = Mail::new().to("a@b.com").subject("Welcome").body_text("Hi");
        d.deliver(&mail).await.unwrap();
        let sent = d.sent().await;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "a@b.com");
        assert_eq!(sent[0].subject, "Welcome");
    }

    #[tokio::test]
    async fn test_deliverer_is_object_safe() {
        let d: Arc<dyn Deliverer> = Arc::new(LogDeliverer);
        let mail = Mail::new().to("a@b.com").subject("Test").body_text("body");
        d.deliver(&mail).await.unwrap();
    }
}
