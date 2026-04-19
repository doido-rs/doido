pub use doido_cache as cache;
pub use doido_config as config;
pub use doido_controller as controller;
pub use doido_core as core;
pub use doido_generators as generators;
pub use doido_jobs as jobs;
pub use doido_mailer as mailer;
pub use doido_middleware as middleware;
pub use doido_model as model;
pub use doido_router as router;
pub use doido_view as view;

// Crates with conflicting module names are exposed under their own namespaces
// to avoid ambiguous glob re-exports (cable/kafka/mcp all define `protocol`).
pub use doido_cable as cable;
pub use doido_kafka as kafka;
pub use doido_mcp as mcp;

// Flat re-exports for ergonomic top-level access
pub use doido_core::Result;
pub use doido_middleware::MiddlewareStack;
pub use doido_generators::{Generator, GeneratedFile};
pub use doido_mailer::{Deliverer, LogDeliverer};
pub mod store {
    pub use doido_cache::store::CacheStore;
}
