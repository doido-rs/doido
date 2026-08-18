pub mod cli;

mod banner;
mod server;

pub use cli::{run, Doido};

pub use doido_cache as cache;
pub use doido_controller as controller;
pub use doido_core as core;
pub use doido_generators as generators;
pub use doido_jobs as jobs;
pub use doido_mailer as mailer;
pub use doido_model as model;
pub use doido_storage as storage;
pub use doido_view as view;

pub use doido_cable as cable;

#[cfg(feature = "auth")]
pub use doido_auth as auth;

pub use doido_controller::MiddlewareStack;
pub use doido_core::Result;
pub use doido_generators::{GeneratedFile, Generator, DOIDO_VERSION};
pub use doido_mailer::{Deliverer, LogDeliverer};

pub mod store {
    pub use doido_cache::store::CacheStore;
}
