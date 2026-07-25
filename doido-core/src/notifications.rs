//! A lightweight instrumentation bus (Rails `ActiveSupport::Notifications`).
//!
//! Subscribe to an event-name prefix and `instrument` events to notify matching
//! subscribers. Synchronous and dependency-free (payloads are strings).

type Subscriber = Box<dyn Fn(&str, &str) + Send + Sync>;

/// A registry of event subscribers.
#[derive(Default)]
pub struct Notifications {
    subscribers: Vec<(String, Subscriber)>,
}

impl Notifications {
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe to events whose name starts with `prefix` (e.g. `"sql."`).
    pub fn subscribe(
        &mut self,
        prefix: &str,
        handler: impl Fn(&str, &str) + Send + Sync + 'static,
    ) {
        self.subscribers
            .push((prefix.to_string(), Box::new(handler)));
    }

    /// Fire `name` with `payload`, invoking every subscriber whose prefix matches.
    pub fn instrument(&self, name: &str, payload: &str) {
        for (prefix, handler) in &self.subscribers {
            if name.starts_with(prefix.as_str()) {
                handler(name, payload);
            }
        }
    }
}
