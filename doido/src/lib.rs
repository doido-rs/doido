pub mod boot;
pub mod cli;

mod banner;
mod config_cmd;
mod server;

pub use boot::{install_runtime_globals, install_test_runtime_globals, BootOptions};
pub use cli::{run, Doido};
pub use doido_core::{BootPolicy, InitPolicy, DEFAULT_LOCALES_DIR};
pub use doido_storage::{service as storage_service, StorageConfig, StorageConfigLoader};

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
