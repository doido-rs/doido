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
