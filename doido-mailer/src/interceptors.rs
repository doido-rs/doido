//! Delivery interceptors and observers (Rails `register_interceptor` /
//! `register_observer`). Interceptors may mutate a mail just before delivery
//! (e.g. redirect all mail to a sink in staging); observers see delivered mail.

use crate::deliverer::Deliverer;
use crate::mail::Mail;
use doido_core::Result;

/// Runs before delivery and may mutate the mail.
pub type Interceptor = fn(&mut Mail);
/// Runs after delivery with the delivered mail.
pub type Observer = fn(&Mail);

/// Wraps a [`Deliverer`] with interceptors (before) and observers (after).
pub struct InterceptingDeliverer<D: Deliverer> {
    inner: D,
    interceptors: Vec<Interceptor>,
    observers: Vec<Observer>,
}

impl<D: Deliverer> InterceptingDeliverer<D> {
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            interceptors: Vec::new(),
            observers: Vec::new(),
        }
    }

    pub fn intercept(mut self, interceptor: Interceptor) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    pub fn observe(mut self, observer: Observer) -> Self {
        self.observers.push(observer);
        self
    }
}

#[async_trait::async_trait]
impl<D: Deliverer> Deliverer for InterceptingDeliverer<D> {
    async fn deliver(&self, mail: &Mail) -> Result<()> {
        let mut prepared = mail.clone();
        for interceptor in &self.interceptors {
            interceptor(&mut prepared);
        }
        self.inner.deliver(&prepared).await?;
        for observer in &self.observers {
            observer(&prepared);
        }
        Ok(())
    }
}
