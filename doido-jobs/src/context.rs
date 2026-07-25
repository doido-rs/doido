//! The application context the [`WorkerEngine`](crate::WorkerEngine) carries and
//! hands to every job handler — the jobs analogue of the controller `Context`.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "jobs-db")]
use doido_model::sea_orm::DatabaseConnection;

/// Shared application context passed to every job's handler.
///
/// A job reaches the application's database (behind `jobs-db`) plus any shared
/// state the worker registered — configuration, clients, caches — via a typed
/// extension store, rather than touching globals directly. The worker process
/// builds one at boot and hands it to
/// [`WorkerEngine::with_context`](crate::WorkerEngine::with_context).
#[derive(Clone, Default)]
pub struct JobContext {
    extensions: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl JobContext {
    /// Build the context handed to job handlers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a shared value, retrievable by type with [`get`](Self::get).
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) -> &mut Self {
        self.extensions.insert(TypeId::of::<T>(), Arc::new(value));
        self
    }

    /// Retrieve a previously registered value by type.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|value| value.clone().downcast::<T>().ok())
    }

    /// The application's database connection (global pool installed at boot).
    #[cfg(feature = "jobs-db")]
    pub fn db(&self) -> &'static DatabaseConnection {
        doido_model::pool::pool()
    }
}
