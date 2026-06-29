//! The application context the [`WorkerEngine`](crate::WorkerEngine) carries and
//! hands to every job handler — the jobs analogue of the controller `Context`.

#[cfg(feature = "jobs-db")]
use doido_model::sea_orm::DatabaseConnection;

/// Shared application context passed to every job's handler.
///
/// Mirrors the controller `Context`: a job reaches the application's database
/// (and, as the framework grows, its config and shared state) through the
/// context the engine carries, rather than touching globals directly. The
/// worker process builds one at boot and hands it to
/// [`WorkerEngine::with_context`](crate::WorkerEngine::with_context).
#[derive(Clone, Default)]
pub struct JobContext {
    // Private so the set of fields can grow without breaking construction;
    // build via [`JobContext::new`].
    _private: (),
}

impl JobContext {
    /// Build the context handed to job handlers.
    pub fn new() -> Self {
        Self::default()
    }

    /// The application's database connection (global pool installed at boot).
    #[cfg(feature = "jobs-db")]
    pub fn db(&self) -> &'static DatabaseConnection {
        doido_model::pool::pool()
    }
}
