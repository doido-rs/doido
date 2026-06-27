pub use doido_cache as cache;
// Routing (the former `doido-router`) now lives in `doido-controller`, so the
// `routes!` macro and the axum re-export are reachable via `doido::controller`.
pub use doido_controller as controller;
pub use doido_core as core;
pub use doido_generators as generators;
pub use doido_jobs as jobs;
pub use doido_mailer as mailer;
pub use doido_model as model;
pub use doido_view as view;

pub use doido_cable as cable;

// Flat re-exports for ergonomic top-level access
pub use doido_core::Result;
pub use doido_generators::{GeneratedFile, Generator};
pub use doido_mailer::{Deliverer, LogDeliverer};
// MiddlewareStack now lives in doido-controller (merged from doido-middleware).
pub use doido_controller::MiddlewareStack;
pub mod store {
    pub use doido_cache::store::CacheStore;
}
